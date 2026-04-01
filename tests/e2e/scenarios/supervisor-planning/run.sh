#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/../../lib" && pwd)/common.sh"
scenario_dir="$(cd "$(dirname "$0")" && pwd)"
e2e_load_scenario_metadata "$scenario_dir"
e2e_prepare_scenario_dirs "$NAME"
e2e_use_short_xdg_paths "sp"
e2e_require_local_supervisor_endpoint

supervisor_base_url="${ORCAS_SUPERVISOR_BASE_URL}"
supervisor_model="${ORCAS_SUPERVISOR_MODEL}"
supervisor_api_key_env="${ORCAS_SUPERVISOR_API_KEY_ENV:-}"
supervisor_reasoning_effort="${ORCAS_SUPERVISOR_REASONING_EFFORT:-}"
supervisor_max_output_tokens="${ORCAS_SUPERVISOR_MAX_OUTPUT_TOKENS:-16384}"
supervisor_temperature="${ORCAS_SUPERVISOR_TEMPERATURE:-0.0}"

cat >"$E2E_SCENARIO_XDG_CONFIG_HOME/orcas/config.toml" <<EOF
[codex]
binary_path = "/home/emmy/git/codex/codex-rs/target/debug/codex"
listen_url = "ws://127.0.0.1:4500"
connection_mode = "spawn_if_needed"
config_overrides = []

[codex.reconnect]
initial_delay_ms = 150
max_delay_ms = 5000
multiplier = 2.0

[supervisor]
base_url = "$supervisor_base_url"
api_key_env = "$supervisor_api_key_env"
model = "$supervisor_model"
reasoning_effort = "$supervisor_reasoning_effort"
temperature = $supervisor_temperature
max_output_tokens = $supervisor_max_output_tokens

[supervisor.proposals]
auto_create_on_report_recorded = false
EOF

daemon_log="$E2E_SCENARIO_LOGS_DIR/orcasd.log"
e2e_orcasd --connect-only >"$daemon_log" 2>&1 &
daemon_pid=$!
cleanup() {
  kill "$daemon_pid" >/dev/null 2>&1 || true
}
trap cleanup EXIT

sleep 5

workstream_output="$(e2e_orcas workstreams create --title "E2E Planning" --objective "Validate supervisor planning flow" --priority normal)"
workstream_id="$(printf '%s\n' "$workstream_output" | awk -F': ' '/^workstream_id:/ {print $2; exit}')"

workunit_output="$(e2e_orcas workunits create --workstream "$workstream_id" --title "Planning work unit" --task "Draft a short implementation plan before execution")"
workunit_id="$(printf '%s\n' "$workunit_output" | awk -F': ' '/^work_unit_id:/ {print $2; exit}')"

assignment_start_log="$E2E_SCENARIO_REPORTS_DIR/assignment-start.txt"
e2e_orcas assignments start --workunit "$workunit_id" --worker harness-worker --worker-kind harness --instructions "Draft a planning report with at least two steps." --cwd "$e2e_repo_root" >"$assignment_start_log" 2>&1 &
assignment_start_pid=$!
assignment_start_cleanup() {
  kill "$assignment_start_pid" >/dev/null 2>&1 || true
}
trap assignment_start_cleanup EXIT
sleep 5
report_id="$(awk -F': ' '/^report_id:/ {print $2; exit}' "$assignment_start_log")"

if [[ -z "$report_id" ]]; then
  for _ in $(seq 1 20); do
    reports_output="$(e2e_orcas reports list-for-workunit --workunit "$workunit_id" 2>/dev/null || true)"
    report_id="$(printf '%s\n' "$reports_output" | awk -F'\t' '/^report-/ {print $1; exit}')"
    [[ -n "$report_id" ]] && break
    sleep 2
  done
fi

e2e_orcas workunits edit --workunit "$workunit_id" --status awaiting-decision >"$E2E_SCENARIO_REPORTS_DIR/workunit-edit.txt"

proposal_output="$(
  e2e_orcas proposals create \
    --workunit "$workunit_id" \
    --report "$report_id" \
    --requested-by harness \
    --note "Generate a bounded continue proposal for the pre-execution planning step. Keep every field terse. Use exactly 2 instructions, exactly 2 acceptance criteria, exactly 2 stop conditions, exactly 2 expected report fields, and a concise boundedness note. Set plan_assessment and plan_revision_proposal to null. Do not escalate or mark the work complete."
)"
proposal_id="$(printf '%s\n' "$proposal_output" | awk -F': ' '/^proposal_id:/ {print $2; exit}')"

e2e_orcas proposals get --proposal "$proposal_id" >"$E2E_SCENARIO_REPORTS_DIR/proposal-get.txt"
approve_output="$(e2e_orcas proposals approve --proposal "$proposal_id" --reviewed-by harness --review-note "Planning looks coherent" --rationale "Plan is inspectable and actionable" --type continue)"
approved_decision_id="$(printf '%s\n' "$approve_output" | awk -F': ' '/^approved_decision_id:/ {print $2; exit}')"

test -n "$workstream_id"
test -n "$workunit_id"
test -n "$report_id"
test -n "$proposal_id"
test -n "$approved_decision_id"

wait "$assignment_start_pid" >/dev/null 2>&1 || true

echo "PASS"
