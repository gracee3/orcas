# Configuration

## Overview

TT uses a user-level TOML config file at `~/.tt/config.toml` for persistent defaults.

The runtime model on this branch is:

1. one host/home `ttd`
2. one shared TT app-server per host/home
3. `ttd` attaches to that app-server and does not spawn it
4. `tt app-server ...` is the recommended lifecycle surface

Use the config file for durable defaults, environment variables for per-process overrides, and CLI flags for one-off operator sessions.

## Recommended Shared Runtime Example

This is the primary host/home setup. It uses an TT-managed shared app-server and leaves direct TT/OpenAI auth to the host environment unless you explicitly override it.

```toml
[tt]
binary_path = "/path/to/tt"
connection_mode = "connect_only"
config_overrides = []

[tt.reconnect]
initial_delay_ms = 150
max_delay_ms = 5000
multiplier = 2.0

[tt.app_server.default]
enabled = true
owner = "tt"
transport = "websocket"
listen_url = "ws://127.0.0.1:4500"

[tt.responses]
base_url = "https://api.openai.com/v1"

[tt.direct_api]
# auth_file = "~/.tt/auth.json"

[supervisor]
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
model = "gpt-5.4"
reasoning_effort = "high"
max_output_tokens = 2000

[supervisor.proposals]
auto_create_on_report_recorded = false

[defaults]
# worktree_root = "/path/to/worktrees/tt"
model = "gpt-5"
```

Operationally:

1. `tt app-server add default` ensures the shared app-server definition exists.
2. `tt app-server start default` launches the shared listener.
3. `tt daemon start` starts `ttd`, which connects to the configured app-server endpoint.
4. `tt app-server status default` and `tt app-server info default` show the shared runtime endpoint and listener details.

TT also refreshes the checked-in repo `.tt/` template into the shared app-server `RUNTIME_HOME` when the app-server is added or started. In practice that means:

- source template: `./.tt/`
- runtime target: `~/.tt/app-server/default/tt-home/.tt`

That gives the shared runtime a managed `config.toml` and lane-agent defaults without turning the pack into per-workstream state.

## Local Provider Example

Profiles and provider definitions let you keep the shared app-server model while selecting a different model backend for specific roles or workstreams.

```toml
[tt.profiles.local]
model_provider = "vllm"
model = "local-model"

[tt.model_providers.vllm]
name = "vLLM"
base_url = "http://127.0.0.1:8000/v1"
wire_api = "responses"
```

This example is additive. Keep the shared app-server configuration above and layer local provider selection through role, workstream, or CLI overrides.

## Auth Behavior

The default documented path is host auth:

1. TT uses the host’s existing TT/OpenAI auth state unless you explicitly override it.
2. The main shared-runtime example leaves `tt.direct_api.auth_file` unset.
3. If you need an explicit file override, set `tt.direct_api.auth_file` yourself.

TT does not require a dedicated auth file in the primary recommended setup.

## Public Config Shape

The public nested `tt` shape on this branch is:

1. `[tt]`
2. `[tt.reconnect]`
3. `[tt.app_server.default]`
4. `[tt.responses]`
5. `[tt.direct_api]`
6. `[tt.profiles.<name>]`
7. `[tt.model_providers.<name>]`

Important fields are:

1. `tt.binary_path`
2. `tt.connection_mode`
3. `tt.config_overrides`
4. `tt.app_server.default.owner`
5. `tt.app_server.default.transport`
6. `tt.app_server.default.listen_url`
7. `tt.responses.base_url`
8. `tt.direct_api.auth_file`
9. `defaults.cwd`
10. `defaults.worktree_root`
11. `defaults.model`

The generated default config follows this shape.

## Environment Variables

### Logging

`RUST_LOG` controls the tracing filter for all binaries.

Examples:

```bash
RUST_LOG=info ttd
RUST_LOG=ttd=debug,tokio=info ttd
RUST_LOG=tt=debug tt doctor
```

`TT_LOG_RUNTIME_CYCLE` enables runtime-cycle logging when set to `1`, `true`, `yes`, or `on`.
`TT_DEFAULT_WORKTREE_ROOT` overrides the default worktree root for workstream and TT spawn commands.

### Runtime Overrides

The daemon and CLI process manager recognize:

1. `TT_RUNTIME_BIN`
2. `TT_RUNTIME_LISTEN_URL`
3. `TT_DEFAULT_CWD`
4. `TT_DEFAULT_WORKTREE_ROOT`
5. `TT_DEFAULT_MODEL`
6. `TT_CONNECTION_MODE`
7. `TT_DAEMON_BINARY_PATH`
8. `TT_DAEMON_BUILD_FINGERPRINT`

`TT_CONNECTION_MODE` is still available as a process override, but the documented shared-runtime configuration uses `connect_only`.

If you override the listen URL, keep the CLI, daemon, and shared app-server pointed at the same endpoint.

## Logging And Paths

TT writes structured logs under:

```bash
~/.tt/logs/
```

The main files are:

1. `ttd.log`
2. `tt.log`
3. `app-server-default.log` for the TT-managed shared app-server

The daemon socket lives under:

```bash
${TT_HOME:-~/.tt}/runtime/ttd.sock
```

`ttd` and the shared app-server both use the same host/home root unless you explicitly change `TT_HOME`.

## Role Pack

The repo includes a checked-in `.tt/` scaffold that acts as the template for the shared app-server home, and TT copies it into `~/.tt/app-server/default/tt-home/.tt`.

TT copies the `.tt` subtree from that pack into the shared app-server `RUNTIME_HOME`:

- source template: `.tt/`
- runtime target: `~/.tt/app-server/default/tt-home/.tt`

The runtime target is managed by TT and refreshed on `tt app-server add` and `tt app-server start`.
