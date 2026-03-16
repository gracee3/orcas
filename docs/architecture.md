# Orcas Architecture

## Goal

Orcas is structured to survive Codex upgrades by isolating app-server specifics behind Orcas-owned boundaries.

## Crate Boundaries

### `orcas-core`

Owns stable local concepts that should not depend on Codex transport details:

- app config and runtime defaults
- Orcas error/result types
- runtime paths
- thread/session metadata persisted by Orcas
- cross-crate event envelope types
- JSON session store abstraction and implementation

### `orcas-codex`

Owns the unstable edge where Orcas talks to Codex:

- daemon launch/status management
- `CodexTransport` abstraction
- WebSocket transport implementation
- JSON-RPC request/response/notification types
- request ID generation
- event pump
- reconnect/backoff loop
- narrow typed protocol structs for the first app-server slice
- approval-routing surface for server -> client requests

### `orcas-supervisor`

Owns the first executable and operator flow:

- `SupervisorService`
- CLI shape
- proof-of-life commands
- local persistence updates after thread actions
- simple event streaming for turn output

### `orcas-tui`

Owns only a placeholder frontend shell in this pass.

The crate exists now so future UI work can depend on stable Orcas interfaces instead of building directly on supervisor code.

## Runtime Model

1. Supervisor loads Orcas config and paths.
2. Supervisor resolves whether to connect or spawn Codex.
3. `LocalCodexDaemonManager` ensures the WebSocket endpoint exists.
4. `CodexClient` owns a long-lived connection loop.
5. Requests go through Orcas JSON-RPC types, not ad hoc JSON scattered through the CLI.
6. Notifications are mapped into Orcas event envelopes and broadcast to subscribers.
7. Supervisor persists thread metadata to Orcas state after thread-affecting operations.

## Event Model

`CodexClient` emits Orcas-owned `EventEnvelope` values that currently cover:

- connection state changes
- thread started/status changed
- turn started/completed
- item started/completed
- agent message delta
- server request surfaced to the approval router
- warning events

This keeps higher layers decoupled from raw app-server notification payloads.

## Persistence Model

Current persistence is intentionally lightweight:

- config: TOML
- state: JSON
- logs: plain text file from spawned Codex app-server

`JsonSessionStore` is behind the `OrcasSessionStore` trait so a future SQLite-backed store can replace it without changing the CLI or transport layer.

## Future Expansion Path

This scaffold is meant to support:

- a separately managed Orcas supervisor process
- a richer TUI using the same event/state model
- a browser bridge/backend
- multi-client event fanout
- approval workflows
- richer thread lifecycle features such as review, rollback, and fork

The intended next architectural move is an Orcas-side service that owns connection state once and fans out events to multiple clients.
