use clap::Parser;
use config::Config;

mod cli;
mod config;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    let config = if let Some(path) = args.config.as_deref() {
        Config::load(path)?
    } else {
        Config::default()
    };

    cli::execute(&config, args.command)
}
