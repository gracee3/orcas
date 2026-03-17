use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ipc::{ThreadLoadedStatus, ThreadMonitorState};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreadRegistry {
    pub threads: BTreeMap<String, ThreadMetadata>,
    pub last_connected_endpoint: Option<String>,
}

impl ThreadRegistry {
    pub fn upsert(&mut self, metadata: ThreadMetadata) {
        self.threads.insert(metadata.id.clone(), metadata);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMetadata {
    pub id: String,
    pub name: Option<String>,
    pub preview: String,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub cwd: Option<PathBuf>,
    pub endpoint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub loaded_status: ThreadLoadedStatus,
    #[serde(default)]
    pub active_flags: Vec<String>,
    #[serde(default)]
    pub active_turn_id: Option<String>,
    #[serde(default)]
    pub last_seen_turn_id: Option<String>,
    #[serde(default)]
    pub recent_output: Option<String>,
    #[serde(default)]
    pub recent_event: Option<String>,
    #[serde(default)]
    pub turn_in_flight: bool,
    #[serde(default)]
    pub monitor_state: ThreadMonitorState,
    #[serde(default = "Utc::now")]
    pub last_sync_at: DateTime<Utc>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub raw_summary: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadDescriptor {
    pub id: String,
    pub model: Option<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDescriptor {
    pub thread_id: String,
    pub turn_id: String,
    pub status: String,
}
