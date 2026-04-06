# TT Skills Todo

Working notes for the TT skill cluster:
`agent`, `i3`, `tt`, `process`, `services`, and `git`.

## Goal

Clarify how these skills should divide responsibilities, how they hand off work,
and what the operator-facing conventions should be.

## Discussion Topics

- `agent`
  - Decide when to spawn a subagent versus handling work directly.
  - Define the minimum handoff payload for spawned work.
  - Clarify how agent threads report progress back to the parent lane.
- `i3`
  - Define the canonical workspace/window naming scheme.
  - Decide which i3 actions are safe for autonomous execution.
  - Clarify what state should be mirrored in the TT runtime versus left local.
- `tt`
  - Specify the boundaries of session lifecycle ownership.
  - Decide how shared app-server coordination should be described to operators.
  - Define what counts as a recoverable versus unrecoverable TT runtime failure.
- `process`
  - Enumerate the supported process lifecycle states.
  - Decide how long-lived background processes should be tracked and retired.
  - Clarify what metadata is required before a process may be restarted.
- `services`
  - Define the lifecycle contract for managed services and daemons.
  - Decide what “healthy”, “degraded”, and “stopped” should mean in TT terms.
  - Clarify how service management differs from generic process control.
- `git`
  - Define branch/worktree naming conventions for TT work.
  - Decide when a branch should be created versus using an existing worktree.
  - Clarify the commit boundary for each logical TT change.

## Open Questions

- Should `agent` be the only skill allowed to delegate to other skills?
- Should `process` and `services` share a single lifecycle vocabulary?
- Should `tt` own the app-server contract exclusively, or should `services`
  share some of that surface area?
- What is the smallest set of commands each skill must expose for the operator
  flow to feel complete?

## Next Steps

1. Trim this list down to the decisions that block current work.
2. Turn each blocking item into a concrete implementation task.
3. Move any resolved notes into the relevant `SKILL.md` files.
