use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::aur::Authentication;
use crate::config::Configuration;

pub fn check<P: AsRef<Path>>(config_path: P, packages: Vec<String>) -> Result<()> {
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let voted = auth.check_vote(&packages)?;
    for v in voted.iter() {
        println!("{}", fancy(&v)?);
    }

    Ok(())
}

fn fancy(voted: &(String, Option<bool>)) -> Result<String> {
    Ok(format!(
        "{} {}",
        voted.0.bold().white(),
        match voted.1 {
            Some(status) => match status {
                true => "Yes".bright_green(),
                false => "No".bright_red(),
            },
            None => "N/A".bright_yellow(),
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fancy() {
        // Voted
        let voted = ("pacman-mirrorup".to_owned(), Some(true));
        let result = fancy(&voted).unwrap();
        let expect = format!("{} {}", voted.0.bold().white(), "Yes".bright_green());
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Unvoted
        let voted = ("pacman-mirrorup".to_owned(), Some(false));
        let result = fancy(&voted).unwrap();
        let expect = format!("{} {}", voted.0.bold().white(), "No".bright_red());
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // N/A
        let voted = ("pacman-mirrorup".to_owned(), None);
        let result = fancy(&voted).unwrap();
        let expect = format!("{} {}", voted.0.bold().white(), "N/A".bright_yellow());
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);
    }
}
