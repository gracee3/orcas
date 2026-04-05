# Operations

## Starting The System

Start the daemon directly when you want to run it in the foreground.

```bash
ttd
```

Use the systemd user manager when you want the daemon managed as a service that shares the same XDG paths as the CLI.

```bash
systemctl --user start tt-daemon.service
systemctl --user enable tt-daemon.service
```

The TT CLI can also request that the daemon be started on demand.

```bash
tt daemon start
```

## Checking Status

Check the unit state with the user manager.

```bash
systemctl --user status tt-daemon.service
```

Check TT-level daemon state through the CLI.

```bash
tt daemon status
tt doctor
```

The doctor command reports the config path, `state.db`, runtime directory, socket path, daemon log path, and current TT endpoint.

The current checked-in operator surface is CLI-first:

- `ttd` is the durable local daemon and IPC boundary
- `tt` is the strongest checked-in operator surface
- `tt tui` is the checked-in supervisor-backed dashboard wrapper for launching and resuming live upstream TT TUI sessions

The dashboard wrapper can be closed independently of the TT child sessions it launches. If you resume a thread into the TT TUI, that child process stays separate and keeps running until you terminate it directly. On startup, the dashboard attaches to the supervisor lane rooted at `~/.tt` and then renders the border HUD with an explicit shortcut legend on its own line. It no longer renders workstream/thread sidebars, fades the HUD in and out on `F2`, and keeps tab switching available for live sessions.

## Logs

Use `journalctl --user` for unit lifecycle events and startup failures.

```bash
journalctl --user -u tt-daemon.service -e
journalctl --user -u tt-daemon.service -f
```

Use the file logs for the application’s own tracing output.

```bash
tail -f ~/.tt/logs/ttd.log
tail -f ~/.tt/logs/tt.log
```

Common log patterns include socket bind failures, stale runtime cleanup, upstream connection failures, and request validation errors. If a client cannot connect, check the daemon log first, then confirm the socket path exists and is responsive. Use `tt-app-server.log` only when you need raw upstream subprocess output.

## Restarting And Stopping

```bash
systemctl --user restart tt-daemon.service
systemctl --user stop tt-daemon.service
```

The CLI exposes the same operations through the daemon API.

```bash
tt daemon restart
tt daemon stop
```

## Common Issues

### Daemon Not Starting

Check the daemon log and the unit status. The usual causes are a bad TT binary path, a missing runtime directory, or a failure to bind the local socket.

```bash
systemctl --user status tt-daemon.service
tail -n 100 ~/.tt/logs/ttd.log
```

### Socket Conflict Or Stale Socket

TT uses a Unix socket, not a TCP port. If another process already owns the socket path, or if a stale socket file remains after a crash, the daemon cannot bind cleanly. Stop the old process or remove the stale runtime artifacts after confirming nothing is still running.

```bash
tt daemon status
systemctl --user stop tt-daemon.service
```

### Permission Issues

If the daemon cannot create its config, data, log, or runtime directories, check the ownership of your user-scoped XDG paths and whether the user service inherited the expected environment.

### Binary Not Found In `PATH`

If `tt` or `ttd` are not found, install them into a directory on your `PATH` or invoke them with an absolute path.

```bash
install -m 0755 bin/tt ~/.local/bin/tt
install -m 0755 bin/ttd ~/.local/bin/ttd
```

## Debugging Workflow

When something fails, isolate the daemon from the operator CLI.

1. Run the daemon in the foreground and watch its log file.
2. Increase verbosity with `RUST_LOG=debug`.
3. Check whether the CLI can connect to the local socket.
4. Verify whether the upstream TT endpoint is reachable from the daemon.

Example:

```bash
RUST_LOG=debug ttd
tt daemon status
tt doctor
```

If the CLI can talk to the daemon but the daemon reports an upstream failure, the problem is usually in the TT endpoint or the local TT binary path. If the CLI cannot reach the daemon at all, focus on the socket path, unit state, and daemon log first.

## Upgrade Considerations

Replacing TT binaries is normally a file swap followed by a daemon restart. Keep the config and state directories in place so the daemon can reuse the existing workflow state.

```bash
systemctl --user stop tt-daemon.service
sudo install -m 0755 ./ttd /usr/local/bin/ttd
sudo install -m 0755 ./tt /usr/local/bin/tt
systemctl --user start tt-daemon.service
```

If the unit file changed, reload systemd before restarting.

```bash
systemctl --user daemon-reload
systemctl --user restart tt-daemon.service
```

## Repo Hygiene

For repository maintenance around TT itself, prefer bounded integration branches and disposable temporary worktrees over long-lived ad hoc merge state.

- land validated repair stacks onto `main` in small bounded batches
- remove temporary integration worktrees after the merge is validated
- keep active dirty feature lanes intentionally preserved until they are merged, archived, or explicitly discarded
- do not assume an unmerged dirty branch is stale just because `main` now contains equivalent cherry-picked fixes
