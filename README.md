# Orcas

Orcas is a local Rust scaffold for orchestrating Codex `app-server` over WebSocket.

The first pass is intentionally narrow:

- long-lived local Codex daemon model
- Orcas-owned transport and JSON-RPC boundary
- small supervisor CLI for proof-of-life
- placeholder TUI crate for future frontend separation
- lightweight local config and thread persistence

## Why WebSocket

Orcas targets `codex app-server --listen ws://127.0.0.1:PORT` so one Codex daemon can support multiple independent clients later.

Codex is treated as an external process. Orcas does not depend on Codex internal crates as a stable SDK.

## Workspace

- `crates/orcas-core`: shared config, errors, events, session metadata, persistence
- `crates/orcas-codex`: daemon management, WebSocket transport, JSON-RPC client, typed protocol slice
- `crates/orcas-supervisor`: `orcas` CLI and proof-of-life flow
- `crates/orcas-tui`: placeholder ratatui shell
- `docs/architecture.md`: crate boundaries and runtime model
- `docs/codex-app-server-notes.md`: protocol notes from the local Codex checkout

## Pinned Local Codex Path

Default Orcas config pins the local debug build path:

- `/home/emmy/git/codex/codex-rs/target/debug/codex`

Build it if needed:

```bash
cd /home/emmy/git/codex/codex-rs
cargo build -p codex-cli --bin codex
```

## Build

```bash
cd /home/emmy/git/orcas
cargo check
cargo test
```

## Config And State

- config: `~/.config/orcas/config.toml`
- state: `~/.local/share/orcas/state.json`
- logs: `~/.local/share/orcas/logs/codex-app-server.log`

## Proof Of Life

```bash
cd /home/emmy/git/orcas
cargo run -p orcas-supervisor -- supervisor doctor
cargo run -p orcas-supervisor -- supervisor daemon start
cargo run -p orcas-supervisor -- supervisor models list
```

Start a thread:

```bash
cargo run -p orcas-supervisor -- supervisor threads start \
  --cwd /home/emmy/git/orcas \
  --model gpt-5.4
```

Send a prompt to an existing thread:

```bash
cargo run -p orcas-supervisor -- supervisor prompt \
  --thread <THREAD_ID> \
  --text "Summarize this repo in two bullets."
```

End-to-end quickstart:

```bash
cargo run -p orcas-supervisor -- supervisor quickstart \
  --cwd /home/emmy/git/orcas \
  --model gpt-5.4 \
  --text "Say hello in one sentence."
```

Launch the placeholder TUI:

```bash
cargo run -p orcas-tui
```

## Current Scope

Implemented now:

- `initialize` handshake
- `thread/start`, `thread/resume`, `thread/read`, `thread/list`
- `turn/start`, `turn/interrupt`
- `model/list`
- event streaming for:
  - `thread/started`
  - `thread/status/changed`
  - `turn/started`
  - `turn/completed`
  - `item/started`
  - `item/completed`
  - `item/agentMessage/delta`

## Limitations

- WebSocket only for now
- approvals are surfaced and rejected by default
- protocol layer is intentionally narrow
- `threads list` currently shows the raw server-side thread set

See [docs/architecture.md](docs/architecture.md) and [docs/codex-app-server-notes.md](docs/codex-app-server-notes.md) for the detailed notes.
