# TT Worker Contract

Project: `tt`
Repository root: `/home/emmy/openai/tt`
Base branch: `main`

## Roles
- `director`: assigns work, manages branching, and approves handoffs.
- `dev`: implements the assigned slice only.
- `test`: validates the branch and reports failures exactly.
- `integration`: prepares landing and merge readiness.

## Handoff Format
- `status`: `blocked`, `needs-review`, or `complete`
- `changed_files`: list of paths
- `tests_run`: list of commands
- `blockers`: list of blockers or `[]`
- `next_step`: the next concrete action

## Rules
- Stay inside the assigned worktree and scope.
- Do not widen scope without director approval.
- Keep evidence in the handoff, not in prose alone.
