#![allow(unused_crate_dependencies)]

use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use orcas_core::StoredState;

#[derive(Debug, Parser)]
struct Args {
    /// Seed or mutated state.json input to normalize through StoredState.
    #[arg(long)]
    input: PathBuf,
    /// Destination file. Defaults to in-place rewrite of --input.
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let raw = fs::read_to_string(&args.input)
        .with_context(|| format!("read {}", args.input.display()))?;
    let state = StoredState::from_json_str(&raw)
        .with_context(|| format!("parse {}", args.input.display()))?;
    let mut encoded = state
        .to_pretty_json()
        .with_context(|| format!("serialize {}", args.input.display()))?;
    encoded.push('\n');
    let output = args.output.unwrap_or_else(|| args.input.clone());
    fs::write(&output, encoded).with_context(|| format!("write {}", output.display()))?;
    Ok(())
}
