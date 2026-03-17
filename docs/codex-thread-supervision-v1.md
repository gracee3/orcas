# Codex Thread Supervision v1

This slice adds Orcas-side mirror and monitoring support for Codex app-server threads.
It now also includes Orcas-native assignment metadata for binding a Codex thread to Orcas workflow objects.

## Canonical Boundaries

Codex app-server remains canonical for:

- thread existence
- loaded/runtime status
- turn lifecycle
- item/event delivery
- actual turn execution

Orcas remains canonical for:

- persisted mirror state used by Orcas clients
- monitor attachment intent and attachment status shown to operators
- thread-to-workflow assignment metadata
- any future supervisor decision and approval workflow

This slice does not store Orcas workflow metadata inside Codex thread history.

## Supported v1 Behavior

- discover existing Codex threads, including externally created and headless threads
- read persisted thread history from app-server and persist the normalized Orcas mirror
- best-effort attach a thread for future live monitoring using documented `thread/resume` behavior
- detach a thread in Orcas so the operator surface reports `history only`
- assign a Codex thread to an Orcas workstream, work unit, and supervisor
- pause, resume, and release Orcas-native Codex-thread assignments
- persist thread mirror state and turn state across Orcas restart
- persist Codex-thread assignment state across Orcas restart
- show in the TUI:
  - loaded status
  - live attach status
  - assignment badge / assignment panel
  - persisted turn history
  - aggregated item text
  - turn lifecycle snapshots
  - source kind when app-server exposes it

## Monitor Semantics

Thread monitor state is explicit:

- `detached`: Orcas has history or discovery data only
- `attaching`: Orcas is attempting best-effort live attachment
- `attached`: Orcas requested live monitoring and is ingesting future app-server events
- `errored`: Orcas could not establish best-effort live monitoring

`detached` does not mean the thread is dead. It only means Orcas is not claiming an active live-monitor attachment for that thread.

## Persistence Semantics

Orcas persists:

- normalized thread summaries
- persisted turn/item history already known to Orcas
- turn lifecycle snapshots used by operator views

Orcas does not attempt to reconstruct transient deltas it never observed.

## Assignment Semantics

Codex-thread assignment is an Orcas-native object.

- It binds `codex_thread_id` to `workstream_id`, `work_unit_id`, and `supervisor_id`.
- It is persisted in Orcas collaboration state, not in Codex thread history.
- A thread can still be monitored whether assigned or unassigned.
- Creating an assignment never sends a turn.
- Creating an assignment for an active thread never interrupts, steers, or queues a send.

The daemon enforces:

- at most one active assignment per `codex_thread_id`
- paused assignments are not active
- released assignments are not active
- released assignments remain queryable for audit/history

Current assignment lifecycle operations:

- `create`
- `get`
- `list`
- `pause`
- `resume`
- `release`

## Non-goals In This Slice

- PTY attach or PTY replay
- exact replay of already-missed transient deltas
- whole-thread kill semantics
- process-tree kill semantics
- supervisor decision generation
- approve/reject/send workflow
- automatic bootstrap proposal generation
- steer/interrupt proposal workflow
- automatic supervisor writing into Codex threads
- mutation of app-server’s persisted thread log format
- full assignment / approval / next-turn supervision workflow

## IPC Surface Added

- `threads/list_loaded`
- `thread/read_history`
- `thread/attach`
- `thread/detach`

Existing `thread/start`, `thread/read`, `thread/get`, `thread/resume`, `turn/start`, and `turn/interrupt` remain in place.

Assignment IPC added:

- `codex_assignment/create`
- `codex_assignment/get`
- `codex_assignment/list`
- `codex_assignment/pause`
- `codex_assignment/resume`
- `codex_assignment/release`
