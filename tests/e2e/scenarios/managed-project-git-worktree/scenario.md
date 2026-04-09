# Managed Project Git Worktree

This scenario verifies that managed-project commands work from a linked child git worktree.

It exercises:

- `tt init` from a child worktree path.
- `tt open` from the same child worktree path.
- hidden internal inspection from both the child worktree and the superproject root.
- The daemon’s repo-root resolution for managed-project commands in a worktree checkout.
