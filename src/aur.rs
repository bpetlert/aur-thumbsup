use anyhow::{anyhow, Result};
use cookie::{Cookie, CookieJar, Expiration};
use lazy_static::lazy_static;
use reqwest::{
    blocking::{Client, Response},
    header, redirect, StatusCode, Url,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;
use tracing::debug;

lazy_static! {
    static ref AUR_URL: String = String::from("https://aur.archlinux.org");
    static ref AUR_URL_LOGIN: String = AUR_URL.to_string() + "/login?next=/";
    static ref AUR_URL_PKG_PAGE: String = AUR_URL.to_string() + "/packages/<PKG>";
    static ref AUR_URL_PKG_INFO: String = AUR_URL.to_string() + "/rpc?v=5&type=info";
    static ref AUR_URL_SORT_VOTED_PKG: String =
        AUR_URL.to_string() + "/packages/?O=<OFFSET>&SeB=nd&SB=w&SO=d&PP=250&do_Search=Go";
}

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_HOMEPAGE"),
    ")"
);

/// See: https://wiki.archlinux.org/index.php/Aurweb_RPC_interface#Limitations
const PACKAGE_QUERY_LIMIT: usize = 160;

/// For result table from https://aur.archlinux.org/packages/ page
#[derive(Default, Deserialize, PartialEq, Debug)]
pub struct AurPackageResultItem {
    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Version")]
    pub version: String,

    #[serde(rename = "Votes")]
    pub votes: u64,

    #[serde(rename = "Popularity?")]
    pub popularity: f64,

    #[serde(rename = "Voted", default, deserialize_with = "de_from_yes")]
    pub voted: bool,

    #[serde(rename = "Notify", default, deserialize_with = "de_from_yes")]
    pub notify: bool,

    #[serde(rename = "Description")]
    pub description: String,

    #[serde(rename = "Maintainer")]
    pub maintainer: String,
}

fn de_from_yes<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s == "Yes")
}

pub type AurPackageResults = Vec<AurPackageResultItem>;

pub trait Extraction<T> {
    fn from_html(html: &Html) -> Result<T>;
}

impl Extraction<AurPackageResults> for AurPackageResults {
    /// Extract package list from AUR packages page
    fn from_html(html: &Html) -> Result<AurPackageResults> {
        let mut aur_packages = AurPackageResults::new();

        let table_selector = match Selector::parse("div#pkglist-results table.results tbody tr") {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        let td_selector = match Selector::parse("td") {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        let table = html.select(&table_selector);
        for row in table {
            let cols: Vec<String> = row
                .select(&td_selector)
                .into_iter()
                .map(|td| td.inner_html().trim().to_owned())
                .collect();

            let name: String = match Html::parse_fragment(cols[1].as_str())
                .select(&Selector::parse("a").expect("Paring selector"))
                .next()
            {
                Some(n) => n.inner_html(),
                None => cols[1].to_owned(),
            };

            let version: String = cols[2].to_owned();
            let votes: u64 = cols[3].parse::<u64>()?;
            let popularity: f64 = cols[4].parse::<f64>()?;
            let voted: bool = cols[5] == "Yes";
            let notify: bool = cols[6] == "Yes";
            let description: String = cols[7].to_owned();

            let maintainer: String = match Html::parse_fragment(cols[8].as_str())
                .select(&Selector::parse("a").expect("Paring selector"))
                .next()
            {
                // Maintainer with link
                // <a href="/account/NAME" title="View account information for NAME">NAME</a>
                Some(m) => m.inner_html(),

                // Orphan
                // <span>orphan</span>
                None => match Html::parse_fragment(cols[8].as_str())
                    .select(&Selector::parse("span").expect("Paring selector"))
                    .next()
                {
                    Some(s) => s.inner_html(),
                    None => String::new(),
                },
            };

            aur_packages.push(AurPackageResultItem {
                name,
                version,
                votes,
                popularity,
                voted,
                notify,
                description,
                maintainer,
            });
        }

        Ok(aur_packages)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum VoteResult {
    Voted,
    AlreadyVoted,
    UnVoted,
    AlreadyUnVoted,
    NotAvailable,
    Failed,
}

#[derive(Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct Account {
    pub user: String,
    pub pass: String,
    pub cookie_file: PathBuf,
}

#[derive(Debug)]
pub struct Authentication {
    session: Option<Client>,
    cookie_jar: CookieJar,
}

impl Authentication {
    pub fn new() -> Self {
        Authentication {
            session: None,
            cookie_jar: CookieJar::new(),
        }
    }

    pub fn login(&mut self, account: &Account) -> Result<()> {
        if self.login_with_cookie_file(&account.cookie_file).is_err() {
            debug!("Failed to login using cookies.");

            self.login_with_user_pass(account)?;
            debug!("Logged in using user, pass.");

            self.save_cookie(&account.cookie_file)?;
            debug!(
                "Save cookie to `{}`",
                &account.cookie_file.to_str().expect("To str")
            );
        }

        debug!("Logged in using cookies.");
        Ok(())
    }

    pub fn is_login(&self) -> Result<()> {
        if self.session.is_some() {
            return Ok(());
        }
        Err(anyhow!("Not logged in."))
    }

    pub fn check_vote(&self, packages: &[String]) -> Result<Vec<(String, Option<bool>)>> {
        self.is_login()?;
        let session = self.session.as_ref().expect("as ref");

        let mut voted: Vec<(String, Option<bool>)> = Vec::new();
        for pkg in packages.iter() {
            let url = Url::parse(AUR_URL_PKG_PAGE.replace("<PKG>", pkg).as_str())?;
            let response = session.get(url).send()?;
            let page = Html::parse_document(response.text()?.as_str());
            let vote_status = self.is_vote_html(&page)?;
            voted.push((pkg.to_owned(), vote_status));
        }

        Ok(voted)
    }

    pub fn vote(&self, packages: &[String]) -> Result<Vec<(String, VoteResult)>> {
        self.is_login()?;
        let session = self.session.as_ref().expect("as ref");

        let mut result: Vec<(String, VoteResult)> = Vec::new();
        for pkg in packages.iter() {
            let url = Url::parse(AUR_URL_PKG_PAGE.replace("<PKG>", pkg).as_str())?;
            let response = session.get(url).send()?;
            let page = Html::parse_document(response.text()?.as_str());
            if let Some(status) = self.is_vote_html(&page)? {
                match status {
                    true => result.push((pkg.to_owned(), VoteResult::AlreadyVoted)),
                    false => {
                        if let Err(err) = self.do_vote(pkg, true, &page) {
                            debug!("{}", err);
                            result.push((pkg.to_owned(), VoteResult::Failed));
                            continue;
                        }

                        result.push((pkg.to_owned(), VoteResult::Voted));
                    }
                }
            } else {
                result.push((pkg.to_owned(), VoteResult::NotAvailable))
            }
        }

        Ok(result)
    }

    pub fn unvote(&self, packages: &[String]) -> Result<Vec<(String, VoteResult)>> {
        self.is_login()?;
        let session = self.session.as_ref().expect("as ref");

        let mut result: Vec<(String, VoteResult)> = Vec::new();
        for pkg in packages.iter() {
            let url = Url::parse(AUR_URL_PKG_PAGE.replace("<PKG>", pkg).as_str())?;
            let response = session.get(url).send()?;
            let page = Html::parse_document(response.text()?.as_str());
            if let Some(status) = self.is_vote_html(&page)? {
                match status {
                    true => {
                        if let Err(err) = self.do_vote(pkg, false, &page) {
                            debug!("{}", err);
                            result.push((pkg.to_owned(), VoteResult::Failed));
                            continue;
                        }

                        result.push((pkg.to_owned(), VoteResult::UnVoted));
                    }
                    false => result.push((pkg.to_owned(), VoteResult::AlreadyUnVoted)),
                }
            } else {
                result.push((pkg.to_owned(), VoteResult::NotAvailable))
            }
        }

        Ok(result)
    }

    pub fn list_voted_pkgs(&self) -> Result<AurPackageResults> {
        self.is_login()?;
        let session = self.session.as_ref().expect("as ref");

        let mut voted_pkgs = AurPackageResults::new();
        let mut offset: i32 = -250;
        loop {
            offset += 250;
            let url = Url::parse(
                AUR_URL_SORT_VOTED_PKG
                    .replace("<OFFSET>", offset.to_string().as_str())
                    .as_str(),
            )?;
            let response = session.get(url).send()?;
            let page = Html::parse_document(response.text()?.as_str());
            let packages = AurPackageResults::from_html(&page)?;

            if packages.is_empty() {
                return Ok(voted_pkgs);
            }

            for pkg in packages {
                if !pkg.voted {
                    return Ok(voted_pkgs);
                }
                voted_pkgs.push(pkg);
            }
        }
    }

    pub(self) fn login_with_user_pass(&mut self, account: &Account) -> Result<()> {
        debug!("Attempt to login using user and password.");

        let login_url = Url::parse_with_params(
            &AUR_URL_LOGIN,
            &[
                ("user", account.user.as_str()),
                ("passwd", account.pass.as_str()),
                ("remember_me", "on"),
            ],
        )?;
        debug!("Login URL: {login_url}");

        // Stop redirect to https://aur.archlinux.org/ after logged in
        let login_no_redirect = redirect::Policy::custom(|attempt| {
            if attempt.status() == StatusCode::FOUND
                && attempt.url().to_string() == (AUR_URL.to_string() + "/")
            {
                return attempt.stop();
            }
            redirect::Policy::default().redirect(attempt)
        });
        let login_client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .cookie_store(true)
            .redirect(login_no_redirect)
            .gzip(true)
            .http2_prior_knowledge()
            .use_rustls_tls()
            .build()?;
        let login_response = login_client.get(login_url).send()?;
        debug!("Login response: {login_response:?}");

        // Login success
        if login_response.status() == StatusCode::FOUND
            && login_response
                .url()
                .to_string()
                .contains(&AUR_URL.to_string())
        {
            // Get AURSID for login cookie
            if let Some(aursid) = login_response.headers().get(header::SET_COOKIE) {
                let cookie_str = aursid.to_str()?.to_owned();
                let mut c = Cookie::parse(cookie_str)?;
                c.set_domain("aur.archlinux.org");
                self.cookie_jar.add(c);

                // Access https://aur.archlinux.org/ with AURSID to get another cookies
                let (response, _) = self.login_with_cookies()?;

                // Get AURTZ, AURLANG cookie
                let aur_cookies = response.headers().get_all(header::SET_COOKIE);
                for c in aur_cookies.iter() {
                    let cookie_str = c.to_str()?.to_owned();
                    let mut cookie = Cookie::parse(cookie_str)?;
                    cookie.set_domain("aur.archlinux.org");
                    self.cookie_jar.add(cookie);
                }

                // Re-login using cookies
                let (response, session) = self.login_with_cookies()?;
                let logged_page = Html::parse_document(response.text()?.as_str());
                self.is_login_html(&logged_page)?;
                self.session = Some(session);

                return Ok(());
            }

            return Err(anyhow!("Login failed: no cookie found."));
        }

        self.session = None;

        if !login_response.status().is_success() {
            return Err(anyhow!("Unable to access `{}`", &AUR_URL_LOGIN.to_string()));
        }

        // Login failed, get error messages
        let page = Html::parse_document(login_response.text()?.as_str());
        let error_list = LoginErrorList::from_html(&page)?;
        if !error_list.errors.is_empty() {
            return Err(anyhow!("Login failed: {}", error_list.errors.join(", ")));
        }

        Err(anyhow!("Login failed"))
    }

    pub(self) fn login_with_cookie_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        debug!("Attemp to login using cookies.");

        // Load cookies from file
        let cookie_file = File::open(path)?;
        let reader = BufReader::new(cookie_file);
        for line in reader.lines() {
            let c = Cookie::parse(line?)?;
            self.cookie_jar.add(c);
        }

        let (response, session) = self.login_with_cookies()?;
        let logged_page = Html::parse_document(response.text()?.as_str());
        self.is_login_html(&logged_page)?;
        self.session = Some(session);
        Ok(())
    }

    pub(self) fn login_with_cookies(&mut self) -> Result<(Response, Client)> {
        // Add cookies to headers, ordering is matter
        let mut headers = header::HeaderMap::new();
        // AURTZ
        if let Some(aurtz) = self.cookie_jar.get("AURTZ") {
            if let Some(expire_time) = aurtz.expires() {
                match expire_time {
                    Expiration::DateTime(d) => {
                        if d.unix_timestamp() < OffsetDateTime::now_utc().unix_timestamp() {
                            debug!("Cookies were expired.");
                            return Err(anyhow!("Cookies were expired."));
                        }
                    }
                    Expiration::Session => (),
                }
            }

            let code = aurtz.encoded().to_string();
            headers.insert(header::COOKIE, code.parse()?);
        }
        // AURLANG
        if let Some(aurlang) = self.cookie_jar.get("AURLANG") {
            let code = aurlang.encoded().to_string();
            headers.append(header::COOKIE, code.parse()?);
        }
        // AURSID
        if let Some(aursid) = self.cookie_jar.get("AURSID") {
            let code = aursid.encoded().to_string();
            headers.append(header::COOKIE, code.parse()?);
        }

        let session = Client::builder()
            .user_agent(APP_USER_AGENT)
            .default_headers(headers)
            .cookie_store(true)
            .gzip(true)
            .http2_prior_knowledge()
            .use_rustls_tls()
            .build()?;
        let aur_url = Url::parse(&AUR_URL)?;
        let response = session.get(aur_url).send()?;

        if response.status().is_success() {
            return Ok((response, session));
        }

        Err(anyhow!(
            "Unable to access `{}` with AURSID cookie",
            &AUR_URL.to_string()
        ))
    }

    pub(self) fn save_cookie<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.is_login()?;

        let mut cookie_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(path)?;

        // AURTZ
        if let Some(aurtz) = self.cookie_jar.get("AURTZ") {
            writeln!(cookie_file, "{}", aurtz.encoded())?;
        }
        // AURLANG
        if let Some(aurlang) = self.cookie_jar.get("AURLANG") {
            writeln!(cookie_file, "{}", aurlang.encoded())?;
        }
        // AURSID
        if let Some(aursid) = self.cookie_jar.get("AURSID") {
            writeln!(cookie_file, "{}", aursid.encoded())?;
        }

        Ok(())
    }

    /// Extract vote status from html
    pub(self) fn is_vote_html(&self, html: &Html) -> Result<Option<bool>> {
        // Voted
        let do_unvote_selector = match Selector::parse(
            "div#actionlist li form[action$=\"vote/\"] input[name=\"do_UnVote\"]",
        ) {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        if html.select(&do_unvote_selector).next().is_some() {
            return Ok(Some(true));
        }

        // Unvoted
        let do_vote_selector = match Selector::parse(
            "div#actionlist li form[action$=\"vote/\"] input[name=\"do_Vote\"]",
        ) {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        if html.select(&do_vote_selector).next().is_some() {
            return Ok(Some(false));
        }

        Ok(None)
    }

    pub(self) fn extract_token(&self, html: &Html) -> Result<String> {
        let token_selector = match Selector::parse(
            "div#actionlist li form[action$=\"vote/\"] input[name=\"token\"]",
        ) {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        if let Some(token) = html.select(&token_selector).next() {
            return Ok(token.value().attr("value").unwrap_or_default().to_owned());
        }

        Ok(String::new())
    }

    pub(self) fn do_vote(&self, pkg: &str, vote: bool, page: &Html) -> Result<()> {
        let session = self.session.as_ref().expect("as ref");
        // Get token
        let token = self.extract_token(page)?;

        // Get pkgbase for pkg
        let pkgbase_selector = match Selector::parse("table#pkginfo tr td a[href*=\"/pkgbase/\"]") {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("Error: create selector: {:?}", err)),
        };

        let pkgbase: String = match page.select(&pkgbase_selector).next() {
            Some(element) => match element.value().attr("href") {
                Some(link) => link.to_owned(),
                None => return Err(anyhow!("Error: cannot get pkgbase of {}", pkg)),
            },
            None => return Err(anyhow!("Error: cannot get pkgbase of {}", pkg)),
        };

        let url = Url::parse(
            &(AUR_URL.to_string()
                + &pkgbase
                + match vote {
                    true => "vote/",
                    false => "unvote/",
                }),
        )?;

        let mut params = HashMap::new();
        params.insert("token", token);
        params.insert(
            match vote {
                true => "do_Vote",
                false => "do_UnVote",
            },
            pkg.to_owned(),
        );
        debug!("Un(Vote) URL: {}", url);

        let response = session.post(url).form(&params).send()?;

        if !response.status().is_success() {
            if vote {
                return Err(anyhow!("Error: cannot vote for {}", pkg));
            } else {
                return Err(anyhow!("Error: cannot unvote {}", pkg));
            }
        }

        Ok(())
    }

    /// Check if user logged in using html from https://aur.archlinux.org/
    pub(self) fn is_login_html(&self, html: &Html) -> Result<()> {
        let logout_selector = match Selector::parse("div#archdev-navbar li a[href=\"/logout/\"]") {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };
        match html.select(&logout_selector).next() {
            Some(_) => Ok(()),
            None => Err(anyhow!("Not logged in.")),
        }
    }
}

#[derive(Default, Deserialize, PartialEq, Debug)]
struct LoginErrorList {
    pub errors: Vec<String>,
}

impl Extraction<LoginErrorList> for LoginErrorList {
    /// Extract error list from AUR login page
    fn from_html(html: &Html) -> Result<LoginErrorList> {
        let mut error_list = LoginErrorList::default();

        let errlist_selector = match Selector::parse("ul.errorlist li") {
            Ok(selector) => selector,
            Err(err) => return Err(anyhow!("{:?}", err)),
        };

        let errlist = html.select(&errlist_selector);
        error_list.errors = errlist
            .into_iter()
            .map(|li| li.inner_html().trim().to_owned())
            .collect();

        Ok(error_list)
    }
}

/// For data from https://aur.archlinux.org/rpc?v=5&type=info&arg[]=pkg1&arg[]=pkg2&…
/// See: https://wiki.archlinux.org/index.php/Aurweb_RPC_interface#info_2
#[derive(Deserialize)]
struct AurPackageInfoResult {
    #[serde(rename(deserialize = "results"))]
    results: AurPackageInfo,
}

/// For data from https://aur.archlinux.org/rpc?v=5&type=info&arg[]=pkg1&arg[]=pkg2&…
/// See: https://wiki.archlinux.org/index.php/Aurweb_RPC_interface#info_2
#[derive(Deserialize, Default, Debug)]
pub struct AurPackageInfoItem {
    #[serde(rename(deserialize = "Name"))]
    pub name: String,

    #[serde(rename(deserialize = "Version"))]
    pub version: String,
}

pub type AurPackageInfo = Vec<AurPackageInfoItem>;

pub trait AurInfoQuery<T> {
    fn info_query(pkgs: &[String]) -> Result<T>;
}

impl AurInfoQuery<AurPackageInfo> for AurPackageInfo {
    fn info_query(pkgs: &[std::string::String]) -> Result<AurPackageInfo> {
        let client = Client::builder()
            .user_agent(APP_USER_AGENT)
            .gzip(true)
            .http2_prior_knowledge()
            .use_rustls_tls()
            .build()?;

        let mut results: AurPackageInfo = Vec::new();
        for chunk in pkgs.chunks(PACKAGE_QUERY_LIMIT) {
            let queries: Vec<(&str, &str)> =
                chunk.iter().map(|pkg| ("arg[]", pkg.as_str())).collect();
            let url = Url::parse_with_params(&AUR_URL_PKG_INFO, &queries)?;
            let response = client.get(url).send()?;
            let mut info_results: AurPackageInfoResult = response.json()?;
            results.append(&mut info_results.results);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_aur_pkgs_no_sort_voted() {
        // Extract package list from html
        let html_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-user-no-sort-voted-packages.html"
        ));
        let page = Html::parse_document(html_raw);
        let aur_packages = AurPackageResults::from_html(&page).expect("Paring AUR package results");
        assert_eq!(aur_packages.len(), 50);

        // Compare with the same data in CSV format
        let pkglist_csv = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-user-no-sort-voted-packages.csv"
        ));
        let mut pkglist = csv::Reader::from_reader(pkglist_csv.as_bytes());
        let pkgs: AurPackageResults = pkglist
            .deserialize()
            .map(|result| result.expect("AurPackageResultItem"))
            .collect();
        for n in 0..pkgs.len() {
            assert_eq!(aur_packages[n], pkgs[n], "Failed at record: {}", n);
        }

        // Check voted pkgs
        assert_eq!(aur_packages.into_iter().filter(|pkg| pkg.voted).count(), 12);
    }

    #[test]
    fn test_extract_aur_pkgs_sort_voted_with_orphan() {
        // Extract package list from html
        let html_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-aur-pkgs-sort-voted-with-orphan.html"
        ));
        let page = Html::parse_document(html_raw);
        let aur_packages = AurPackageResults::from_html(&page).expect("Paring AUR package results");
        assert_eq!(aur_packages.len(), 250);

        // Check orphan packages
        assert_eq!(
            aur_packages
                .into_iter()
                .filter(|pkg| pkg.maintainer == "orphan")
                .count(),
            12
        );
    }

    #[test]
    fn test_extract_login_error_page() {
        // Login success
        let html_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-user-no-sort-voted-packages.html"
        ));
        let page = Html::parse_document(html_raw);
        let error_list = LoginErrorList::from_html(&page).expect("Paring login error");
        assert_eq!(error_list.errors.len(), 0);

        // Login failed
        let html_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-login-error.html"
        ));
        let page = Html::parse_document(html_raw);
        let error_list = LoginErrorList::from_html(&page).expect("Paring login error");
        assert_eq!(error_list.errors.len(), 1);
        assert_eq!(error_list.errors[0], "Bad username or password.");
    }

    #[test]
    fn test_check_login_page() {
        let html_raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-in-page.html"
        ));
        let page = Html::parse_document(html_raw);
        let auth = Authentication::new();
        assert!(auth.is_login_html(&page).is_ok());
    }

    #[test]
    fn test_is_vote_html() {
        // Voted package
        let voted_pkg_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-pkg-info-voted.html"
        ));
        let page = Html::parse_document(voted_pkg_page);
        let auth = Authentication::new();
        assert_eq!(auth.is_vote_html(&page).unwrap(), Some(true));

        // Unvoted package
        let unvoted_pkg_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-pkg-info-unvoted.html"
        ));
        let page = Html::parse_document(unvoted_pkg_page);
        let auth = Authentication::new();
        assert_eq!(auth.is_vote_html(&page).unwrap(), Some(false));

        // N/A
        let not_pkg_info_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-in-page.html"
        ));
        let page = Html::parse_document(not_pkg_info_page);
        let auth = Authentication::new();
        assert_eq!(auth.is_vote_html(&page).unwrap(), None);
    }

    #[test]
    fn test_extract_token() {
        // From voted package
        let voted_pkg_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-pkg-info-voted.html"
        ));
        let page = Html::parse_document(voted_pkg_page);
        let auth = Authentication::new();
        let token = auth.extract_token(&page).unwrap();
        let expect = "FAKETOKENFAKETOKENFAKETOKENFAKET".to_owned();
        assert_eq!(token, expect, "`{}` != `{}`", token, expect);

        // From unvoted package
        let unvoted_pkg_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-logged-pkg-info-unvoted.html"
        ));
        let page = Html::parse_document(unvoted_pkg_page);
        let auth = Authentication::new();
        let token = auth.extract_token(&page).unwrap();
        let expect = "FAKETOKENFAKETOKENFAKETOKENFAKET".to_owned();
        assert_eq!(token, expect, "`{}` != `{}`", token, expect);

        // N/A
        let na_pkg_page = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/",
            "test-login-error.html"
        ));
        let page = Html::parse_document(na_pkg_page);
        let auth = Authentication::new();
        let token = auth.extract_token(&page).unwrap();
        let expect = "".to_owned();
        assert_eq!(token, expect, "`{}` != `{}`", token, expect);
    }

    #[test]
    fn test_aur_info_query() {
        let pkgs = vec!["pacman-mirrorup".to_owned(), "networkd-broker".to_owned()];
        let aur_pkg_info: AurPackageInfo = AurPackageInfo::info_query(&pkgs).unwrap();
        assert_eq!(aur_pkg_info.len(), 2);
        assert_eq!(aur_pkg_info[0].name, "networkd-broker");
        assert_eq!(aur_pkg_info[1].name, "pacman-mirrorup");
    }
}
