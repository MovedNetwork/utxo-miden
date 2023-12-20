use crate::{
    config::Config,
    utils::HexString,
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
                let key_pair = miden_crypto::dsa::rpo_falcon512::KeyPair::new()?;
                let key = Key {
                    owner: key_pair.public_key().into(),
                    pair: key_pair,
                };
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
                write_state(&state, config)?;
            }
            Self::ProcessTransaction { signer, tx_path } => {
                let signer: String = signer.into();
                let key_path = config.no_zk_path.join(format!(
                    "{}.json",
                    signer.strip_prefix("0x").unwrap_or(&signer)
                ));
                let state_path = config.no_zk_path.join("state.json");
                let key: Key = read_json_file(&key_path).context("Failed to read key file")?;
                let mut state: State =
                    read_json_file(&state_path).context("Failed to read state file")?;
                let transaction: Transaction = read_json_file(Path::new(&tx_path))
                    .context("Failed to read transaction file")?;

                let signed_transaction = SignedTransaction::new(transaction, key.pair)
                    .context("Failed to sign transaction")?;
                state
                    .process_tx(signed_transaction)
                    .context("Error processing transaction")?;

                write_state(&state, config)?;
            }
        }

        Ok(())
    }
}

fn write_state(state: &State, config: &Config) -> anyhow::Result<()> {
    let state_root: String = HexString::from(state.get_root()).into();
    println!("State root = {state_root}");
    let output = serde_json::to_string_pretty(&state)?;
    let output_path = config.no_zk_path.join("state.json");
    std::fs::write(&output_path, output).context("Failed to write state file")?;
    println!("State written to {output_path:?}");
    Ok(())
}

fn read_json_file<T: for<'a> serde::Deserialize<'a>>(path: &Path) -> anyhow::Result<T> {
    let data = std::fs::read_to_string(path)?;
    let t: T = serde_json::from_str(&data)?;
    Ok(t)
}
