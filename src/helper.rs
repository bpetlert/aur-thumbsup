use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

pub type PkgName = String;
pub type PkgVersion = String;

#[derive(PartialEq, Eq, Debug)]
pub enum Versioning {
    Older,
    Same,
    Newer,
}

#[derive(PartialEq, Eq, Debug)]
pub enum SelectRepository {
    #[allow(dead_code)]
    Official,

    NonOfficial,

    #[allow(dead_code)]
    All,
}

/// Check if file has read-write only for user
pub fn is_file_secure<P: AsRef<Path>>(path: P) -> Result<bool> {
    let permissions = File::open(path)?.metadata()?.permissions();
    Ok(permissions.mode() & 0o666 == 0o600)
}

/// List all installed packages on system
pub fn list_installed_pkgs() -> Result<HashMap<PkgName, PkgVersion>> {
    let packman_child = Command::new("/usr/bin/pacman")
        .arg("-Q")
        .stdout(Stdio::piped())
        .spawn()?;

    let pacman_output = packman_child.wait_with_output()?;
    let lines = String::from_utf8(pacman_output.stdout)?;
    let pkglist: HashMap<PkgName, PkgVersion> = lines
        .split('\n')
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.split(' ').collect();
            (cols[0].to_owned(), cols[1].to_owned())
        })
        .collect();
    Ok(pkglist)
}

/// Compare version using `/usr/bin/vercmp` from pacman
pub fn vercmp<L, R>(left: L, right: R) -> Result<Versioning>
where
    L: AsRef<OsStr>,
    R: AsRef<OsStr>,
{
    let output = Command::new("/usr/bin/vercmp")
        .arg(&left)
        .arg(&right)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("Error calling `vercmp`"));
    }

    let output = String::from_utf8(output.stdout)?;
    let result = output.trim().parse::<i32>()?;
    match result.cmp(&0) {
        Ordering::Less => Ok(Versioning::Older),
        Ordering::Equal => Ok(Versioning::Same),
        Ordering::Greater => Ok(Versioning::Newer),
    }
}

/// List available repositories on system
pub fn list_repos(select: SelectRepository) -> Result<Vec<String>> {
    let output = Command::new("/usr/bin/pacman-conf")
        .arg("--repo-list")
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("Error calling `pacman-conf`"));
    }

    let lines = String::from_utf8(output.stdout)?;
    let repolist: Vec<String> = lines
        .split('\n')
        .into_iter()
        .filter(|repo| !repo.is_empty())
        .filter(|repo| match select {
            SelectRepository::Official => {
                repo == &"core" || repo == &"extra" || repo == &"community" || repo == &"multilib"
            }
            SelectRepository::NonOfficial => {
                !(repo == &"core"
                    || repo == &"extra"
                    || repo == &"community"
                    || repo == &"multilib")
            }
            SelectRepository::All => true,
        })
        .map(|repo| repo.to_owned())
        .collect();
    Ok(repolist)
}

/// List installed packages from a repository
pub fn list_installed_pkgs_repo<S: AsRef<str>>(repo: S) -> Result<HashMap<PkgName, PkgVersion>> {
    let mut packman_child = Command::new("/usr/bin/pacman")
        .arg("-Sl")
        .arg(repo.as_ref())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(pacman_output) = packman_child.stdout.take() {
        let mut grep_child = Command::new("/usr/bin/grep")
            .arg("\\[installed\\]$")
            .stdin(pacman_output)
            .stdout(Stdio::piped())
            .spawn()?;
        if let Some(grep_output) = grep_child.stdout.take() {
            let awk_child = Command::new("/usr/bin/awk")
                .arg("{ print $2, $3 }")
                .stdin(grep_output)
                .stdout(Stdio::piped())
                .spawn()?;
            packman_child.wait()?;
            grep_child.wait()?;
            let awk_output = awk_child.wait_with_output()?;
            let lines = String::from_utf8(awk_output.stdout)?;
            let pkglist: Vec<&str> = lines.split('\n').collect();
            let pkgs: HashMap<PkgName, PkgVersion> = pkglist
                .iter()
                .filter(|pkg| !pkg.is_empty())
                .map(|pkg| {
                    let pkg_info: Vec<&str> = pkg.split(' ').collect();
                    (pkg_info[0].to_owned(), pkg_info[1].to_owned())
                })
                .collect();
            return Ok(pkgs);
        }
    }
    Err(anyhow!(
        "Failed list installed package from {}",
        repo.as_ref()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_secure() {
        let f1 = tempfile::Builder::new()
            .prefix("aur-thumbsup-foo-")
            .suffix(".toml")
            .tempfile()
            .unwrap();
        let filename = f1.path();
        let f2 = f1.reopen().unwrap();
        let mut permissions = f2.metadata().unwrap().permissions();
        permissions.set_mode(0o600);
        assert_eq!(permissions.mode(), 0o600);

        let is_secure = is_file_secure(filename).unwrap();
        assert!(is_secure);
    }

    #[test]
    fn test_version_compare() {
        assert_eq!(
            vercmp("0.3.0-1", "0.3.0.r5.ge7b1840-1").unwrap(),
            Versioning::Older
        );

        assert_eq!(vercmp("0.3.0-1", "0.3.0-1").unwrap(), Versioning::Same);

        assert_eq!(
            vercmp("0.3.0.r5.ge7b1840-1", "0.3.0-1").unwrap(),
            Versioning::Newer
        );
    }

    #[test]
    fn test_list_repos() {
        // Official repositories
        let repolist = list_repos(SelectRepository::Official).unwrap();
        assert_eq!(repolist.len(), 4);
        assert!(repolist.contains(&"core".to_owned()));
        assert!(repolist.contains(&"extra".to_owned()));
        assert!(repolist.contains(&"community".to_owned()));
        assert!(repolist.contains(&"multilib".to_owned()));

        // Non-official repositories
        let repolist = list_repos(SelectRepository::NonOfficial).unwrap();
        assert!(!repolist.contains(&"core".to_owned()));
        assert!(!repolist.contains(&"extra".to_owned()));
        assert!(!repolist.contains(&"community".to_owned()));
        assert!(!repolist.contains(&"multilib".to_owned()));

        // All repositories
        let repolist = list_repos(SelectRepository::All).unwrap();
        assert!(repolist.len() >= 4);
        assert!(repolist.contains(&"core".to_owned()));
        assert!(repolist.contains(&"extra".to_owned()));
        assert!(repolist.contains(&"community".to_owned()));
        assert!(repolist.contains(&"multilib".to_owned()));
    }

    #[test]
    fn test_list_installed_pkgs_repo() {
        let pkgs = list_installed_pkgs_repo("core").unwrap();
        assert!(pkgs.contains_key("pacman"));
        assert!(pkgs.contains_key("systemd"));
        assert!(pkgs.contains_key("systemd-libs"));
    }
}
