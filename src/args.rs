use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use std::path::PathBuf;

lazy_static! {
    static ref DEFAULT_CONFIG_FILE: PathBuf =
        PathBuf::from(std::env::var("HOME").unwrap() + "/.config/aur-thumbsup.toml");
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
pub struct Arguments {
    /// Configuration file
    ///
    #[clap(
        short = 'c',
        long,
        parse(from_os_str),
        default_value = DEFAULT_CONFIG_FILE.to_str().unwrap()
    )]
    pub config: PathBuf,

    #[clap(subcommand)]
    pub cmd: Option<Commands>,
}

#[derive(Subcommand, PartialEq, Debug)]
pub enum Commands {
    #[clap(about = "Vote for packages")]
    Vote {
        #[clap(required = true)]
        packages: Vec<String>,
    },

    #[clap(about = "Unvote packages")]
    Unvote {
        #[clap(required = true)]
        packages: Vec<String>,
    },

    #[clap(about = "Unvote for all installed packages")]
    UnvoteAll {},

    #[clap(about = "Check for voted packages")]
    Check {
        #[clap(required = true)]
        packages: Vec<String>,
    },

    #[clap(about = "List all voted packages")]
    List {},

    #[clap(about = "Vote/Unvote for installed packages")]
    Autovote {},

    #[clap(about = "Create configuration file")]
    CreateConfig {
        #[clap(required = true, parse(from_os_str))]
        path: PathBuf,
    },

    #[clap(about = "Check configuration file")]
    CheckConfig {
        #[clap(required = true, parse(from_os_str))]
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{FromArgMatches, IntoApp};

    #[test]
    fn main_flags() {
        Arguments::into_app().debug_assert();

        // No argument
        let args =
            Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec!["test"]))
                .unwrap();
        assert_eq!(args.config, DEFAULT_CONFIG_FILE.to_path_buf());
        assert_eq!(args.cmd, None);

        // short config flag
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "-c",
            "/etc/aur-thumbsup.toml",
        ]))
        .unwrap();
        assert_eq!(args.config, PathBuf::from(r"/etc/aur-thumbsup.toml"));
        assert_eq!(args.cmd, None);

        // long config flag
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "--config",
            "/etc/aur-thumbsup.toml",
        ]))
        .unwrap();
        assert_eq!(args.config, PathBuf::from(r"/etc/aur-thumbsup.toml"));
        assert_eq!(args.cmd, None);
    }

    #[test]
    fn vote_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "vote", "pkg1", "pkg2"]),
        )
        .unwrap();
        assert_eq!(
            args.cmd,
            Some(Commands::Vote {
                packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
            })
        );
    }

    #[test]
    fn unvote_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "unvote", "pkg1", "pkg2"]),
        )
        .unwrap();
        assert_eq!(
            args.cmd,
            Some(Commands::Unvote {
                packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
            })
        );
    }

    #[test]
    fn unvote_all_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "unvote-all"]),
        )
        .unwrap();
        assert_eq!(args.cmd, Some(Commands::UnvoteAll {}));
    }

    #[test]
    fn check_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "check", "pkg1", "pkg2"]),
        )
        .unwrap();
        assert_eq!(
            args.cmd,
            Some(Commands::Check {
                packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
            })
        );
    }

    #[test]
    fn list_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "list"]),
        )
        .unwrap();
        assert_eq!(args.cmd, Some(Commands::List {}));
    }

    #[test]
    fn autovote_cmd() {
        let args = Arguments::from_arg_matches(
            &Arguments::into_app().get_matches_from(vec!["test", "autovote"]),
        )
        .unwrap();
        assert_eq!(args.cmd, Some(Commands::Autovote {}));
    }

    #[test]
    fn create_config_cmd() {
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "create-config",
            "/etc/aur-thumbsup.toml",
        ]))
        .unwrap();
        assert_eq!(
            args.cmd,
            Some(Commands::CreateConfig {
                path: PathBuf::from(r"/etc/aur-thumbsup.toml")
            })
        );
    }

    #[test]
    fn check_config_cmd() {
        let args = Arguments::from_arg_matches(&Arguments::into_app().get_matches_from(vec![
            "test",
            "check-config",
            "/etc/aur-thumbsup.toml",
        ]))
        .unwrap();
        assert_eq!(
            args.cmd,
            Some(Commands::CheckConfig {
                path: PathBuf::from(r"/etc/aur-thumbsup.toml")
            })
        );
    }
}
