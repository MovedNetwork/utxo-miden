use crate::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod prove;

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
    /// Generate a proof from executing MidenVM.
    Prove,
}

pub fn execute(config: &Config, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Prove => {
            let output = prove::execute(config)?;
            output.write_to_file(&config.outputs_path)?;
            println!("Proof written to {:?}", config.outputs_path);
        }
    }

    Ok(())
}
