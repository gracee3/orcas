# Codex role pack for Orcas

Included:
- project-scoped Codex custom-agent files under `.codex/agents/`
- matching plain role-instruction files for Orcas direct injection
- `docs/roles.md` with usage notes and developer-instructions guidance

Recommended startup pattern for all four lanes:
1. set the lane's `developer_instructions`
2. send `ack`
3. expect `understood, please proceed with input`
4. send the real task
