#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "$0")/../../lib" && pwd)/common.sh"
scenario_dir="$(cd "$(dirname "$0")" && pwd)"
e2e_load_scenario_metadata "$scenario_dir"
e2e_prepare_scenario_dirs "$NAME"

daemon_log="$E2E_SCENARIO_LOGS_DIR/ttd.log"

e2e_start_managed_daemon "$daemon_log"
trap e2e_stop_managed_daemon EXIT

sleep 5

e2e_tt doctor >/dev/null

log_dir="$E2E_SCENARIO_XDG_DATA_HOME/tt/logs"
test -d "$log_dir"
test -f "$log_dir/ttd.log"
test -f "$log_dir/tt.log"

echo "PASS"
