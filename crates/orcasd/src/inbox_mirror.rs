use reqwest::Client;
use tracing::debug;

use orcas_core::ipc::{
    OperatorInboxMirrorApplyRequest, OperatorInboxMirrorApplyResponse,
    OperatorInboxMirrorCheckpointQueryRequest, OperatorInboxMirrorCheckpointQueryResponse,
    OperatorInboxMirrorGetResponse, OperatorInboxMirrorListResponse,
};
use orcas_core::{OrcasError, OrcasResult};

#[derive(Debug, Clone)]
pub struct OperatorInboxMirrorHttpClient {
    client: Client,
    base_url: String,
}

impl OperatorInboxMirrorHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub async fn checkpoint(
        &self,
        origin_node_id: &str,
    ) -> OrcasResult<OperatorInboxMirrorCheckpointQueryResponse> {
        let request = OperatorInboxMirrorCheckpointQueryRequest {
            origin_node_id: origin_node_id.to_string(),
        };
        let response = self
            .client
            .get(self.url(&format!(
                "operator-inbox/{}/checkpoint",
                request.origin_node_id
            )))
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorInboxMirrorCheckpointQueryResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        debug!(
            origin_node_id,
            sequence = response.checkpoint.current_sequence,
            "mirror checkpoint fetched"
        );
        Ok(response)
    }

    pub async fn apply(
        &self,
        request: &OperatorInboxMirrorApplyRequest,
    ) -> OrcasResult<OperatorInboxMirrorApplyResponse> {
        let response = self
            .client
            .post(self.url("operator-inbox/mirror/apply"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorInboxMirrorApplyResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn list(&self, origin_node_id: &str) -> OrcasResult<OperatorInboxMirrorListResponse> {
        let response = self
            .client
            .get(self.url(&format!("operator-inbox/{origin_node_id}/items")))
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorInboxMirrorListResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn get(
        &self,
        origin_node_id: &str,
        item_id: &str,
    ) -> OrcasResult<OperatorInboxMirrorGetResponse> {
        let response = self
            .client
            .get(self.url(&format!("operator-inbox/{origin_node_id}/items/{item_id}")))
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorInboxMirrorGetResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }
}
