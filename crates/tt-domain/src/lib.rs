//! Pure TT v2 domain types.
//!
//! This crate is intentionally narrow: it defines TT-owned concepts that sit
//! on top of Codex runtime state without depending on sqlite, transport, or UI
//! frameworks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid as _;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub objective: String,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectStatus {
    Active,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnit {
    pub id: String,
    pub project_id: String,
    pub slug: Option<String>,
    pub title: String,
    pub task: String,
    pub status: WorkUnitStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkUnitStatus {
    Ready,
    Blocked,
    Running,
    Review,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadBinding {
    pub codex_thread_id: String,
    pub work_unit_id: Option<String>,
    pub role: ThreadRole,
    pub status: ThreadBindingStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThreadRole {
    Develop,
    Review,
    Test,
    Integrate,
    Todo,
    Chat,
    Learn,
    Handoff,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThreadBindingStatus {
    Proposed,
    Bound,
    Detached,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBinding {
    pub id: String,
    pub codex_thread_id: String,
    pub repo_root: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub base_ref: Option<String>,
    pub base_commit: Option<String>,
    pub landing_target: Option<String>,
    pub strategy: WorkspaceStrategy,
    pub sync_policy: WorkspaceSyncPolicy,
    pub cleanup_policy: WorkspaceCleanupPolicy,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceStrategy {
    Shared,
    DedicatedWorktree,
    Ephemeral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceSyncPolicy {
    Manual,
    RebaseBeforeReview,
    RebaseBeforeLanding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceCleanupPolicy {
    KeepUntilClosed,
    PruneAfterLanding,
    KeepForAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceStatus {
    Requested,
    Ready,
    Dirty,
    Ahead,
    Behind,
    Conflicted,
    Merged,
    Abandoned,
    Pruned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeRun {
    pub id: String,
    pub workspace_binding_id: String,
    pub readiness: MergeReadiness,
    pub authorization: MergeAuthorizationStatus,
    pub execution: MergeExecutionStatus,
    pub head_commit: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeReadiness {
    Unknown,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeAuthorizationStatus {
    NotRequested,
    Authorized,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeExecutionStatus {
    NotStarted,
    Running,
    Succeeded,
    Failed,
}
