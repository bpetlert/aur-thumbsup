use anyhow::{anyhow, Result};
use colored::Colorize;
use std::{fmt::Write, path::Path};

use crate::{
    aur::{Authentication, VoteResult},
    config::Configuration,
};

pub fn vote<P: AsRef<Path>>(config_path: P, packages: Vec<String>) -> Result<()> {
    let config = Configuration::load_and_verify_config(&config_path)?;
    let mut auth = Authentication::new();
    auth.login(&config.account)?;
    let results = auth.vote(&packages)?;

    let mut output = String::new();
    for result in results.iter() {
        writeln!(output, "{}", fancy(result)?)?;
    }
    print!("{}", output);

    Ok(())
}

pub fn fancy(status: &(String, VoteResult)) -> Result<String> {
    Ok(format!(
        "{}    {}",
        status.0.bold().white(),
        match status.1 {
            VoteResult::AlreadyVoted => "Already voted".bright_green(),
            VoteResult::Voted => "Voted".bright_green(),
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
        // Already voted
        let status = ("pacman-mirrorup".to_owned(), VoteResult::AlreadyVoted);
        let result = fancy(&status).unwrap();
        let expect = format!(
            "{}    {}",
            status.0.bold().white(),
            "Already voted".bright_green()
        );
        assert_eq!(result, expect, "`{}` != `{}`", result, expect);

        // Voted
        let status = ("pacman-mirrorup".to_owned(), VoteResult::Voted);
        let result = fancy(&status).unwrap();
        let expect = format!("{}    {}", status.0.bold().white(), "Voted".bright_green());
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
