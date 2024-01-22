use std::{collections::BTreeMap, path::Path, str::FromStr};

/// Inputs used to initialize MidenVM before execution.
#[derive(Debug)]
pub struct InputFile {
    /// String representation of the initial operand stack, composed of chained field elements.
    pub operand_stack: Vec<u64>,
    /// String representation of the initial advice stack, composed of chained field elements.
    pub advice_stack: Vec<u64>,
    /// Optional map of the initial advice map, each from u256 key to vector of field elements.
    pub advice_map: BTreeMap<[u8; 32], Vec<u64>>,
    /// Optional vector of merkle data which will be loaded into the initial merkle store.
    pub merkle_tree: Option<Vec<[u8; 32]>>,
}

impl InputFile {
    pub fn parse(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let input_file: SerializeableInputFile = serde_json::from_str(&data)?;
        input_file.try_into()
    }
}

/// Logically the same as `InputFile` above, but where types are represented in a
/// more serialization-friendly way. This struct is intentionally private because
/// it is an implementation detail. The `InputFile` itself is the API for the rest of the code.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SerializeableInputFile {
    pub operand_stack: Vec<String>,
    pub advice_stack: Vec<String>,
    pub advice_map: BTreeMap<String, Vec<String>>,
    pub merkle_tree: Option<Vec<String>>,
}

impl TryFrom<SerializeableInputFile> for InputFile {
    type Error = anyhow::Error;

    fn try_from(value: SerializeableInputFile) -> Result<Self, Self::Error> {
        let operand_stack: anyhow::Result<Vec<u64>> = value
            .operand_stack
            .into_iter()
            .map(|s| u64::from_str(&s).map_err(Into::into))
            .collect();

        let advice_stack: anyhow::Result<Vec<u64>> = value
            .advice_stack
            .into_iter()
            .map(|s| u64::from_str(&s).map_err(Into::into))
            .collect();

        let advice_map: anyhow::Result<BTreeMap<[u8; 32], Vec<u64>>> = value
            .advice_map
            .into_iter()
            .map(|(key, value)| {
                let mut buf = [0u8; 32];
                let s = key.strip_prefix("0x").unwrap_or(&key);
                hex::decode_to_slice(s, &mut buf)?;
                // Advice map value is a vector of field elements
                let v: anyhow::Result<Vec<u64>> = value
                    .into_iter()
                    .map(|s| u64::from_str(&s).map_err(Into::into))
                    .collect();
                Ok((buf, v?))
            })
            .collect();

        let merkle_tree: Option<anyhow::Result<Vec<[u8; 32]>>> = value.merkle_tree.map(|t| {
            t.into_iter()
                .map(|hex_str| {
                    let mut buf = [0u8; 32];
                    let s = hex_str.strip_prefix("0x").unwrap_or(&hex_str);
                    hex::decode_to_slice(s, &mut buf)?;
                    Ok(buf)
                })
                .collect()
        });

        Ok(Self {
            operand_stack: operand_stack?,
            advice_stack: advice_stack?,
            advice_map: advice_map?,
            merkle_tree: merkle_tree.transpose()?,
        })
    }
}
