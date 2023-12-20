use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Path to the file with UTXO MidenVM assemby (masm) code to execute.
    pub code_path: PathBuf,
    /// Path to the file with inputs used to initialize the MidenVM
    pub inputs_path: PathBuf,
    /// Path to the file where output from the CLI are written
    pub outputs_path: PathBuf,
    /// Directory where data from no-zk part of the CLI is saved.
    pub no_zk_path: PathBuf,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&data)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        let base_path = Path::new("masm");
        Self {
            code_path: base_path.join("utxo.masm"),
            inputs_path: base_path.join("utxo.inputs"),
            outputs_path: base_path.join("utxo.outputs"),
            no_zk_path: Path::new("example").into(),
        }
    }
}
