use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{OrcasError, OrcasResult};
use crate::paths::AppPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub codex: CodexDaemonConfig,
    pub defaults: DefaultsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            codex: CodexDaemonConfig::default(),
            defaults: DefaultsConfig::default(),
        }
    }
}

impl AppConfig {
    pub async fn load_or_default(paths: &AppPaths) -> OrcasResult<Self> {
        if tokio::fs::try_exists(&paths.config_file).await? {
            let raw = tokio::fs::read_to_string(&paths.config_file).await?;
            Ok(toml::from_str(&raw)?)
        } else {
            Ok(Self::default())
        }
    }

    pub async fn write_default_if_missing(paths: &AppPaths) -> OrcasResult<Self> {
        paths.ensure().await?;
        let config = Self::load_or_default(paths).await?;
        if !tokio::fs::try_exists(&paths.config_file).await? {
            let raw = toml::to_string_pretty(&config)?;
            tokio::fs::write(&paths.config_file, raw).await?;
        }
        Ok(config)
    }

    pub fn resolve_codex_bin(&self) -> OrcasResult<PathBuf> {
        if self.codex.binary_path.as_os_str().is_empty() {
            return Err(OrcasError::Config(
                "codex.binary_path must be set to a concrete local build path".to_string(),
            ));
        }
        Ok(self.codex.binary_path.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexDaemonConfig {
    pub binary_path: PathBuf,
    pub listen_url: String,
    pub connection_mode: CodexConnectionMode,
    pub reconnect: ReconnectPolicy,
    pub config_overrides: Vec<String>,
}

impl Default for CodexDaemonConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("/home/emmy/git/codex/codex-rs/target/debug/codex"),
            listen_url: "ws://127.0.0.1:4500".to_string(),
            connection_mode: CodexConnectionMode::SpawnIfNeeded,
            reconnect: ReconnectPolicy::default(),
            config_overrides: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodexConnectionMode {
    ConnectOnly,
    SpawnIfNeeded,
    SpawnAlways,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 150,
            max_delay_ms: 5_000,
            multiplier: 2.0,
            max_attempts: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    pub cwd: Option<PathBuf>,
    pub model: Option<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            cwd: None,
            model: Some("gpt-5".to_string()),
        }
    }
}
