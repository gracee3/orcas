use reqwest::Client;
use tracing::debug;

use orcas_core::ipc::{
    OperatorRemoteActionClaimRequest, OperatorRemoteActionClaimResponse,
    OperatorRemoteActionCompleteRequest, OperatorRemoteActionCompleteResponse,
    OperatorRemoteActionCreateRequest, OperatorRemoteActionCreateResponse,
    OperatorRemoteActionFailRequest, OperatorRemoteActionFailResponse,
    OperatorRemoteActionGetRequest, OperatorRemoteActionGetResponse,
    OperatorRemoteActionListRequest, OperatorRemoteActionListResponse,
};
use orcas_core::{OrcasError, OrcasResult};

#[derive(Debug, Clone)]
pub struct RemoteActionHttpClient {
    client: Client,
    base_url: String,
}

impl RemoteActionHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    pub async fn create(
        &self,
        request: &OperatorRemoteActionCreateRequest,
    ) -> OrcasResult<OperatorRemoteActionCreateResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/request"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionCreateResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn list(
        &self,
        request: &OperatorRemoteActionListRequest,
    ) -> OrcasResult<OperatorRemoteActionListResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/list"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionListResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn get(
        &self,
        request: &OperatorRemoteActionGetRequest,
    ) -> OrcasResult<OperatorRemoteActionGetResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/get"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionGetResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn claim(
        &self,
        request: &OperatorRemoteActionClaimRequest,
    ) -> OrcasResult<OperatorRemoteActionClaimResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/claim"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionClaimResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        debug!(
            origin_node_id = %response.origin_node_id,
            claimed = response.requests.len(),
            "remote action claims fetched"
        );
        Ok(response)
    }

    pub async fn complete(
        &self,
        request: &OperatorRemoteActionCompleteRequest,
    ) -> OrcasResult<OperatorRemoteActionCompleteResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/complete"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionCompleteResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }

    pub async fn fail(
        &self,
        request: &OperatorRemoteActionFailRequest,
    ) -> OrcasResult<OperatorRemoteActionFailResponse> {
        let response = self
            .client
            .post(self.url("operator-actions/fail"))
            .json(request)
            .send()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .error_for_status()
            .map_err(|error| OrcasError::Transport(error.to_string()))?
            .json::<OperatorRemoteActionFailResponse>()
            .await
            .map_err(|error| OrcasError::Transport(error.to_string()))?;
        Ok(response)
    }
}
