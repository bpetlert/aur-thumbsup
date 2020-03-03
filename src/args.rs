use lazy_static::lazy_static;
use std::path::PathBuf;
use structopt::StructOpt;

lazy_static! {
    static ref DEFAULT_CONFIG_FILE: PathBuf =
        PathBuf::from(std::env::var("HOME").unwrap() + "/.config/aur-thumbsup.toml");
}

#[derive(StructOpt, PartialEq, Debug)]
#[structopt(author, about)]
pub struct Arguments {
    /// Configuration file
    ///
    #[structopt(
        short = "c",
        long = "config",
        parse(from_os_str),
        default_value = DEFAULT_CONFIG_FILE.to_str().unwrap()
    )]
    pub config: PathBuf,

    /// Increment verbosity level once per call
    /// [error, -v: warn, -vv: info, -vvv: debug, -vvvv: trace]
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(subcommand)]
    pub cmd: Option<Commands>,
}

#[derive(StructOpt, PartialEq, Debug)]
pub enum Commands {
    #[structopt(about = "Vote for packages")]
    Vote {
        #[structopt(required = true)]
        packages: Vec<String>,
    },

    #[structopt(about = "Unvote packages")]
    Unvote {
        #[structopt(required = true)]
        packages: Vec<String>,
    },

    #[structopt(about = "Unvote for all installed packages")]
    UnvoteAll {},

    #[structopt(about = "Check for voted packages")]
    Check {
        #[structopt(required = true)]
        packages: Vec<String>,
    },

    #[structopt(about = "List all voted packages")]
    List {},

    #[structopt(about = "Vote/Unvote for installed packages")]
    Autovote {},

    #[structopt(about = "Create configuration file")]
    CreateConfig {
        #[structopt(required = true, parse(from_os_str))]
        path: PathBuf,
    },

    #[structopt(about = "Check configuration file")]
    CheckConfig {
        #[structopt(required = true, parse(from_os_str))]
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_flags() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: None
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&["test"]))
        );

        assert_eq!(
            Arguments {
                config: PathBuf::from(r"/etc/aur-thumbsup.toml"),
                verbose: 0,
                cmd: None
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&[
                "test",
                "-c",
                "/etc/aur-thumbsup.toml"
            ]))
        );

        assert_eq!(
            Arguments {
                config: PathBuf::from(r"/etc/aur-thumbsup.toml"),
                verbose: 0,
                cmd: None
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&[
                "test",
                "--config",
                "/etc/aur-thumbsup.toml"
            ]))
        );

        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 4,
                cmd: None
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&["test", "-vvvv"]))
        );

        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 4,
                cmd: None
            },
            Arguments::from_clap(
                &Arguments::clap().get_matches_from(&["test", "-v", "-v", "-v", "-v"])
            )
        );

        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 4,
                cmd: None
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&[
                "test",
                "--verbose",
                "--verbose",
                "--verbose",
                "--verbose"
            ]))
        );
    }

    #[test]
    fn test_vote() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: Some(Commands::Vote {
                    packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
                })
            },
            Arguments::from_clap(
                &Arguments::clap().get_matches_from(&["test", "vote", "pkg1", "pkg2"])
            )
        );
    }

    #[test]
    fn test_unvote() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: Some(Commands::Unvote {
                    packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
                })
            },
            Arguments::from_clap(
                &Arguments::clap().get_matches_from(&["test", "unvote", "pkg1", "pkg2"])
            )
        );
    }

    #[test]
    fn test_check() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: Some(Commands::Check {
                    packages: vec!["pkg1".to_owned(), "pkg2".to_owned()]
                })
            },
            Arguments::from_clap(
                &Arguments::clap().get_matches_from(&["test", "check", "pkg1", "pkg2"])
            )
        );
    }

    #[test]
    fn test_list() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: Some(Commands::List {})
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&["test", "list"]))
        );
    }

    #[test]
    fn test_autovote() {
        assert_eq!(
            Arguments {
                config: DEFAULT_CONFIG_FILE.to_path_buf(),
                verbose: 0,
                cmd: Some(Commands::Autovote {})
            },
            Arguments::from_clap(&Arguments::clap().get_matches_from(&["test", "autovote"]))
        );
    }
}
