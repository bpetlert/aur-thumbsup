use anyhow::{anyhow, Result};
use dialoguer::{Input, PasswordInput};
use std::path::{Path, PathBuf};

use crate::config::Configuration;

pub fn create_config<P: AsRef<Path>>(path: P) -> Result<()> {
    if path.as_ref().exists() {
        return Err(anyhow!("`{}` is exist.", path.as_ref().to_str().unwrap()));
    }

    let aur_user = Input::<String>::new()
        .with_prompt("AUR user name")
        .interact()?;
    let password = PasswordInput::new().with_prompt("Password").interact()?;
    let sys_username = std::env::var("USER")?;

    let mut config = Configuration::default();
    config.account.user = aur_user;
    config.account.pass = password;
    config.account.cookie_file =
        PathBuf::from(format!("/var/tmp/aur-thumbsup-{}.cookie", sys_username));
    config.to_file(&path)?;

    println!("Created `{}`", &path.as_ref().to_str().unwrap());
    Ok(())
}
