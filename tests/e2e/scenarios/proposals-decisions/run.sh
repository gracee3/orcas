#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/../../lib" && pwd)/common.sh"
scenario_dir="$(cd "$(dirname "$0")" && pwd)"
e2e_load_scenario_metadata "$scenario_dir"
e2e_prepare_scenario_dirs "$NAME"
e2e_use_short_xdg_paths "pd"

tt_cli() {
  e2e_tt --connect-only "$@"
}

e2e_normalize_state_json "$scenario_dir/seed_state.json" "$E2E_SCENARIO_XDG_DATA_HOME/tt/state.json"
rm -f "$E2E_SCENARIO_XDG_DATA_HOME/tt/state.db" "$E2E_SCENARIO_XDG_DATA_HOME/tt/state.db-wal" "$E2E_SCENARIO_XDG_DATA_HOME/tt/state.db-shm"

daemon_log="$E2E_SCENARIO_LOGS_DIR/ttd.log"
e2e_ttd --connect-only >"$daemon_log" 2>&1 &
daemon_pid=$!
cleanup() {
  kill "$daemon_pid" >/dev/null 2>&1 || true
}
trap cleanup EXIT

sleep 5

workstream_id="ws-proposals"
workunit_id="wu-proposals"
report_id="report-proposals"
proposal_id="proposal-proposals"

tt_cli supervisor work proposals get --proposal "$proposal_id" >"$E2E_SCENARIO_REPORTS_DIR/proposal-get.txt"
approve_output="$(tt_cli supervisor work proposals approve --proposal "$proposal_id" --reviewed-by harness --review-note "Looks good" --rationale "Proposal is valid" --type accept)"
approved_decision_id="$(printf '%s\n' "$approve_output" | awk -F': ' '/^decision_id:/ {print $2; exit}')"

test -n "$workstream_id"
test -n "$workunit_id"
test -n "$report_id"
test -n "$proposal_id"
test -n "$approved_decision_id"

echo "PASS"
