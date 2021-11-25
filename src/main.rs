use anyhow::Result;
use log::{debug, error, LevelFilter};
use std::{env, process};
use structopt::StructOpt;

mod args;
mod aur;
mod cmds;
mod config;
mod helper;

use crate::args::{Arguments, Commands};
use crate::cmds::autovote::autovote;
use crate::cmds::check::check;
use crate::cmds::checkconfig::check_config;
use crate::cmds::createconfig::create_config;
use crate::cmds::list::list;
use crate::cmds::unvote::unvote;
use crate::cmds::unvoteall::unvote_all;
use crate::cmds::vote::vote;

fn run_app() -> Result<()> {
    let arguments = Arguments::from_args();
    let log_level = match arguments.verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        4 => LevelFilter::Trace,
        _ => LevelFilter::Trace,
    };

    let mut log_builder = pretty_env_logger::formatted_builder();
    if let Ok(value) = env::var("RUST_LOG") {
        log_builder.parse_filters(&value);
    } else {
        log_builder.filter_level(log_level);
    }
    log_builder.init();
    debug!("Run with {:?}", arguments);

    if let Some(cmd) = arguments.cmd {
        match cmd {
            Commands::Vote { packages } => vote(arguments.config, packages, arguments.quiet)?,
            Commands::Unvote { packages } => unvote(arguments.config, packages, arguments.quiet)?,
            Commands::UnvoteAll {} => unvote_all(arguments.config, arguments.quiet)?,
            Commands::Check { packages } => check(arguments.config, packages, arguments.quiet)?,
            Commands::List {} => list(arguments.config, arguments.quiet)?,
            Commands::Autovote {} => autovote(arguments.config, arguments.quiet)?,
            Commands::CreateConfig { path } => create_config(path, arguments.quiet)?,
            Commands::CheckConfig { path } => check_config(path, arguments.quiet)?,
        }
    }

    Ok(())
}

fn main() {
    process::exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    });
}
