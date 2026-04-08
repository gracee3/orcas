# Next Milestones

- The daemon socket transport is live. The next step is to deepen the request surface into thread/workspace orchestration and richer workspace actions.
- The TUI now talks through the daemon helper; the next target is more ergonomic selection/drill-down and workspace actions in the interactive shell.
- `tt-git` owns repo/worktree discovery and merge-readiness inspection; keep using it as the source of truth for landing policy and cleanup decisions.
- Keep the TT contract snapshot in `crates/tt-runtime/contracts/tt-contract-index.json` aligned with the local TT checkout.
- Add any future TT-facing typed fields to `tt-runtime::contract` before wiring them into runtime code.
- When TT updates land upstream, regenerate the snapshot with `cargo run -p tt-runtime --bin tt-contract-sync -- --out crates/tt-runtime/contracts/tt-contract-index.json` and run the contract drift test.
