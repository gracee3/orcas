use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::info;

use orcas_core::ipc::{
    OperatorInboxMirrorApplyRequest, OperatorInboxMirrorApplyResponse,
    OperatorInboxMirrorCheckpointQueryRequest, OperatorInboxMirrorCheckpointQueryResponse,
    OperatorInboxMirrorGetResponse, OperatorInboxMirrorListResponse,
    OperatorNotificationAckRequest, OperatorNotificationAckResponse,
    OperatorNotificationGetRequest, OperatorNotificationGetResponse,
    OperatorNotificationListRequest, OperatorNotificationListResponse,
    OperatorNotificationSuppressRequest, OperatorNotificationSuppressResponse,
};
use orcas_core::{AppPaths, OrcasResult};

use crate::store::InboxMirrorStore;

#[derive(Debug, Clone)]
pub struct InboxMirrorServerConfig {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
}

#[derive(Clone)]
pub struct InboxMirrorServer {
    store: Arc<InboxMirrorStore>,
}

impl InboxMirrorServer {
    pub fn new(store: InboxMirrorStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub async fn serve(self, bind_addr: SocketAddr) -> OrcasResult<()> {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        self.serve_with_listener(listener).await
    }

    pub async fn serve_with_listener(self, listener: tokio::net::TcpListener) -> OrcasResult<()> {
        let app = Router::new()
            .route("/operator-inbox/mirror/apply", post(apply))
            .route(
                "/operator-inbox/{origin_node_id}/checkpoint",
                get(checkpoint),
            )
            .route("/operator-inbox/{origin_node_id}/items", get(list_items))
            .route(
                "/operator-inbox/{origin_node_id}/items/{item_id}",
                get(get_item),
            )
            .route(
                "/operator-notifications/list",
                post(list_notification_candidates),
            )
            .route(
                "/operator-notifications/get",
                post(get_notification_candidate),
            )
            .route(
                "/operator-notifications/ack",
                post(ack_notification_candidate),
            )
            .route(
                "/operator-notifications/suppress",
                post(suppress_notification_candidate),
            )
            .with_state(self.store);
        let bind_addr = listener.local_addr()?;
        info!(%bind_addr, "orcas-server listening");
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn apply(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorInboxMirrorApplyRequest>,
) -> Result<Json<OperatorInboxMirrorApplyResponse>, String> {
    let result = store
        .apply_batch(
            request.origin_node_id.as_str(),
            request.checkpoint.clone(),
            &request.changes,
        )
        .map_err(|error| error.to_string())?;
    Ok(Json(OperatorInboxMirrorApplyResponse {
        origin_node_id: request.origin_node_id,
        checkpoint: result.checkpoint,
        mirror_checkpoint: result.mirror_checkpoint,
        applied_changes: result.applied_changes,
        skipped_changes: result.skipped_changes,
    }))
}

async fn checkpoint(
    State(store): State<Arc<InboxMirrorStore>>,
    Path(OperatorInboxMirrorCheckpointQueryRequest { origin_node_id }): Path<
        OperatorInboxMirrorCheckpointQueryRequest,
    >,
) -> Result<Json<OperatorInboxMirrorCheckpointQueryResponse>, String> {
    let checkpoint = store
        .checkpoint(origin_node_id.as_str())
        .map_err(|error| error.to_string())?;
    Ok(Json(OperatorInboxMirrorCheckpointQueryResponse {
        origin_node_id,
        checkpoint,
    }))
}

async fn list_items(
    State(store): State<Arc<InboxMirrorStore>>,
    Path(origin_node_id): Path<String>,
) -> Result<Json<OperatorInboxMirrorListResponse>, String> {
    let response = store
        .list(origin_node_id.as_str(), None)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn get_item(
    State(store): State<Arc<InboxMirrorStore>>,
    Path((origin_node_id, item_id)): Path<(String, String)>,
) -> Result<Json<OperatorInboxMirrorGetResponse>, String> {
    let item = store
        .get(origin_node_id.as_str(), item_id.as_str())
        .map_err(|error| error.to_string())?;
    Ok(Json(OperatorInboxMirrorGetResponse {
        origin_node_id,
        item,
    }))
}

async fn list_notification_candidates(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorNotificationListRequest>,
) -> Result<Json<OperatorNotificationListResponse>, String> {
    let response = store
        .notification_candidates(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn get_notification_candidate(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorNotificationGetRequest>,
) -> Result<Json<OperatorNotificationGetResponse>, String> {
    let candidate = store
        .notification_candidate(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(OperatorNotificationGetResponse {
        origin_node_id: request.origin_node_id,
        candidate,
    }))
}

async fn ack_notification_candidate(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorNotificationAckRequest>,
) -> Result<Json<OperatorNotificationAckResponse>, String> {
    let response = store
        .acknowledge_notification_candidate(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn suppress_notification_candidate(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorNotificationSuppressRequest>,
) -> Result<Json<OperatorNotificationSuppressResponse>, String> {
    let response = store
        .suppress_notification_candidate(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

pub fn app(store: InboxMirrorStore) -> Router {
    Router::new()
        .route("/operator-inbox/mirror/apply", post(apply))
        .route(
            "/operator-inbox/{origin_node_id}/checkpoint",
            get(checkpoint),
        )
        .route("/operator-inbox/{origin_node_id}/items", get(list_items))
        .route(
            "/operator-inbox/{origin_node_id}/items/{item_id}",
            get(get_item),
        )
        .route(
            "/operator-notifications/list",
            post(list_notification_candidates),
        )
        .route(
            "/operator-notifications/get",
            post(get_notification_candidate),
        )
        .route(
            "/operator-notifications/ack",
            post(ack_notification_candidate),
        )
        .route(
            "/operator-notifications/suppress",
            post(suppress_notification_candidate),
        )
        .with_state(Arc::new(store))
}

#[allow(dead_code)]
pub async fn serve_from_paths(paths: AppPaths, bind_addr: SocketAddr) -> OrcasResult<()> {
    let db_path = paths.data_dir.join("server_inbox.db");
    let store = InboxMirrorStore::open(db_path)?;
    InboxMirrorServer::new(store).serve(bind_addr).await
}
