# TUI Behavior Contract

This note defines the read-only operator contract for the Orcas TUI.

## Focus Targets

The TUI has three navigable focus targets:

- `threads`
- `workstreams`
- `work_units`

Focus order is cyclic:

1. `threads`
2. `workstreams`
3. `work_units`
4. back to `threads`

The focused panel must display an explicit focus marker in its title.

## Navigation Rules

- `Tab` cycles focus in the order above.
- `j` and `k` only change selection within the currently focused list.
- Moving the focused list must not silently mutate selection in unrelated lists.

## Parent / Child Reconciliation

- `workstream` selection is the parent selection for `work_unit`.
- When the selected workstream changes, the selected work unit is reconciled to the first work unit in that workstream.
- If the selected workstream has no work units, `selected_work_unit_id` becomes `None`.
- If an event or reconnect causes the selected work unit to no longer belong to the selected workstream, the TUI must clear or replace that work-unit selection immediately instead of showing an invalid combination.

## Detail / History Rules

- The selected work unit controls the report/decision detail pane and the history pane.
- Work-unit detail and history are loaded lazily through the existing read-only getter.
- A late-arriving detail response for a non-selected work unit must not overwrite the visible detail/history for the currently selected work unit.

## Reconnect Rules

- Reconnect is snapshot-first.
- On reconnect, the authoritative daemon snapshot replaces client summary state.
- Work-unit detail cache must be cleared on snapshot replacement.
- Selection must be reconciled against the new snapshot before rendering.

## Compact Layout Rules

- Compact layouts may stack panels more aggressively and show fewer rows.
- Compact layouts must still keep panel titles legible.
- Compact layouts must keep the focus marker visible.
- Compact layouts must keep the selected item visible in the focused list.
- Compact layouts must not show detail/history that contradicts the selected work unit.
- The stronger signal-visibility contract is targeted at `120x40`, `100x30`, and `80x24`.
- Below that boundary, the TUI still needs to remain legible and selection-correct, but fewer secondary details may be visible at once.

## Collaboration Display Rules

- `parse_result` and `needs_supervisor_review` are separate signals and must remain separately visible.
- `failed`, `interrupted`, and `lost` must be rendered explicitly.
- Reused `worker_session_id` may be shown, but it must not imply assignment continuity.
