# Managed Project Thread Control

This scenario verifies the per-thread control loop for a managed project:

- hidden internal spawn creates live managed threads that the operator can watch in Codex TUI
- hidden internal control can mark a worker role `manual_next_turn`
- hidden internal inspect shows the updated per-thread control mode immediately
- switching the same role back to `director` restores automatic control

The demo keeps the thread lifecycle short on purpose. It does not wait for the
full multi-round `taskflow` project to finish; it only proves that TT can:

1. create a live managed thread
2. mark that thread as manually controlled for the next turn
3. show that state in inspection output
4. switch the thread back to director control
