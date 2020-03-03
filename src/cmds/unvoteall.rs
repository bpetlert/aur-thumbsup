use anyhow::Result;
use std::path::Path;

use crate::aur::Authentication;
use crate::cmds::unvote::fancy;
use crate::config::Configuration;

pub fn unvote_all<P: AsRef<Path>>(config_path: P) -> Result<()> {
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let voted_pkgs = auth.list_voted_pkgs()?;
    let packages: Vec<String> = voted_pkgs.iter().map(|pkg| pkg.name.to_owned()).collect();
    let results = auth.unvote(&packages)?;
    for result in results.iter() {
        println!("{}", fancy(&result)?);
    }

    Ok(())
}
