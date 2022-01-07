use anyhow::Result;
use std::path::Path;

use crate::config::Configuration;

pub fn check_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let _ = Configuration::load_and_verify_config(&path)?;

    println!(
        "`{}` file is valid and secure.",
        path.as_ref().to_str().unwrap()
    );

    Ok(())
}
