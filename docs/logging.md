# Logging

## Overview

TT uses Rust `tracing` with `tracing_subscriber` and plain-text file logs.

The logging model is intentionally simple:

1. Each binary writes to its own component log file.
2. Runtime verbosity is controlled with `RUST_LOG`.
3. Raw TT app-server stdout/stderr goes to a separate diagnostic log file.

TT no longer maintains a merged aggregate log.

## Runtime Log Level Control

`RUST_LOG` controls the tracing filter for all TT binaries.

If `RUST_LOG` is unset or invalid, TT falls back to:

```text
{component}=info,info,tokio=info
```

Examples:

```bash
RUST_LOG=info ttd
RUST_LOG=debug ttd
RUST_LOG=ttd=debug,tokio=info ttd
RUST_LOG=tt=debug tt doctor
```

TT also supports one logging-related boolean env var:

1. `TT_LOG_RUNTIME_CYCLE` enables extra runtime-cycle logs when set to `1`, `true`, `yes`, or `on`.

## Log Locations

TT keeps logs under its home root.

The logs directory is:

```bash
${TT_HOME:-~/.tt}/logs/
```

Current log files:

1. `ttd.log` for the daemon.
2. `tt.log` for the operator CLI.
3. `tt-app-server.log` for raw TT app-server stdout/stderr diagnostics.

The log, config, data, and runtime directories are created automatically on startup.

## Recommended Debug Workflow

Start with the component log that matches the failing surface:

1. `ttd.log` for daemon startup, IPC, persistence, authority-store, and upstream lifecycle issues.
2. `tt.log` for CLI command issues.

Use `tt-app-server.log` only when the semantic daemon logs point to an upstream TT app-server problem and you need the raw subprocess output.

Good first steps:

```bash
tail -f ~/.tt/logs/ttd.log
RUST_LOG=debug ttd
tt daemon status
tt doctor
```

For targeted debugging, prefer component-specific filters over global `debug`.
