use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Run,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Command::Run => println!("TODO: execute a utxo something"),
    }
}
