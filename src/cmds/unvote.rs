use anyhow::{anyhow, Result};
use colored::Colorize;
use std::path::Path;

use crate::aur::{Authentication, VoteResult};
use crate::config::Configuration;

pub fn unvote<P: AsRef<Path>>(config_path: P, packages: Vec<String>) -> Result<()> {
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let results = auth.unvote(&packages)?;
    for result in results.iter() {
        println!("{}", fancy(&result)?);
    }

    Ok(())
}

pub fn fancy(status: &(String, VoteResult)) -> Result<String> {
    Ok(format!(
        "{}    {}",
        status.0.bold().white(),
        match status.1 {
            VoteResult::AlreadyUnVoted => "Already unvoted".bright_green(),
            VoteResult::UnVoted => "Unvoted".bright_green(),
            VoteResult::Failed => "Failed".bright_red(),
            VoteResult::NotAvailable => "N/A".bright_red(),
            _ => return Err(anyhow!("Incorrect vote status")),
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fancy() {
        // Already unvoted
        let status = ("pacman-mirrorup".to_owned(), VoteResult::AlreadyUnVoted);
        let result = fancy(&status).unwrap();
        let expect = format!(
            "{}    {}",
            status.0.bold().white(),
            "Already unvoted".bright_green()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Unvoted
        let status = ("pacman-mirrorup".to_owned(), VoteResult::UnVoted);
        let result = fancy(&status).unwrap();
        let expect = format!(
            "{}    {}",
            status.0.bold().white(),
            "Unvoted".bright_green()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Failed
        let status = ("pacman-mirrorup".to_owned(), VoteResult::Failed);
        let result = fancy(&status).unwrap();
        let expect = format!("{}    {}", status.0.bold().white(), "Failed".bright_red());
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // N/A
        let status = ("pacman-mirrorup".to_owned(), VoteResult::NotAvailable);
        let result = fancy(&status).unwrap();
        let expect = format!("{}    {}", status.0.bold().white(), "N/A".bright_red());
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);
    }
}
