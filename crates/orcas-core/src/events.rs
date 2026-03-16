use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub received_at: DateTime<Utc>,
    pub source: String,
    pub event: OrcasEvent,
}

impl EventEnvelope {
    pub fn new(source: impl Into<String>, event: OrcasEvent) -> Self {
        Self {
            received_at: Utc::now(),
            source: source.into(),
            event,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrcasEvent {
    ConnectionStateChanged(ConnectionState),
    ThreadStarted {
        thread_id: String,
        preview: String,
    },
    ThreadStatusChanged {
        thread_id: String,
        status: String,
    },
    TurnStarted {
        thread_id: String,
        turn_id: String,
    },
    TurnCompleted {
        thread_id: String,
        turn_id: String,
        status: String,
    },
    ItemStarted {
        thread_id: String,
        turn_id: String,
        item_id: String,
        item_type: String,
    },
    ItemCompleted {
        thread_id: String,
        turn_id: String,
        item_id: String,
        item_type: String,
    },
    AgentMessageDelta {
        thread_id: String,
        turn_id: String,
        item_id: String,
        delta: String,
    },
    ServerRequest {
        method: String,
    },
    Warning {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    pub endpoint: String,
    pub status: String,
    pub detail: Option<String>,
}
