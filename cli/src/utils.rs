use miden_crypto::{Felt, Word};
use std::str::FromStr;
use winter_utils::{Deserializable, Serializable};

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
