use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, anyhow};
use tt_sandboxing::linux::{LinuxSandboxExecution, LinuxSandboxRequest, plan_execution};

pub enum RequestSource {
    Stdin,
    Json(String),
    File(PathBuf),
}

pub fn load_request(source: RequestSource) -> Result<LinuxSandboxRequest> {
    let raw = match source {
        RequestSource::Stdin => read_stdin()?,
        RequestSource::Json(raw) => raw,
        RequestSource::File(path) => {
            if path.as_os_str() == "-" {
                read_stdin()?
            } else {
                std::fs::read_to_string(&path)
                    .with_context(|| format!("read sandbox request from {}", path.display()))?
            }
        }
    };

    serde_json::from_str(&raw).context("parse sandbox request json")
}

pub fn run_request(request: LinuxSandboxRequest, bwrap_path: &Path) -> Result<ExitStatus> {
    match plan_execution(&request, bwrap_path)? {
        LinuxSandboxExecution::DirectExec { command, cwd } => {
            let (program, args) = command
                .split_first()
                .ok_or_else(|| anyhow!("sandbox request did not contain a command"))?;
            let mut child = Command::new(program);
            child.args(args);
            if let Some(cwd) = cwd {
                child.current_dir(cwd);
            }
            child.status().context("execute direct sandbox command")
        }
        LinuxSandboxExecution::Bubblewrap {
            program,
            args,
            command,
            cwd: _,
        } => {
            let mut child = Command::new(program);
            child.args(args);
            child.args(command);
            child.status().context("execute bubblewrap sandbox command")
        }
    }
}

fn read_stdin() -> Result<String> {
    let mut raw = String::new();
    std::io::stdin()
        .read_to_string(&mut raw)
        .context("read sandbox request from stdin")?;
    Ok(raw)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::ExitStatus;

    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn load_request_parses_json_input() {
        let request = load_request(RequestSource::Json(
            json!({
                "policy": {
                    "type": "dangerFullAccess"
                },
                "command": ["/bin/echo", "hello"],
                "cwd": "/repo"
            })
            .to_string(),
        ))
        .expect("load request");

        assert_eq!(request.cwd, Some(std::path::PathBuf::from("/repo")));
        assert_eq!(request.command, vec!["/bin/echo", "hello"]);
    }

    #[test]
    fn load_request_parses_file_input() {
        let file = NamedTempFile::new().expect("temp file");
        fs::write(
            file.path(),
            json!({
                "policy": {
                    "type": "dangerFullAccess"
                },
                "command": ["/bin/true"]
            })
            .to_string(),
        )
        .expect("write request");

        let request =
            load_request(RequestSource::File(file.path().to_path_buf())).expect("load request");

        assert_eq!(request.command, vec!["/bin/true"]);
        assert_eq!(request.cwd, None);
    }

    #[test]
    fn run_request_executes_direct_commands_without_bubblewrap() {
        let request = LinuxSandboxRequest {
            policy: tt_runtime::protocol::types::SandboxPolicy::DangerFullAccess,
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 7".to_string(),
            ],
            cwd: None,
        };

        let status = run_request(request, Path::new("/usr/bin/bwrap")).expect("run request");

        assert_exit_code(status, 7);
    }

    fn assert_exit_code(status: ExitStatus, expected: i32) {
        assert_eq!(status.code(), Some(expected));
    }
}
