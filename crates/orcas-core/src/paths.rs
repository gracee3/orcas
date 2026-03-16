use std::path::PathBuf;

use crate::error::{OrcasError, OrcasResult};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub state_file: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> OrcasResult<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| OrcasError::Config("unable to resolve config directory".to_string()))?
            .join("orcas");
        let data_dir = dirs::data_dir()
            .ok_or_else(|| OrcasError::Config("unable to resolve data directory".to_string()))?
            .join("orcas");
        let logs_dir = data_dir.join("logs");
        Ok(Self {
            config_file: config_dir.join("config.toml"),
            state_file: data_dir.join("state.json"),
            config_dir,
            data_dir,
            logs_dir,
        })
    }

    pub async fn ensure(&self) -> OrcasResult<()> {
        tokio::fs::create_dir_all(&self.config_dir).await?;
        tokio::fs::create_dir_all(&self.data_dir).await?;
        tokio::fs::create_dir_all(&self.logs_dir).await?;
        Ok(())
    }
}
