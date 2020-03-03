use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::aur::{AurPackage, Authentication};
use crate::config::Configuration;
use crate::helper::{list_installed_pkgs, vercmp, PkgName, PkgVersion, Versioning};

pub fn list<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let voted_pkgs = auth.list_voted_pkgs()?;
    let installed_pkgs: HashMap<PkgName, PkgVersion> = list_installed_pkgs()?;
    for pkg in &voted_pkgs {
        println!("{}", fancy(pkg, &installed_pkgs)?);
    }

    Ok(())
}

fn fancy(aur_pkg: &AurPackage, installed_pkgs: &HashMap<PkgName, PkgVersion>) -> Result<String> {
    let mut status: Vec<String> = Vec::new();

    // Install?
    if let Some(local_ver) = installed_pkgs.get(&aur_pkg.name) {
        let result: String = match vercmp(&local_ver, &aur_pkg.version)? {
            Versioning::Older => format!("{}, {}", local_ver.bright_red(), "Outdated".bright_red()),
            Versioning::Same => format!("{}", local_ver.bright_green()),
            Versioning::Newer => {
                format!("{}, {}", local_ver.bright_yellow(), "Newer".bright_yellow())
            }
        };
        status.push(format!("{} {}", "Installed:".cyan(), result));
    }

    // Orphan?
    if aur_pkg.maintainer == "orphan" {
        status.push(format!("{}", "Orphaned".bright_red()));
    }

    Ok(format!(
        "{} {}{}",
        aur_pkg.name.bold().white(),
        aur_pkg.version.bold().bright_green(),
        match status.is_empty() {
            true => "".to_owned(),
            false => format!(" [{}]", status.join(", ")),
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fancy() {
        let mut aur_pkg = AurPackage {
            name: "pacman-mirrorup".to_owned(),
            version: "0.3.0-1".to_owned(),
            votes: 1,
            popularity: 0.99,
            voted: true,
            notify: true,
            description: "A service to retrieve the best and latest Pacman mirror list based on user's geography".to_owned(),
            maintainer: "bpetlert".to_owned()
        };
        let mut installed_pkgs: HashMap<PkgName, PkgVersion> = HashMap::new();
        installed_pkgs.insert("pacman-mirrorup".to_owned(), "0.3.0-1".to_owned());

        // Same version
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {} [{} {}]",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
            "Installed:".cyan(),
            installed_pkgs[&aur_pkg.name].bright_green()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // AUR is newer
        aur_pkg.version = "0.3.0.r5.ge7b1840-1".to_owned();
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {} [{} {}, {}]",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
            "Installed:".cyan(),
            installed_pkgs[&aur_pkg.name].bright_red(),
            "Outdated".bright_red()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // local is newer
        aur_pkg.version = "0.3.0-1".to_owned();
        *installed_pkgs.get_mut(&aur_pkg.name).unwrap() = "0.3.0.r5.ge7b1840-1".to_owned();
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {} [{} {}, {}]",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
            "Installed:".cyan(),
            installed_pkgs[&aur_pkg.name].bright_yellow(),
            "Newer".bright_yellow()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Same version but orphan
        aur_pkg.version = "0.3.0-1".to_owned();
        aur_pkg.maintainer = "orphan".to_owned();
        *installed_pkgs.get_mut(&aur_pkg.name).unwrap() = "0.3.0-1".to_owned();
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {} [{} {}, {}]",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
            "Installed:".cyan(),
            installed_pkgs[&aur_pkg.name].bright_green(),
            "Orphaned".bright_red()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Not install and orphan
        aur_pkg.maintainer = "orphan".to_owned();
        installed_pkgs.remove(&aur_pkg.name);
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {} [{}]",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
            "Orphaned".bright_red()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Not install and not orphan
        aur_pkg.maintainer = "bpetlert".to_owned();
        installed_pkgs.remove(&aur_pkg.name);
        let result = fancy(&aur_pkg, &installed_pkgs).unwrap();
        let expect = format!(
            "{} {}",
            aur_pkg.name.bold().white(),
            aur_pkg.version.bold().bright_green(),
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);
    }
}
