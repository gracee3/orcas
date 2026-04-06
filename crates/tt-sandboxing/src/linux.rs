use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tt_runtime::protocol::types::SandboxPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxSandboxRequest {
    pub policy: SandboxPolicy,
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxSandboxExecution {
    DirectExec {
        command: Vec<OsString>,
        cwd: Option<PathBuf>,
    },
    Bubblewrap {
        program: PathBuf,
        args: Vec<OsString>,
        command: Vec<OsString>,
        cwd: Option<PathBuf>,
    },
}

#[derive(Debug, Error)]
pub enum LinuxSandboxRequestError {
    #[error("sandbox policy did not describe any command arguments")]
    EmptyCommand,
}

pub fn plan_execution(
    request: &LinuxSandboxRequest,
    bwrap_path: &Path,
) -> Result<LinuxSandboxExecution, LinuxSandboxRequestError> {
    if request.command.is_empty() {
        return Err(LinuxSandboxRequestError::EmptyCommand);
    }

    let command = request
        .command
        .iter()
        .cloned()
        .map(OsString::from)
        .collect::<Vec<_>>();

    match &request.policy {
        SandboxPolicy::DangerFullAccess | SandboxPolicy::ExternalSandbox { .. } => {
            Ok(LinuxSandboxExecution::DirectExec {
                command,
                cwd: request.cwd.clone(),
            })
        }
        SandboxPolicy::ReadOnly {
            access,
            network_access,
        } => Ok(LinuxSandboxExecution::Bubblewrap {
            program: bwrap_path.to_path_buf(),
            args: render_bwrap_args(
                *network_access,
                false,
                false,
                &[],
                roots_from_access(access),
                request.cwd.as_deref(),
            ),
            command,
            cwd: request.cwd.clone(),
        }),
        SandboxPolicy::WorkspaceWrite {
            writable_roots,
            read_only_access,
            network_access,
            exclude_tmpdir_env_var,
            exclude_slash_tmp,
        } => Ok(LinuxSandboxExecution::Bubblewrap {
            program: bwrap_path.to_path_buf(),
            args: render_bwrap_args(
                *network_access,
                *exclude_tmpdir_env_var,
                *exclude_slash_tmp,
                writable_roots,
                roots_from_access(read_only_access),
                request.cwd.as_deref(),
            ),
            command,
            cwd: request.cwd.clone(),
        }),
    }
}

fn render_bwrap_args(
    network_access: bool,
    exclude_tmpdir_env_var: bool,
    exclude_slash_tmp: bool,
    writable_roots: &[String],
    readonly_roots: Vec<PathBuf>,
    cwd: Option<&Path>,
) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("--die-with-parent"),
        OsString::from("--proc"),
        OsString::from("/proc"),
        OsString::from("--dev"),
        OsString::from("/dev"),
    ];

    if !network_access {
        args.push(OsString::from("--unshare-net"));
    }

    if exclude_tmpdir_env_var {
        for name in ["TMPDIR", "TMP", "TEMP"] {
            args.push(OsString::from("--unsetenv"));
            args.push(OsString::from(name));
        }
    }

    if exclude_slash_tmp {
        args.push(OsString::from("--tmpfs"));
        args.push(OsString::from("/tmp"));
    }

    args.push(OsString::from("--ro-bind"));
    args.push(OsString::from("/"));
    args.push(OsString::from("/"));

    for root in writable_roots {
        args.push(OsString::from("--bind"));
        let root = OsString::from(root);
        args.push(root.clone());
        args.push(root);
    }

    for root in readonly_roots {
        args.push(OsString::from("--ro-bind"));
        let root = root.into_os_string();
        args.push(root.clone());
        args.push(root);
    }

    if let Some(cwd) = cwd {
        args.push(OsString::from("--chdir"));
        args.push(cwd.as_os_str().to_owned());
    }

    args.push(OsString::from("--"));
    args
}

fn roots_from_access(access: &Value) -> Vec<PathBuf> {
    match access {
        Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(PathBuf::from))
            .collect(),
        Value::Object(map) => map
            .get("roots")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(PathBuf::from))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn danger_full_access_bypasses_bwrap() {
        let request = LinuxSandboxRequest {
            policy: SandboxPolicy::DangerFullAccess,
            command: vec!["/bin/echo".to_string(), "hello".to_string()],
            cwd: Some(PathBuf::from("/repo")),
        };

        let execution =
            plan_execution(&request, Path::new("/usr/bin/bwrap")).expect("plan execution");

        assert_eq!(
            execution,
            LinuxSandboxExecution::DirectExec {
                command: vec![OsString::from("/bin/echo"), OsString::from("hello")],
                cwd: Some(PathBuf::from("/repo")),
            }
        );
    }

    #[test]
    fn workspace_write_requests_mount_roots_and_tmpfs() {
        let request = LinuxSandboxRequest {
            policy: SandboxPolicy::WorkspaceWrite {
                writable_roots: vec!["/repo".to_string(), "/repo/worktrees/tt-1".to_string()],
                read_only_access: Value::Array(vec![Value::String("/repo/.git".to_string())]),
                network_access: false,
                exclude_tmpdir_env_var: true,
                exclude_slash_tmp: true,
            },
            command: vec!["/bin/echo".to_string()],
            cwd: Some(PathBuf::from("/repo")),
        };

        let execution =
            plan_execution(&request, Path::new("/usr/bin/bwrap")).expect("plan execution");

        assert_eq!(
            execution,
            LinuxSandboxExecution::Bubblewrap {
                program: PathBuf::from("/usr/bin/bwrap"),
                args: vec![
                    OsString::from("--die-with-parent"),
                    OsString::from("--proc"),
                    OsString::from("/proc"),
                    OsString::from("--dev"),
                    OsString::from("/dev"),
                    OsString::from("--unshare-net"),
                    OsString::from("--unsetenv"),
                    OsString::from("TMPDIR"),
                    OsString::from("--unsetenv"),
                    OsString::from("TMP"),
                    OsString::from("--unsetenv"),
                    OsString::from("TEMP"),
                    OsString::from("--tmpfs"),
                    OsString::from("/tmp"),
                    OsString::from("--ro-bind"),
                    OsString::from("/"),
                    OsString::from("/"),
                    OsString::from("--bind"),
                    OsString::from("/repo"),
                    OsString::from("/repo"),
                    OsString::from("--bind"),
                    OsString::from("/repo/worktrees/tt-1"),
                    OsString::from("/repo/worktrees/tt-1"),
                    OsString::from("--ro-bind"),
                    OsString::from("/repo/.git"),
                    OsString::from("/repo/.git"),
                    OsString::from("--chdir"),
                    OsString::from("/repo"),
                    OsString::from("--"),
                ],
                command: vec![OsString::from("/bin/echo")],
                cwd: Some(PathBuf::from("/repo")),
            }
        );
    }

    #[test]
    fn empty_command_is_rejected() {
        let request = LinuxSandboxRequest {
            policy: SandboxPolicy::DangerFullAccess,
            command: Vec::new(),
            cwd: None,
        };

        let error = plan_execution(&request, Path::new("/usr/bin/bwrap"))
            .expect_err("empty command should fail");

        assert_eq!(
            error.to_string(),
            "sandbox policy did not describe any command arguments"
        );
    }
}
