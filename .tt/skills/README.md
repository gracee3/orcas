# TT Skills Index

This directory contains the TT skill set used by the `.tt` role pack.
Each skill lives in its own subdirectory as a `SKILL.md` file.

## Skills

- `agent`: subagent spawning and lane coordination.
- `chat`: human-facing conversation, summary, and handoff.
- `clean`: cleanup, hygiene, and stale-artifact removal.
- `direct`: main TT playbook for operator intake, todo tracking, and capability dispatch.
- `doctor`: diagnosis and failure localization.
- `git`: branch, worktree, and repo-state coordination.
- `human`: operator liaison and requirement clarification.
- `i3`: window-manager coordination and workspace control.
- `learn`: repo recon, reading, and gap-finding.
- `process`: process lifecycle management and long-lived process state.
- `propose`: decision drafting and structured recommendations.
- `runtime`: TT session and shared app-server coordination.
- `services`: background service lifecycle coordination.
- `test`: validation, harness, and evidence capture.

## Changelog

- `2026-04-05`: renamed the role pack from Orcas/Codex to TT and moved the skill tree under `.tt/skills/`.
- `2026-04-05`: renamed the `codex` skill to `runtime` and updated the direct skill map to reference TT terminology.
- `2026-04-05`: kept the skill layout flat and file-based so the index stays easy to scan and update.
