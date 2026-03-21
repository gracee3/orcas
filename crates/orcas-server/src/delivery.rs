use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::Value;

use orcas_core::ipc::{
    NotificationDeliveryJob, NotificationDeliveryJobStatus, NotificationRecipient,
    NotificationSubscription, NotificationTransportKind, OperatorNotificationCandidate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDeliveryOutcome {
    pub status: NotificationDeliveryJobStatus,
    pub receipt: Option<Value>,
    pub error: Option<String>,
}

impl NotificationDeliveryOutcome {
    pub fn delivered(receipt: Option<Value>) -> Self {
        Self {
            status: NotificationDeliveryJobStatus::Delivered,
            receipt,
            error: None,
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            status: NotificationDeliveryJobStatus::Failed,
            receipt: None,
            error: Some(error.into()),
        }
    }
}

pub struct NotificationDeliveryContext<'a> {
    pub job: &'a NotificationDeliveryJob,
    pub candidate: &'a OperatorNotificationCandidate,
    pub recipient: &'a NotificationRecipient,
    pub subscription: &'a NotificationSubscription,
}

pub trait NotificationDeliveryTransport: Send + Sync {
    fn kind(&self) -> NotificationTransportKind;
    fn dispatch(&self, context: &NotificationDeliveryContext<'_>) -> NotificationDeliveryOutcome;
}

#[derive(Debug, Default)]
pub struct LogNotificationDeliveryTransport;

impl NotificationDeliveryTransport for LogNotificationDeliveryTransport {
    fn kind(&self) -> NotificationTransportKind {
        NotificationTransportKind::Log
    }

    fn dispatch(&self, context: &NotificationDeliveryContext<'_>) -> NotificationDeliveryOutcome {
        NotificationDeliveryOutcome::delivered(Some(serde_json::json!({
            "job_id": context.job.job_id,
            "candidate_id": context.job.candidate_id,
            "subscription_id": context.job.subscription_id,
            "recipient_id": context.job.recipient_id,
            "transport_kind": context.job.transport_kind,
            "candidate_status": context.candidate.status,
            "recipient_enabled": context.recipient.enabled,
            "subscription_enabled": context.subscription.enabled,
        })))
    }
}

#[derive(Debug, Default)]
pub struct MockNotificationDeliveryTransport {
    outcomes: Mutex<BTreeMap<String, NotificationDeliveryOutcome>>,
}

impl MockNotificationDeliveryTransport {
    pub fn with_job_outcome(
        job_id: impl Into<String>,
        outcome: NotificationDeliveryOutcome,
    ) -> Self {
        let mut outcomes = BTreeMap::new();
        outcomes.insert(job_id.into(), outcome);
        Self {
            outcomes: Mutex::new(outcomes),
        }
    }

    pub fn set_job_outcome(&self, job_id: impl Into<String>, outcome: NotificationDeliveryOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(job_id.into(), outcome);
        }
    }
}

impl NotificationDeliveryTransport for MockNotificationDeliveryTransport {
    fn kind(&self) -> NotificationTransportKind {
        NotificationTransportKind::Mock
    }

    fn dispatch(&self, context: &NotificationDeliveryContext<'_>) -> NotificationDeliveryOutcome {
        self.outcomes
            .lock()
            .ok()
            .and_then(|outcomes| outcomes.get(&context.job.job_id).cloned())
            .unwrap_or_else(|| {
                NotificationDeliveryOutcome::delivered(Some(serde_json::json!({
                    "mock": true,
                    "job_id": context.job.job_id,
                })))
            })
    }
}
