use crate::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod run;

#[derive(Parser)]
pub struct Cli {
    /// Path to JSON file containing the config.
    /// If not present the default config values are used.
    #[clap(short, long)]
    pub config: Option<PathBuf>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Run,
}

pub fn execute(config: &Config, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Run => run::execute(config)?,
    }

    Ok(())
}
