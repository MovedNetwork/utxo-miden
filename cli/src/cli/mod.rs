use crate::{config::Config, utils, utxo::SerializedSignedTransaction};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

pub mod no_zk;
pub mod prove;

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
    Prove {
        #[clap(short, long)]
        tx_path: String,
    },
    #[clap(subcommand)]
    NoZk(no_zk::Command),
}

pub fn execute(config: &Config, command: Command) -> anyhow::Result<()> {
    match command {
        Command::Prove { tx_path } => {
            let signed_tx: SerializedSignedTransaction =
                utils::read_json_file(Path::new(&tx_path))?;
            let output = prove::execute(config, signed_tx.try_into()?)?;
            output.write_to_file(&config.outputs_path)?;
            println!("Proof written to {:?}", config.outputs_path);
        }
        Command::NoZk(sub_command) => sub_command.execute(config)?,
    }

    Ok(())
}
