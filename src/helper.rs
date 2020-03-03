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

pub fn is_file_secure<P: AsRef<Path>>(path: P) -> Result<bool> {
    let permissions = File::open(path)?.metadata()?.permissions();
    Ok(permissions.mode() & 0o666 == 0o600)
}

pub fn list_installed_pkgs() -> Result<HashMap<PkgName, PkgVersion>> {
    let packman_child = Command::new("/usr/bin/pacman")
        .arg("-Q")
        .stdout(Stdio::piped())
        .spawn()?;

    let pacman_output = packman_child.wait_with_output()?;
    let lines = String::from_utf8(pacman_output.stdout)?;
    let pkglist: HashMap<PkgName, PkgVersion> = lines
        .split("\n")
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.split(" ").collect();
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
}
