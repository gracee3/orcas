# Phased Fibonacci

This scenario verifies a real multi-phase tracked-thread workflow on one live Codex lane.

## What Is Seeded

1. The harness creates an isolated git repository and dedicated worktree lane under `target/e2e/worktrees/...`.
2. The harness commits `plan.md` into that worktree before execution begins.
3. The workstream, work unit, and tracked-thread workspace contract are created before the first live assignment.
4. The harness exports `plan.md` plus per-phase operator, supervisor, and agent prompts under `target/e2e/artifacts/...`.
5. The harness inspects the workstream runtime before execution and expects zero active managed threads.

## What Is Live

- Phase 1 is a real scoping/report turn on the declared tracked-thread worktree lane.
- Phase 2 is a real implementation turn that builds the Fibonacci CLI core on that same lane.
- Phase 3 is a real completion turn that adds repeatable tests and final polish on that same lane.
- The phase gates use real operator `Continue` and `MarkComplete` decisions rather than seeded proposal state.
- Orcas binds the tracked-thread automatically on the first live assignment and keeps the same lane attached across all three phases.

## Pass Conditions

- The tracked-thread workspace contract exists before phase 1.
- The workstream runtime exists before execution with zero managed lane threads.
- Phase 1 reports without editing files and binds the tracked thread automatically.
- The runtime shows exactly one managed lane thread and no unmanaged external thread leak after phase 1.
- Phase 2 produces a buildable Fibonacci CLI with `main.c`, `fib.c`, `fib.h`, and `Makefile`.
- Phase 3 adds repeatable tests and leaves `make test` passing in the declared worktree.
- The same tracked-thread lane and upstream thread stay attached across all three phases.
- Two `Continue` decisions reopen the same work unit for the next bounded phase.
- A final `MarkComplete` decision closes the work unit after the finished project is on disk.

## Fail Conditions

- The tracked-thread workspace contract is missing or mismatched.
- The first live assignment does not auto-bind the lane.
- The runtime shows unmanaged thread leakage or loses lane continuity between phases.
- A phase edits files outside the declared worktree or drifts beyond its bounded objective.
- The final Fibonacci project is missing required files or `make test` fails.
- Final persisted work-unit or tracked-thread state contradicts the executed lane.
