#![allow(unused_crate_dependencies)]

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {
    /// Working directory to host the daemon in.
    #[arg(long)]
    cwd: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = cli.cwd.unwrap_or(std::env::current_dir()?);
    let runtime = tt_daemon::DaemonRuntime::open(cwd)?;
    let server = tt_daemon::DaemonServer::new(runtime);
    eprintln!("tt daemon listening on {}", server.socket_path().display());
    server.serve()
}
