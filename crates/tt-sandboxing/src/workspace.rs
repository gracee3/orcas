use serde_json::Value;
use tt_core::ipc::TrackedThreadWorkspaceFilesystemScope;
use tt_runtime::protocol::types::SandboxPolicy;

pub fn workspace_write_policy_for_turn(
    scope: &TrackedThreadWorkspaceFilesystemScope,
    cwd: Option<&str>,
) -> Option<SandboxPolicy> {
    let writable_roots = if cwd == Some(scope.repository_root.as_str()) {
        scope.workspace_lifecycle_roots.clone()
    } else {
        scope.worker_turn_roots.clone()
    };
    workspace_write_policy(writable_roots)
}

pub fn workspace_write_policy(writable_roots: Vec<String>) -> Option<SandboxPolicy> {
    if writable_roots.is_empty() {
        return None;
    }

    Some(SandboxPolicy::WorkspaceWrite {
        writable_roots,
        read_only_access: Value::Null,
        network_access: false,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: true,
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use tt_core::ipc::TrackedThreadWorkspaceFilesystemScope;

    fn sample_scope() -> TrackedThreadWorkspaceFilesystemScope {
        TrackedThreadWorkspaceFilesystemScope {
            repository_root: "/repo".to_string(),
            worktree_path: "/repo/worktrees/tt-1".to_string(),
            worktree_parent: "/repo/worktrees".to_string(),
            git_dir: Some("/repo/worktrees/tt-1/.git".to_string()),
            git_common_dir: Some("/repo/.git/worktrees/tt-1".to_string()),
            worker_turn_roots: vec![
                "/repo/worktrees/tt-1".to_string(),
                "/repo/worktrees/tt-1/.git".to_string(),
                "/repo/.git/worktrees/tt-1".to_string(),
            ],
            workspace_lifecycle_roots: vec![
                "/repo/worktrees/tt-1".to_string(),
                "/repo/worktrees/tt-1/.git".to_string(),
                "/repo/.git/worktrees/tt-1".to_string(),
                "/repo".to_string(),
                "/repo/worktrees".to_string(),
            ],
        }
    }

    #[test]
    fn workspace_write_policy_returns_none_for_empty_roots() {
        assert!(workspace_write_policy(Vec::new()).is_none());
    }

    #[test]
    fn workspace_write_policy_uses_expected_runtime_shape() {
        let policy =
            workspace_write_policy(vec!["/repo".to_string()]).expect("workspace write policy");

        assert_eq!(
            serde_json::to_value(policy).expect("serialize policy"),
            json!({
                "type": "workspaceWrite",
                "writable_roots": ["/repo"],
                "read_only_access": null,
                "network_access": false,
                "exclude_tmpdir_env_var": false,
                "exclude_slash_tmp": true,
            })
        );
    }

    #[test]
    fn workspace_write_policy_for_turn_uses_lifecycle_roots_when_cwd_matches_repository_root() {
        let policy = workspace_write_policy_for_turn(&sample_scope(), Some("/repo"))
            .expect("workspace write policy");

        assert_eq!(
            serde_json::to_value(policy).expect("serialize policy"),
            json!({
                "type": "workspaceWrite",
                "writable_roots": [
                    "/repo/worktrees/tt-1",
                    "/repo/worktrees/tt-1/.git",
                    "/repo/.git/worktrees/tt-1",
                    "/repo",
                    "/repo/worktrees",
                ],
                "read_only_access": null,
                "network_access": false,
                "exclude_tmpdir_env_var": false,
                "exclude_slash_tmp": true,
            })
        );
    }

    #[test]
    fn workspace_write_policy_for_turn_uses_worker_roots_when_cwd_differs_from_repository_root() {
        let policy = workspace_write_policy_for_turn(&sample_scope(), Some("/repo/worktrees/tt-1"))
            .expect("workspace write policy");

        assert_eq!(
            serde_json::to_value(policy).expect("serialize policy"),
            json!({
                "type": "workspaceWrite",
                "writable_roots": [
                    "/repo/worktrees/tt-1",
                    "/repo/worktrees/tt-1/.git",
                    "/repo/.git/worktrees/tt-1",
                ],
                "read_only_access": null,
                "network_access": false,
                "exclude_tmpdir_env_var": false,
                "exclude_slash_tmp": true,
            })
        );
    }
}
