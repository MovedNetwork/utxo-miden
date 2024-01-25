use crate::{config::Config, utxo::State};
use anyhow::Context;
use miden_core::{Felt, Word};
use std::{path::Path, str::FromStr};
use winter_utils::{Deserializable, Serializable};

pub fn write_state(state: &State, config: &Config) -> anyhow::Result<()> {
    let state_root: String = HexString::from(state.get_root()).into();
    println!("State root = {state_root}");
    let output = serde_json::to_string_pretty(&state)?;
    let output_path = config.no_zk_path.join("state.json");
    std::fs::write(&output_path, output).context("Failed to write state file")?;
    println!("State written to {output_path:?}");
    Ok(())
}

pub fn read_json_file<T: for<'a> serde::Deserialize<'a>>(path: &Path) -> anyhow::Result<T> {
    let data = std::fs::read_to_string(path)?;
    let t: T = serde_json::from_str(&data)?;
    Ok(t)
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct HexString {
    pub bytes: Vec<u8>,
}

impl FromStr for HexString {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_str = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(hex_str)?;
        Ok(Self { bytes })
    }
}

impl From<HexString> for String {
    fn from(value: HexString) -> Self {
        format!("0x{}", hex::encode(value.bytes))
    }
}

impl TryFrom<String> for HexString {
    type Error = hex::FromHexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<HexString> for Word {
    type Error = anyhow::Error;

    fn try_from(value: HexString) -> Result<Self, Self::Error> {
        let mut word = Word::default();
        for (x, e) in value.bytes.chunks_exact(8).zip(word.iter_mut()) {
            *e = felt_from_bytes(x)?;
        }
        Ok(word)
    }
}

impl TryFrom<HexString> for Felt {
    type Error = anyhow::Error;

    fn try_from(value: HexString) -> Result<Self, Self::Error> {
        felt_from_bytes(&value.bytes)
    }
}

impl From<Word> for HexString {
    fn from(value: Word) -> Self {
        let mut bytes = Vec::new();
        for e in value {
            e.write_into(&mut bytes);
        }
        Self { bytes }
    }
}

fn felt_from_bytes(bytes: &[u8]) -> anyhow::Result<Felt> {
    Felt::read_from_bytes(bytes)
        .map_err(|e| anyhow::Error::msg(format!("Failed to parse field element {e:?}")))
}
