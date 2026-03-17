# Orcas CLI and TUI Cheat Sheet

Run these commands from the repo root.

## Build Once

```bash
cargo build -p orcas-supervisor --bin orcas
cargo build -p orcas-tui --bin orcas-tui
```

Then use:

```bash
./target/debug/orcas ...
./target/debug/orcas-tui
```

## `orcas` vs `orcasd`

- `orcas` is the supervisor CLI.
- `orcasd` is the daemon process itself.

Run the daemon directly only if you want it in the foreground:

```bash
cargo run -p orcas-daemon --bin orcasd
```

For normal daemon management, use:

```bash
./target/debug/orcas supervisor ...
```

## Daemon

```bash
./target/debug/orcas supervisor daemon status
./target/debug/orcas supervisor daemon start
./target/debug/orcas supervisor daemon restart
./target/debug/orcas supervisor daemon stop
```

## Health and Models

```bash
./target/debug/orcas supervisor doctor
./target/debug/orcas supervisor models list
```

## Threads

```bash
./target/debug/orcas supervisor threads list
./target/debug/orcas supervisor threads read --thread <thread-id>
./target/debug/orcas supervisor threads start
./target/debug/orcas supervisor threads start --cwd <path> --model <model>
./target/debug/orcas supervisor threads start --ephemeral
./target/debug/orcas supervisor threads resume --thread <thread-id>
./target/debug/orcas supervisor threads resume --thread <thread-id> --cwd <path> --model <model>
```

## Turns

```bash
./target/debug/orcas supervisor turns list-active
./target/debug/orcas supervisor turns get --thread <thread-id> --turn <turn-id>
```

## Workstreams

```bash
./target/debug/orcas supervisor workstreams list
./target/debug/orcas supervisor workstreams get --workstream <workstream-id>
./target/debug/orcas supervisor workstreams create --title "Title" --objective "Goal"
./target/debug/orcas supervisor workstreams create --title "Title" --objective "Goal" --priority high
```

## Workunits

```bash
./target/debug/orcas supervisor workunits list
./target/debug/orcas supervisor workunits list --workstream <workstream-id>
./target/debug/orcas supervisor workunits get --workunit <workunit-id>
./target/debug/orcas supervisor workunits create --workstream <workstream-id> --title "Title" --task "Task"
./target/debug/orcas supervisor workunits create --workstream <workstream-id> --title "Title" --task "Task" --dependency <workunit-id>
```

Repeat `--dependency` to add more than one dependency.

## Assignments

```bash
./target/debug/orcas supervisor assignments get --assignment <assignment-id>
./target/debug/orcas supervisor assignments start --workunit <workunit-id> --worker <worker-id>
./target/debug/orcas supervisor assignments start --workunit <workunit-id> --worker <worker-id> --instructions "Do X"
./target/debug/orcas supervisor assignments start --workunit <workunit-id> --worker <worker-id> --worker-kind codex --cwd <path> --model <model>
```

## Reports

```bash
./target/debug/orcas supervisor reports get --report <report-id>
./target/debug/orcas supervisor reports list-for-workunit --workunit <workunit-id>
```

## Decisions

```bash
./target/debug/orcas supervisor decisions apply --workunit <workunit-id> --type continue --rationale "Reason"
./target/debug/orcas supervisor decisions apply --workunit <workunit-id> --report <report-id> --type mark-complete --rationale "Done"
./target/debug/orcas supervisor decisions apply --workunit <workunit-id> --type redirect --rationale "Reason" --worker <worker-id> --worker-kind codex --instructions "Next step"
```

Valid `--type` values:

```text
accept
continue
redirect
mark-complete
escalate-to-human
```

## Proposals

```bash
./target/debug/orcas supervisor proposals get --proposal <proposal-id>
./target/debug/orcas supervisor proposals list-for-workunit --workunit <workunit-id>
./target/debug/orcas supervisor proposals create --workunit <workunit-id>
./target/debug/orcas supervisor proposals create --workunit <workunit-id> --report <report-id> --note "Review this" --requested-by emmy
./target/debug/orcas supervisor proposals create --workunit <workunit-id> --supersede-open
./target/debug/orcas supervisor proposals approve --proposal <proposal-id>
./target/debug/orcas supervisor proposals approve --proposal <proposal-id> --reviewed-by emmy --review-note "Looks right"
./target/debug/orcas supervisor proposals approve --proposal <proposal-id> --type redirect --rationale "Follow-up needed" --worker <worker-id> --worker-kind codex --instruction "Next step"
./target/debug/orcas supervisor proposals reject --proposal <proposal-id> --reviewed-by emmy --review-note "Needs revision"
```

`proposals approve` also supports repeated fields:

```bash
./target/debug/orcas supervisor proposals approve \
  --proposal <proposal-id> \
  --type continue \
  --instruction "Step 1" \
  --instruction "Step 2" \
  --acceptance "Condition 1" \
  --stop-condition "Stop if blocked" \
  --expected-report-field summary
```

## Prompt and Quickstart

These are CLI flows, not TUI flows.

```bash
./target/debug/orcas supervisor prompt --thread <thread-id> --text "Hello"
./target/debug/orcas supervisor quickstart --text "Do the thing"
./target/debug/orcas supervisor quickstart --cwd <path> --model <model> --text "Do the thing"
```

## Global Flags

These go before `supervisor`:

```bash
./target/debug/orcas --codex-bin <path> --listen-url <url> --cwd <path> --model <model> supervisor ...
./target/debug/orcas --connect-only supervisor ...
./target/debug/orcas --force-spawn supervisor daemon start
```

## TUI

Start the TUI:

```bash
./target/debug/orcas-tui
```

Or:

```bash
cargo run -p orcas-tui --bin orcas-tui
```

The TUI is currently a read-only operator console:

- no prompt box
- no prompt submission
- top-level views: `Overview`, `Threads`, `Collaboration`

### TUI Keys

```text
q            quit
r            refresh
?            toggle help

1            show Overview
2            show Threads
3            show Collaboration
Tab          cycle top-level view

j / Down     next selection in current view
k / Up       previous selection in current view

h / Left     switch collaboration focus
l / Right    switch collaboration focus
```

### TUI Interaction Notes

- The TUI starts on `Overview`.
- In `Threads`, `j` and `k` move thread selection.
- In `Collaboration`, `h` and `l` both switch focus between `Workstreams` and `Work Units`.
- In `Collaboration`, `j` and `k` move selection in the currently focused list.
- If the daemon is unavailable, expect reconnect status rather than a usable data view until it comes back.

## Help

```bash
./target/debug/orcas --help
./target/debug/orcas supervisor --help
./target/debug/orcas supervisor daemon --help
./target/debug/orcas supervisor threads --help
./target/debug/orcas supervisor turns --help
./target/debug/orcas supervisor workstreams --help
./target/debug/orcas supervisor workunits --help
./target/debug/orcas supervisor assignments --help
./target/debug/orcas supervisor reports --help
./target/debug/orcas supervisor decisions --help
./target/debug/orcas supervisor proposals --help
./target/debug/orcas supervisor prompt --help
./target/debug/orcas supervisor quickstart --help
```
