use anyhow::Result;
use std::{collections::HashMap, fmt::Write, path::Path};

use crate::{
    aur::{AurInfoQuery, AurPackageInfo, Authentication},
    cmds::{unvote, vote},
    config::Configuration,
    helper::{list_installed_pkgs_repo, list_repos, PkgName, PkgVersion, SelectRepository},
};

pub fn autovote<P: AsRef<Path>>(config_path: P) -> Result<()> {
    // [1] Get non-official repositories
    let non_official = list_repos(SelectRepository::NonOfficial)?;

    // [2] Get installed packages from all non-official repositories.
    let mut installed_pkgs: HashMap<PkgName, PkgVersion> = HashMap::new();
    for repo in non_official.iter() {
        let pkgs_in_repo = list_installed_pkgs_repo(repo)?;
        for pkg in pkgs_in_repo.iter() {
            if !installed_pkgs.contains_key(pkg.0) {
                installed_pkgs.insert(pkg.0.to_owned(), pkg.1.to_owned());
            }
        }
    }

    // [3] Get voted packages
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let mut voted_pkgs = auth.list_voted_pkgs()?;

    // [4] Remove voted packages from installed_pkgs and also remove already voted packages from voted_pkgs
    voted_pkgs.retain(|pkg| {
        if installed_pkgs.contains_key(&pkg.name) {
            installed_pkgs.remove(&pkg.name);

            // also remove from voted_pkgs
            false
        } else {
            // keep it
            true
        }
    });

    // [5] Verify if installed packages are AUR package.
    let pkgs: Vec<PkgName> = installed_pkgs.iter().map(|pkg| pkg.0.to_owned()).collect();
    let verified_pkgs = AurPackageInfo::info_query(&pkgs)?;

    // [6] Vote verified packages
    let pkgs: Vec<PkgName> = verified_pkgs
        .iter()
        .map(|pkg| pkg.name.to_owned())
        .collect();
    let results = auth.vote(&pkgs)?;

    let mut output = String::new();
    for result in results.iter() {
        writeln!(output, "{}", vote::fancy(result)?)?;
    }
    print!("{}", output);

    // [7] Unvote the left packages in voted_pkgs
    let pkgs: Vec<PkgName> = voted_pkgs.iter().map(|pkg| pkg.name.to_owned()).collect();
    let results = auth.unvote(&pkgs)?;

    let mut output = String::new();
    for result in results.iter() {
        writeln!(output, "{}", unvote::fancy(result)?)?;
    }
    print!("{}", output);

    Ok(())
}
