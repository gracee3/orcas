# Testing

## Overview

Testing in TT is intentionally layered. Most logic should be covered at the lightest layer that gives confidence, with heavier real-daemon and real-CLI tests reserved for the workflows that matter most at process and operator boundaries.

The current test stack covers:
- fast unit and contract tests
- client and protocol boundary tests
- integration tests against a spawned real `ttd`
- bounded real CLI and operator workflow tests
- regression tests for previously tricky behavior

## Test Layers

### Fast Unit And Contract Tests

Use these for pure logic, serde contracts, parsing, rendering, validation, defaults, and backward-compat behavior.

Typical examples:
- `ttd/src/assignment_comm/*`
- `ttd/src/supervisor.rs`
- `tt-core` shared schema modules
- `tt-runtime/src/protocol/types.rs`

### Integration Tests

Use these when confidence depends on a real boundary but not a full end-to-end campaign.

Typical examples:
- direct client-boundary tests for `TTClient` and `TTIpcClient`
- real Unix-socket tests against a spawned daemon
- persistence and reconnect coverage across restart

### Behavior-Contract Tests

These verify durable external behavior without asserting every internal detail.

Typical examples:
- CLI create/read flows
- proposal and review visibility/action flows
- daemon event and state projection contracts

### Bounded End-To-End Tests

Use real-daemon and real-CLI tests only for the highest-value workflows:
- daemon lifecycle
- authority persistence and projection
- assignment/report/proposal/review operator paths

Keep these scenarios small, deterministic, and focused.

### In-Repo E2E Harness

The checked-in E2E harness lives under `tests/e2e/` and is intentionally opt-in.

Use it for scenario-style operator workflows that need real CLI/daemon behavior but should not run as part of `cargo test` or `make test`.

The harness writes generated state only under `target/e2e/`, which keeps scenario artifacts easy to inspect and easy to remove.

Current lane contract:

- `make test-e2e` is the daily deterministic lane and should stay usable from a normal dirty developer checkout
- clean-git scenarios are opt-in, not default
- the default deterministic lane remains model-free
- proposal-bearing live supervisor scenarios may use an explicit local OpenAI-compatible endpoint, but only as opt-in test scaffolding
- seeded-state proposal scenarios should remain model-free

The main entrypoints are:

- `make test-e2e`
- `make test-e2e-live`
- `make test-e2e-long`
- `make test-e2e SCENARIO=<name>`
- `make test-e2e TAG=<tag>`
- `make clean-e2e`

### Regression Tests

When a bug or ambiguity is found, add a targeted regression test close to the seam that failed. Prefer a small local test over expanding a large matrix.

## Important Harnesses And Seams

High-value existing patterns:
- direct protocol/client tests in `tt-runtime` and `ttd`
- spawned-daemon harness in `crates/ttd/tests/harness.rs`
- fake TT helper for bounded upstream behavior
- fake supervisor `/responses` helper for deterministic proposal generation
- CLI integration tests in `crates/tt/tests/cli_socket.rs`

The TT contract inventory is checked in under `crates/tt-runtime/contracts/` and is regenerated with:

```bash
cargo run -p tt-runtime --bin tt-contract-sync -- \
  --out crates/tt-runtime/contracts/tt-contract-index.json
```

The matching drift test is `contract::tests::contract_index_matches_current_tt_checkout` in `tt-runtime`.

These are enough for most future additions. Prefer extending an existing harness over creating a new one.

## Standard Commands

Run the full suite:

```bash
cargo test --workspace
```

Run the fast developer path:

```bash
make test
```

Run the E2E harness:

```bash
make test-e2e
make test-e2e-live
make test-e2e-long
make test-e2e SCENARIO=hello
make test-e2e TAG=deterministic
make clean-e2e
```

Useful focused examples:

```bash
cargo test -p ttd --lib
cargo test -p ttd --test real_socket -- --nocapture
cargo test -p tt --test cli_socket -- --nocapture
cargo test -p tt-runtime --lib
cargo test -p ttd parse_worker_report_recovers_when_live_worker_corrupts_identity_line -- --nocapture
cargo test -p ttd assignment_start_refreshes_persisted_packet_when_cwd_changes -- --nocapture
```

Coverage:

```bash
cargo llvm-cov --workspace --summary-only
cargo llvm-cov --workspace --html
```

If you only need the lighter shared-library view:

```bash
cargo llvm-cov --workspace --lib --summary-only
```

## Guidance For Adding New Tests

- Prefer the lightest layer that gives confidence.
- Add real-daemon or real-CLI tests only for workflows whose value comes from the real boundary.
- Add regression tests for discovered bugs or tricky edge cases.
- Avoid brittle assertions when exact text is not the contract.
- For schema and protocol work, test omission/default/tag behavior directly.
- For workflow tests, assert stable operator-visible fields rather than incidental formatting.

## Current Status

Recent testing work substantially improved:
- Layer 1 policy and schema coverage
- client/protocol boundary coverage
- real daemon/socket integration coverage
- real CLI and operator workflow coverage
- recovery of malformed live worker report envelopes
- preservation of assignment execution context across redirected or successor turn ingestion
- reliability of the default deterministic E2E lane from a normal dirty checkout

The workspace is in a good stopping state. Future test work should be selective and driven by new features, regressions, or specific cold spots rather than broad expansion.
