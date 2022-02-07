use anyhow::Result;
use clap::Parser;
use std::process;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

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

fn init_log() -> Result<()> {
    let filter = match EnvFilter::try_from_env("RUST_LOG") {
        Ok(f) => f,
        Err(_) => EnvFilter::try_new("aur_thumbsup=warn")?,
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .try_init()
        .expect("Initialize tracing-subscriber");
    Ok(())
}

fn run_app() -> Result<()> {
    let arguments = Arguments::parse();
    init_log().expect("Initialize logging");
    debug!("Run with {:?}", arguments);

    if let Some(cmd) = arguments.cmd {
        match cmd {
            Commands::Vote { packages } => vote(arguments.config, packages)?,
            Commands::Unvote { packages } => unvote(arguments.config, packages)?,
            Commands::UnvoteAll {} => unvote_all(arguments.config)?,
            Commands::Check { packages } => check(arguments.config, packages)?,
            Commands::List {} => list(arguments.config)?,
            Commands::Autovote {} => autovote(arguments.config)?,
            Commands::CreateConfig { path } => create_config(path)?,
            Commands::CheckConfig { path } => check_config(path)?,
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
