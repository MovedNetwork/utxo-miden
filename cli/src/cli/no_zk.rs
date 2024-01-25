use crate::{
    config::Config,
    utils::{self, HexString},
    utxo::{Key, SerializedUtxo, SignedTransaction, State, Transaction, Utxo},
};
use anyhow::Context;
use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand)]
pub enum Command {
    /// Generate a new key pair to use for signing UTXO transactions
    GenerateKeyPair,
    /// Create a new state with a single UTXO in it
    CreateState {
        #[clap(short, long)]
        owner: HexString,
        #[clap(short, long)]
        value: HexString,
    },
    /// Send a transaction, updating the state.
    /// A key file must exist for the signer (one can be created via `GenerateKeyPair`).
    /// The transaction is specified as a JSON file (see `SerializedTransaction`).
    ProcessTransaction {
        #[clap(short, long)]
        signer: HexString,
        #[clap(short, long)]
        tx_path: String,
    },
}

impl Command {
    pub fn execute(self, config: &Config) -> anyhow::Result<()> {
        match self {
            Self::GenerateKeyPair => {
                let key = Key::random()?;
                let output = serde_json::to_string_pretty(&key)?;
                let owner: HexString = key.owner.into();
                let output_path = config
                    .no_zk_path
                    .join(format!("{}.json", hex::encode(owner.bytes)));
                std::fs::write(&output_path, output)?;
                println!("Key written to {output_path:?}");
            }
            Self::CreateState { owner, value } => {
                let mut state = State::empty();
                let initial_utxo = Utxo::try_from(SerializedUtxo { owner, value })?;
                state.insert(initial_utxo)?;
                utils::write_state(&state, config)?;
            }
            Self::ProcessTransaction { signer, tx_path } => {
                let signer: String = signer.into();
                let key_path = config.no_zk_path.join(format!(
                    "{}.json",
                    signer.strip_prefix("0x").unwrap_or(&signer)
                ));
                let state_path = config.no_zk_path.join("state.json");
                let key: Key =
                    utils::read_json_file(&key_path).context("Failed to read key file")?;
                let mut state: State =
                    utils::read_json_file(&state_path).context("Failed to read state file")?;
                let transaction: Transaction = utils::read_json_file(Path::new(&tx_path))
                    .context("Failed to read transaction file")?;

                let signed_transaction = SignedTransaction::new(transaction, key.pair)
                    .context("Failed to sign transaction")?;
                state
                    .process_tx(signed_transaction)
                    .context("Error processing transaction")?;

                utils::write_state(&state, config)?;
            }
        }

        Ok(())
    }
}
