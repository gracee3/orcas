# Next Milestones

- Keep the TT contract snapshot in `crates/tt-runtime/contracts/tt-contract-index.json` aligned with the local TT checkout.
- Add any future TT-facing typed fields to `tt-runtime::contract` before wiring them into runtime code.
- When TT updates land upstream, regenerate the snapshot with `cargo run -p tt-runtime --bin tt-contract-sync -- --out crates/tt-runtime/contracts/tt-contract-index.json` and run the contract drift test.
