//! Shared serializable UI models for TT v2.
//!
//! This crate is the frontend seam used by the TUI first and by a future
//! Leptos CSR client later.

use serde::{Deserialize, Serialize};
use tt_domain as _;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub active_projects: usize,
    pub active_work_units: usize,
    pub bound_threads: usize,
    pub ready_workspaces: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepositorySummary {
    pub repository_root: String,
    pub current_worktree: Option<String>,
    pub current_branch: Option<String>,
    pub current_head_commit: Option<String>,
    pub dirty: bool,
    pub upstream: Option<String>,
    pub ahead_by: Option<u64>,
    pub behind_by: Option<u64>,
    pub merge_ready: bool,
    pub worktree_count: usize,
}
