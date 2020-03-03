use crate::aur::Account;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::helper::is_file_secure;

#[derive(Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct Configuration {
    pub account: Account,
}

impl Configuration {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Configuration> {
        let config_content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(err) => return Err(anyhow!("{} `{}`", err, &path.as_ref().to_str().unwrap())),
        };

        let config: Configuration = toml::from_str(config_content.as_str())?;
        Ok(config)
    }

    pub fn load_and_verify_config<P: AsRef<Path>>(path: P) -> Result<Configuration> {
        if !is_file_secure(&path)? {
            return Err(anyhow!(
                "`{}` file is not secure.",
                &path.as_ref().to_str().unwrap()
            ));
        }

        let config = Configuration::from_file(&path)?;

        if config.account.user.is_empty() {
            return Err(anyhow!("User name is required."));
        }

        if config.account.pass.is_empty() {
            return Err(anyhow!("Password is required."));
        }

        if config.account.cookie_file.as_os_str().is_empty() {
            return Err(anyhow!("Cookie file path is required."));
        }

        Ok(config)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if path.as_ref().exists() {
            return Err(anyhow!("`{}` is exist.", path.as_ref().to_str().unwrap()));
        }

        let toml = toml::to_string(&self)?;

        let mut config_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        config_file.write_all(toml.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_configuration() {
        const CONFIG_FILE: &'static str = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src",
            "/test-aur-thumbsup.toml"
        );
        let config = Configuration::from_file(CONFIG_FILE).unwrap();

        assert_eq!(
            Configuration {
                account: Account {
                    user: "foo".to_owned(),
                    pass: "bar".to_owned(),
                    cookie_file: PathBuf::from(r"/var/tmp/aur-thumbsup-foo.cookie")
                }
            },
            config
        );
    }

    #[test]
    fn test_configuration_to_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("aur-thumbsup-foo.toml");
        let config = Configuration {
            account: Account {
                user: "foo".to_owned(),
                pass: "bar".to_owned(),
                cookie_file: PathBuf::from(r"/var/tmp/aur-thumbsup-foo.cookie"),
            },
        };
        let result = config.to_file(file_path);
        assert!(result.is_ok());
        tempdir.close().unwrap();
    }
}
