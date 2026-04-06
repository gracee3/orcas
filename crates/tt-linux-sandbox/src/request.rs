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
