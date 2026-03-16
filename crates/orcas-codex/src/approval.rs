use async_trait::async_trait;
use serde_json::Value;

use orcas_core::{OrcasError, OrcasResult};

#[derive(Debug, Clone)]
pub enum ApprovalDecision {
    Result(Value),
    Error {
        code: i64,
        message: String,
        data: Option<Value>,
    },
}

#[async_trait]
pub trait ApprovalRouter: Send + Sync {
    async fn resolve(&self, method: &str, params: Option<Value>) -> OrcasResult<ApprovalDecision>;
}

#[derive(Debug, Default)]
pub struct RejectingApprovalRouter;

#[async_trait]
impl ApprovalRouter for RejectingApprovalRouter {
    async fn resolve(&self, method: &str, _params: Option<Value>) -> OrcasResult<ApprovalDecision> {
        Ok(ApprovalDecision::Error {
            code: -32_000,
            message: format!("orcas does not yet handle server request `{method}`"),
            data: None,
        })
    }
}

impl From<&str> for ApprovalDecision {
    fn from(message: &str) -> Self {
        Self::Error {
            code: -32_000,
            message: message.to_string(),
            data: None,
        }
    }
}

impl From<OrcasError> for ApprovalDecision {
    fn from(error: OrcasError) -> Self {
        Self::Error {
            code: -32_000,
            message: error.to_string(),
            data: None,
        }
    }
}
