//! Local orchestration for TT v2.
//!
//! The daemon coordinates TT overlay state, Codex runtime state, and git state.
//! It should remain a thin service layer rather than a policy sink.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tt_codex::{CodexHome, CodexSessionCatalog};
use tt_domain::{Project, ThreadBinding, WorkUnit, WorkspaceBinding};
use tt_store::OverlayStore;
use tt_ui_core::DashboardSummary;

pub const TT_DAEMON_API_VERSION: &str = "v2";

#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub codex_home: Option<PathBuf>,
    pub codex_state_db: Option<PathBuf>,
    pub codex_session_index: Option<PathBuf>,
    pub project_count: usize,
    pub work_unit_count: usize,
    pub bound_thread_count: usize,
    pub ready_workspace_count: usize,
}

#[derive(Debug, Clone)]
pub struct DaemonService {
    store: Arc<OverlayStore>,
    codex_home: Option<CodexHome>,
}

impl DaemonService {
    pub fn new(store: OverlayStore) -> Self {
        Self {
            store: Arc::new(store),
            codex_home: None,
        }
    }

    pub fn with_codex_home(store: OverlayStore, codex_home: CodexHome) -> Self {
        Self {
            store: Arc::new(store),
            codex_home: Some(codex_home),
        }
    }

    pub fn codex_home(&self) -> Option<&CodexHome> {
        self.codex_home.as_ref()
    }

    pub fn store(&self) -> &OverlayStore {
        self.store.as_ref()
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.store.list_projects()
    }

    pub fn list_work_units(&self, project_id: Option<&str>) -> Result<Vec<WorkUnit>> {
        self.store.list_work_units(project_id)
    }

    pub fn list_thread_bindings(&self) -> Result<Vec<ThreadBinding>> {
        self.store.list_thread_bindings()
    }

    pub fn list_workspace_bindings(&self) -> Result<Vec<WorkspaceBinding>> {
        self.store.list_workspace_bindings()
    }

    pub fn codex_catalog(&self) -> Result<Option<CodexSessionCatalog>> {
        self.codex_home
            .as_ref()
            .map(|home| home.session_catalog())
            .transpose()
    }

    pub fn status(&self) -> Result<DaemonStatus> {
        let codex_home = self.codex_home.as_ref();
        Ok(DaemonStatus {
            codex_home: codex_home.map(|home| home.root().to_path_buf()),
            codex_state_db: codex_home.map(|home| home.state_db_path()),
            codex_session_index: codex_home.map(|home| home.session_index_path()),
            project_count: self.store.count_projects()?,
            work_unit_count: self.store.count_work_units()?,
            bound_thread_count: self.store.count_bound_threads()?,
            ready_workspace_count: self.store.count_ready_workspaces()?,
        })
    }

    pub fn dashboard_summary(&self) -> Result<DashboardSummary> {
        let status = self.status()?;
        Ok(DashboardSummary {
            active_projects: status.project_count,
            active_work_units: status.work_unit_count,
            bound_threads: status.bound_thread_count,
            ready_workspaces: status.ready_workspace_count,
        })
    }

    pub fn repository_summary(
        &self,
        cwd: impl AsRef<Path>,
    ) -> Result<Option<tt_ui_core::GitRepositorySummary>> {
        let Some(inspection) = tt_git::GitRepository::inspect(cwd)? else {
            return Ok(None);
        };
        Ok(Some(tt_ui_core::GitRepositorySummary {
            repository_root: inspection.repository_root.display().to_string(),
            current_worktree: inspection
                .current_worktree
                .map(|path| path.display().to_string()),
            current_branch: inspection.current_branch,
            current_head_commit: inspection.current_head_commit,
            dirty: inspection.dirty,
            upstream: inspection.upstream,
            ahead_by: inspection.ahead_by,
            behind_by: inspection.behind_by,
            merge_ready: inspection.merge_readiness == tt_domain::MergeReadiness::Ready,
            worktree_count: inspection.worktrees.len(),
        }))
    }

    pub fn codex_home_root(&self) -> Option<&Path> {
        self.codex_home.as_ref().map(|home| home.root())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;
    use tt_domain::{ProjectStatus, ThreadBindingStatus, ThreadRole, WorkUnitStatus, WorkspaceCleanupPolicy, WorkspaceStatus, WorkspaceStrategy, WorkspaceSyncPolicy};

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 8, 12, 0, 0).unwrap()
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed: {status}", args);
    }

    fn setup_repo() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "tt-daemon-v2-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let repo = root.join("repo");
        let worktree = root.join("worktree");
        std::fs::create_dir_all(&repo).expect("create repo");
        run_git(&repo, &["init", "-b", "main"]);
        run_git(&repo, &["config", "user.name", "TT Test"]);
        run_git(&repo, &["config", "user.email", "tt@example.com"]);
        std::fs::write(repo.join("README.md"), "tt\n").expect("write file");
        run_git(&repo, &["add", "README.md"]);
        run_git(&repo, &["commit", "-m", "initial"]);
        run_git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "tt/tt-1",
                worktree.to_str().expect("worktree"),
                "HEAD",
            ],
        );
        (repo, worktree)
    }

    #[test]
    fn status_reflects_store_counts() {
        let dir = tempdir().expect("tempdir");
        let store = OverlayStore::open_in_dir(dir.path()).expect("open store");
        let service = DaemonService::new(store);

        service
            .store()
            .upsert_project(&Project {
                id: "p1".into(),
                slug: "alpha".into(),
                title: "Alpha".into(),
                objective: "Ship".into(),
                status: ProjectStatus::Active,
                created_at: ts(),
                updated_at: ts(),
            })
            .expect("upsert project");
        service
            .store()
            .upsert_work_unit(&WorkUnit {
                id: "wu1".into(),
                project_id: "p1".into(),
                slug: Some("chunk".into()),
                title: "Chunk".into(),
                task: "Do the work".into(),
                status: WorkUnitStatus::Ready,
                created_at: ts(),
                updated_at: ts(),
            })
            .expect("upsert work unit");
        service
            .store()
            .upsert_thread_binding(&ThreadBinding {
                codex_thread_id: "thread-1".into(),
                work_unit_id: Some("wu1".into()),
                role: ThreadRole::Develop,
                status: ThreadBindingStatus::Bound,
                notes: None,
                created_at: ts(),
                updated_at: ts(),
            })
            .expect("upsert binding");
        service
            .store()
            .upsert_workspace_binding(&WorkspaceBinding {
                id: "ws1".into(),
                codex_thread_id: "thread-1".into(),
                repo_root: "/repo".into(),
                worktree_path: None,
                branch_name: None,
                base_ref: None,
                base_commit: None,
                landing_target: None,
                strategy: WorkspaceStrategy::DedicatedWorktree,
                sync_policy: WorkspaceSyncPolicy::RebaseBeforeLanding,
                cleanup_policy: WorkspaceCleanupPolicy::PruneAfterLanding,
                status: WorkspaceStatus::Ready,
                created_at: ts(),
                updated_at: ts(),
            })
            .expect("upsert workspace");

        let status = service.status().expect("status");
        let summary = service.dashboard_summary().expect("summary");

        assert_eq!(status.project_count, 1);
        assert_eq!(status.work_unit_count, 1);
        assert_eq!(status.bound_thread_count, 1);
        assert_eq!(status.ready_workspace_count, 1);
        assert_eq!(summary.bound_threads, 1);
    }

    #[test]
    fn codex_catalog_is_optional_when_unconfigured() {
        let dir = tempdir().expect("tempdir");
        let service = DaemonService::new(OverlayStore::open_in_dir(dir.path()).expect("open store"));
        assert!(service.codex_catalog().expect("catalog").is_none());
    }

    #[test]
    fn repository_summary_is_optional_outside_a_repo() {
        let dir = tempdir().expect("tempdir");
        let service = DaemonService::new(OverlayStore::open_in_dir(dir.path()).expect("open store"));
        assert!(service
            .repository_summary(dir.path())
            .expect("summary")
            .is_none());
    }

    #[test]
    fn repository_summary_reports_clean_repo() {
        let (_, worktree) = setup_repo();
        let dir = tempdir().expect("tempdir");
        let service = DaemonService::new(OverlayStore::open_in_dir(dir.path()).expect("open store"));
        let summary = service
            .repository_summary(&worktree)
            .expect("summary")
            .expect("repo");

        assert_eq!(summary.current_branch.as_deref(), Some("tt/tt-1"));
        assert!(!summary.dirty);
        assert!(summary.merge_ready);
        assert_eq!(summary.worktree_count, 2);
    }
}
