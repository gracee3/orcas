use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

mod request;

use request::{RequestSource, load_request, run_request};

#[derive(Debug, Parser)]
#[command(
    name = "tt-linux-sandbox",
    about = "Run a command inside a TT Linux sandbox"
)]
struct Cli {
    #[arg(long = "request-file", conflicts_with = "request_json")]
    request_file: Option<PathBuf>,
    #[arg(long = "request-json", conflicts_with = "request_file")]
    request_json: Option<String>,
    #[arg(long = "bwrap-path", default_value = "bwrap")]
    bwrap_path: PathBuf,
    #[arg(long = "dry-run", default_value_t = false)]
    dry_run: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let source = if let Some(raw) = cli.request_json {
        RequestSource::Json(raw)
    } else if let Some(path) = cli.request_file {
        RequestSource::File(path)
    } else {
        RequestSource::Stdin
    };

    let request = load_request(source)?;
    if cli.dry_run {
        println!("{}", serde_json::to_string_pretty(&request)?);
        return Ok(ExitCode::from(0));
    }

    let status = run_request(request, &cli.bwrap_path)?;
    Ok(status
        .code()
        .map(|code| ExitCode::from(code as u8))
        .unwrap_or_else(|| ExitCode::from(1)))
}
