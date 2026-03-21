use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::info;

use crate::delivery::{LogNotificationDeliveryTransport, MockNotificationDeliveryTransport};
use orcas_core::ipc::{
    NotificationDeliveryJobGetRequest, NotificationDeliveryJobGetResponse,
    NotificationDeliveryJobListRequest, NotificationDeliveryJobListResponse,
    NotificationDeliveryRunPendingRequest, NotificationDeliveryRunPendingResponse,
    NotificationRecipientListRequest, NotificationRecipientListResponse,
    NotificationRecipientUpsertRequest, NotificationRecipientUpsertResponse,
    NotificationSubscriptionListRequest, NotificationSubscriptionListResponse,
    NotificationSubscriptionSetEnabledRequest, NotificationSubscriptionSetEnabledResponse,
    NotificationSubscriptionUpsertRequest, NotificationSubscriptionUpsertResponse,
    NotificationTransportKind, OperatorInboxMirrorApplyRequest, OperatorInboxMirrorApplyResponse,
    OperatorInboxMirrorCheckpointQueryRequest, OperatorInboxMirrorCheckpointQueryResponse,
    OperatorInboxMirrorGetResponse, OperatorInboxMirrorListResponse,
    OperatorNotificationAckRequest, OperatorNotificationAckResponse,
    OperatorNotificationGetRequest, OperatorNotificationGetResponse,
    OperatorNotificationListRequest, OperatorNotificationListResponse,
    OperatorNotificationSuppressRequest, OperatorNotificationSuppressResponse,
    OperatorRemoteActionClaimRequest, OperatorRemoteActionClaimResponse,
    OperatorRemoteActionCompleteRequest, OperatorRemoteActionCompleteResponse,
    OperatorRemoteActionCreateRequest, OperatorRemoteActionCreateResponse,
    OperatorRemoteActionFailRequest, OperatorRemoteActionFailResponse,
    OperatorRemoteActionGetRequest, OperatorRemoteActionGetResponse,
    OperatorRemoteActionListRequest, OperatorRemoteActionListResponse,
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
            .route(
                "/operator-notifications/recipients/upsert",
                post(upsert_notification_recipient),
            )
            .route(
                "/operator-notifications/recipients/list",
                post(list_notification_recipients),
            )
            .route(
                "/operator-notifications/subscriptions/upsert",
                post(upsert_notification_subscription),
            )
            .route(
                "/operator-notifications/subscriptions/list",
                post(list_notification_subscriptions),
            )
            .route(
                "/operator-notifications/subscriptions/set_enabled",
                post(set_notification_subscription_enabled),
            )
            .route(
                "/operator-notifications/delivery-jobs/list",
                post(list_notification_delivery_jobs),
            )
            .route(
                "/operator-notifications/delivery-jobs/get",
                post(get_notification_delivery_job),
            )
            .route(
                "/operator-notifications/delivery/run_pending",
                post(run_pending_notification_delivery_jobs),
            )
            .route(
                "/operator-actions/request",
                post(create_remote_action_request),
            )
            .route(
                "/operator-actions/list",
                post(list_remote_action_requests),
            )
            .route("/operator-actions/get", post(get_remote_action_request))
            .route("/operator-actions/claim", post(claim_remote_action_requests))
            .route(
                "/operator-actions/complete",
                post(complete_remote_action_request),
            )
            .route("/operator-actions/fail", post(fail_remote_action_request))
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

async fn upsert_notification_recipient(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationRecipientUpsertRequest>,
) -> Result<Json<NotificationRecipientUpsertResponse>, String> {
    let response = store
        .upsert_notification_recipient(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn list_notification_recipients(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationRecipientListRequest>,
) -> Result<Json<NotificationRecipientListResponse>, String> {
    let response = store
        .list_notification_recipients(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn upsert_notification_subscription(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationSubscriptionUpsertRequest>,
) -> Result<Json<NotificationSubscriptionUpsertResponse>, String> {
    let response = store
        .upsert_notification_subscription(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn list_notification_subscriptions(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationSubscriptionListRequest>,
) -> Result<Json<NotificationSubscriptionListResponse>, String> {
    let response = store
        .list_notification_subscriptions(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn set_notification_subscription_enabled(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationSubscriptionSetEnabledRequest>,
) -> Result<Json<NotificationSubscriptionSetEnabledResponse>, String> {
    let response = store
        .set_notification_subscription_enabled(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn list_notification_delivery_jobs(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationDeliveryJobListRequest>,
) -> Result<Json<NotificationDeliveryJobListResponse>, String> {
    let response = store
        .list_notification_delivery_jobs(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn get_notification_delivery_job(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationDeliveryJobGetRequest>,
) -> Result<Json<NotificationDeliveryJobGetResponse>, String> {
    let job = store
        .get_notification_delivery_job(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(NotificationDeliveryJobGetResponse { job }))
}

async fn run_pending_notification_delivery_jobs(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<NotificationDeliveryRunPendingRequest>,
) -> Result<Json<NotificationDeliveryRunPendingResponse>, String> {
    let response = match request
        .transport_kind
        .unwrap_or(NotificationTransportKind::Log)
    {
        NotificationTransportKind::Log => store
            .dispatch_pending_notification_delivery_jobs(
                &LogNotificationDeliveryTransport,
                request.limit,
            )
            .map_err(|error| error.to_string())?,
        NotificationTransportKind::Mock => store
            .dispatch_pending_notification_delivery_jobs(
                &MockNotificationDeliveryTransport::default(),
                request.limit,
            )
            .map_err(|error| error.to_string())?,
        other => {
            return Err(format!(
                "notification delivery transport kind `{other:?}` is not supported by the local server"
            ));
        }
    };
    Ok(Json(response))
}

async fn create_remote_action_request(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionCreateRequest>,
) -> Result<Json<OperatorRemoteActionCreateResponse>, String> {
    let response = store
        .create_remote_action_request(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn list_remote_action_requests(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionListRequest>,
) -> Result<Json<OperatorRemoteActionListResponse>, String> {
    let response = store
        .list_remote_action_requests(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn get_remote_action_request(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionGetRequest>,
) -> Result<Json<OperatorRemoteActionGetResponse>, String> {
    let response = store
        .get_remote_action_request(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn claim_remote_action_requests(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionClaimRequest>,
) -> Result<Json<OperatorRemoteActionClaimResponse>, String> {
    let response = store
        .claim_remote_action_requests(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn complete_remote_action_request(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionCompleteRequest>,
) -> Result<Json<OperatorRemoteActionCompleteResponse>, String> {
    let response = store
        .complete_remote_action_request(&request)
        .map_err(|error| error.to_string())?;
    Ok(Json(response))
}

async fn fail_remote_action_request(
    State(store): State<Arc<InboxMirrorStore>>,
    Json(request): Json<OperatorRemoteActionFailRequest>,
) -> Result<Json<OperatorRemoteActionFailResponse>, String> {
    let response = store
        .fail_remote_action_request(&request)
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
        .route(
            "/operator-notifications/recipients/upsert",
            post(upsert_notification_recipient),
        )
        .route(
            "/operator-notifications/recipients/list",
            post(list_notification_recipients),
        )
        .route(
            "/operator-notifications/subscriptions/upsert",
            post(upsert_notification_subscription),
        )
        .route(
            "/operator-notifications/subscriptions/list",
            post(list_notification_subscriptions),
        )
        .route(
            "/operator-notifications/subscriptions/set_enabled",
            post(set_notification_subscription_enabled),
        )
        .route(
            "/operator-notifications/delivery-jobs/list",
            post(list_notification_delivery_jobs),
        )
        .route(
            "/operator-notifications/delivery-jobs/get",
            post(get_notification_delivery_job),
        )
        .route(
            "/operator-notifications/delivery/run_pending",
            post(run_pending_notification_delivery_jobs),
        )
        .route("/operator-actions/request", post(create_remote_action_request))
        .route("/operator-actions/list", post(list_remote_action_requests))
        .route("/operator-actions/get", post(get_remote_action_request))
        .route("/operator-actions/claim", post(claim_remote_action_requests))
        .route("/operator-actions/complete", post(complete_remote_action_request))
        .route("/operator-actions/fail", post(fail_remote_action_request))
        .with_state(Arc::new(store))
}

#[allow(dead_code)]
pub async fn serve_from_paths(paths: AppPaths, bind_addr: SocketAddr) -> OrcasResult<()> {
    let db_path = paths.data_dir.join("server_inbox.db");
    let store = InboxMirrorStore::open(db_path)?;
    InboxMirrorServer::new(store).serve(bind_addr).await
}
