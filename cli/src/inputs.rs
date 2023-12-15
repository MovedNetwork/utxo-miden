use std::{path::Path, str::FromStr};

/// Inputs used to initialize MidenVM before execution.
#[derive(Debug)]
pub struct InputFile {
    /// String representation of the initial operand stack, composed of chained field elements.
    pub operand_stack: Vec<u64>,
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
            merkle_tree: merkle_tree.transpose()?,
        })
    }
}
