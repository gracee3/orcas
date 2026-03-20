use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

fn default_persist_extended_history() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub title: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCapabilities {
    pub experimental_api: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_out_notification_methods: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_info: ClientInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<InitializeCapabilities>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResponse {
    #[serde(default)]
    pub server_info: Option<ServerInfo>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub platform_family: Option<String>,
    #[serde(default)]
    pub platform_os: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<AskForApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    #[serde(default)]
    pub experimental_raw_events: bool,
    #[serde(default = "default_persist_extended_history")]
    pub persist_extended_history: bool,
}

impl Default for ThreadStartParams {
    fn default() -> Self {
        Self {
            model: None,
            model_provider: None,
            cwd: None,
            approval_policy: Some(AskForApproval::default()),
            approvals_reviewer: None,
            sandbox: None,
            config: None,
            service_name: Some("orcas".to_string()),
            base_instructions: None,
            developer_instructions: None,
            ephemeral: Some(false),
            experimental_raw_events: false,
            persist_extended_history: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadResumeParams {
    pub thread_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<AskForApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer_instructions: Option<String>,
    #[serde(default = "default_persist_extended_history")]
    pub persist_extended_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadReadParams {
    pub thread_id: String,
    pub include_turns: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_providers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kinds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_term: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
    pub thread_id: String,
    pub input: Vec<UserInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<AskForApproval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_policy: Option<SandboxPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSteerParams {
    pub thread_id: String,
    pub input: Vec<UserInput>,
    pub expected_turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptParams {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserInput {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default)]
        text_elements: Vec<TextElement>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextElement {
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AskForApproval {
    Mode(ApprovalMode),
    Granular { granular: GranularApproval },
}

impl Default for AskForApproval {
    fn default() -> Self {
        Self::Mode(ApprovalMode::Never)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalMode {
    Untrusted,
    OnFailure,
    OnRequest,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranularApproval {
    pub sandbox_approval: bool,
    pub rules: bool,
    pub skill_approval: bool,
    pub request_permissions: bool,
    pub mcp_elicitations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalsReviewer {
    User,
    GuardianSubagent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SandboxPolicy {
    DangerFullAccess,
    ReadOnly {
        #[serde(default)]
        access: Value,
        network_access: bool,
    },
    WorkspaceWrite {
        #[serde(default)]
        writable_roots: Vec<String>,
        #[serde(default)]
        read_only_access: Value,
        network_access: bool,
        exclude_tmpdir_env_var: bool,
        exclude_slash_tmp: bool,
    },
    ExternalSandbox {
        #[serde(default)]
        network_access: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResponse {
    pub thread: Thread,
    pub model: String,
    pub model_provider: String,
    #[serde(default)]
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadResumeResponse {
    pub thread: Thread,
    pub model: String,
    pub model_provider: String,
    #[serde(default)]
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadReadResponse {
    pub thread: Thread,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResponse {
    pub data: Vec<Thread>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStartResponse {
    pub turn: Turn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSteerResponse {
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnInterruptResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResponse {
    pub data: Vec<Model>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartedNotification {
    pub thread: Thread,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStartedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCompletedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    #[serde(default)]
    pub preview: String,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default)]
    pub model_provider: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    pub status: ThreadStatus,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub cli_version: String,
    #[serde(default)]
    pub source: Option<Value>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub turns: Vec<Turn>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    pub status: TurnStatus,
    #[serde(default)]
    pub error: Option<TurnError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnError {
    pub message: String,
    #[serde(default)]
    pub additional_details: Option<String>,
    #[serde(default)]
    pub codex_error_info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ThreadItem {
    pub fn text(&self) -> Option<&str> {
        self.extra.get("text").and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub model: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThreadStatus {
    #[serde(rename = "notLoaded")]
    NotLoaded,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "systemError")]
    SystemError,
    #[serde(rename = "active")]
    Active {
        #[serde(default)]
        active_flags: Vec<String>,
    },
}

impl ThreadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NotLoaded => "notLoaded",
            Self::Idle => "idle",
            Self::SystemError => "systemError",
            Self::Active { .. } => "active",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}

impl TurnStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
            Self::InProgress => "inProgress",
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        ClientInfo, InitializeParams, InitializeResponse, SandboxPolicy, TextElement,
        ThreadStartParams, ThreadStatus, TurnStatus, UserInput,
    };

    #[test]
    fn initialize_params_omits_optional_capabilities_when_absent() {
        let params = InitializeParams {
            client_info: ClientInfo {
                name: "orcas".to_string(),
                title: None,
                version: "0.1.0".to_string(),
            },
            capabilities: None,
        };

        let value = serde_json::to_value(&params).expect("serialize initialize params");
        assert_eq!(value["clientInfo"]["name"], "orcas");
        assert!(value.get("capabilities").is_none());

        let round_trip: InitializeParams =
            serde_json::from_value(value).expect("deserialize initialize params");
        assert!(round_trip.capabilities.is_none());
    }

    #[test]
    fn initialize_response_defaults_missing_optional_fields_to_none() {
        let response =
            serde_json::from_value::<InitializeResponse>(json!({})).expect("deserialize response");

        assert!(response.server_info.is_none());
        assert!(response.user_agent.is_none());
        assert!(response.platform_family.is_none());
        assert!(response.platform_os.is_none());
    }

    #[test]
    fn thread_start_params_default_matches_expected_operator_contract() {
        let params = ThreadStartParams::default();
        let value = serde_json::to_value(&params).expect("serialize thread start params");

        assert_eq!(value["serviceName"], "orcas");
        assert_eq!(value["approvalPolicy"], "never");
        assert_eq!(value["ephemeral"], false);
        assert_eq!(value["experimentalRawEvents"], false);
        assert_eq!(value["persistExtendedHistory"], true);
    }

    #[test]
    fn thread_start_params_deserialize_missing_booleans_with_defaults() {
        let params = serde_json::from_value::<ThreadStartParams>(json!({}))
            .expect("deserialize thread start params");

        assert!(!params.experimental_raw_events);
        assert!(params.persist_extended_history);
        assert!(matches!(params.approval_policy, None));
        assert!(params.ephemeral.is_none());
    }

    #[test]
    fn sandbox_policy_uses_stable_tagged_shape() {
        let policy = SandboxPolicy::WorkspaceWrite {
            writable_roots: vec!["/repo".to_string()],
            read_only_access: json!({"kind":"minimal"}),
            network_access: true,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: true,
        };

        let value = serde_json::to_value(&policy).expect("serialize sandbox policy");
        assert_eq!(value["type"], "workspaceWrite");
        assert_eq!(value["writable_roots"][0], "/repo");

        let round_trip: SandboxPolicy =
            serde_json::from_value(value).expect("deserialize sandbox policy");
        match round_trip {
            SandboxPolicy::WorkspaceWrite {
                writable_roots,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
                ..
            } => {
                assert_eq!(writable_roots, vec!["/repo".to_string()]);
                assert!(network_access);
                assert!(!exclude_tmpdir_env_var);
                assert!(exclude_slash_tmp);
            }
            other => panic!("unexpected sandbox policy: {other:?}"),
        }
    }

    #[test]
    fn user_input_text_preserves_tag_and_default_text_elements() {
        let input = serde_json::from_value::<UserInput>(json!({
            "type": "text",
            "text": "hello"
        }))
        .expect("deserialize user input");

        match input {
            UserInput::Text {
                text,
                text_elements,
            } => {
                assert_eq!(text, "hello");
                assert!(text_elements.is_empty());
            }
        }

        let serialized = serde_json::to_value(&UserInput::Text {
            text: "hello".to_string(),
            text_elements: vec![TextElement::default()],
        })
        .expect("serialize user input");
        assert_eq!(serialized["type"], "text");
        assert_eq!(serialized["text"], "hello");
    }

    #[test]
    fn thread_status_active_defaults_active_flags_when_missing() {
        let status = serde_json::from_value::<ThreadStatus>(json!({
            "type": "active"
        }))
        .expect("deserialize thread status");

        match status {
            ThreadStatus::Active { active_flags } => assert!(active_flags.is_empty()),
            other => panic!("unexpected thread status: {other:?}"),
        }
    }

    #[test]
    fn turn_status_serializes_with_camel_case_variant_names() {
        let value = serde_json::to_value(TurnStatus::InProgress).expect("serialize turn status");
        assert_eq!(value, json!("inProgress"));

        let round_trip: TurnStatus =
            serde_json::from_value(value).expect("deserialize turn status");
        assert!(matches!(round_trip, TurnStatus::InProgress));
    }
}
