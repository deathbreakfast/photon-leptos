#!/usr/bin/env bash
# Resume t3 wave against already-running LG/DUT (after redeploy).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=campaign-ssh.sh
source "$SCRIPT_DIR/campaign-ssh.sh"

LG_PUB="${1:?lg public ip}"
DUT_PUB="${2:?dut public ip}"
DUT_PRIV="${3:?dut private ip}"
profile="aws-t3-medium"
DURATION="${DURATION:-30}"
SOAK_DURATION="${SOAK_DURATION:-3600}"
install="$INSTALL_DIR"

restart_server "$DUT_PUB"
run_substrate "$DUT_PUB" "$profile" || echo "WARN: substrate failed"

restart_server "$DUT_PUB"
run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-hub "$DURATION" per_subscribe
restart_server "$DUT_PUB"
run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-connection "$DURATION" per_subscribe
restart_server "$DUT_PUB"
run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls1 "$DURATION" --ws-mode broadcast_hub
restart_server "$DUT_PUB"
run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-client "$DURATION" per_subscribe
restart_server "$DUT_PUB"
run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-shape "$DURATION" per_subscribe
restart_server "$DUT_PUB"
run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls8 "$SOAK_DURATION" --connections 410 --ws-mode per_subscribe
ssh_cmd "$LG_PUB" "sudo mv $install/reports/bm-pls8-mem-embedded-${profile}.json $install/reports/bm-pls8-per_subscribe-mem-embedded-${profile}.json || true"
restart_server "$DUT_PUB"
run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls8 "$SOAK_DURATION" --connections 410 --ws-mode broadcast_hub
ssh_cmd "$LG_PUB" "sudo mv $install/reports/bm-pls8-mem-embedded-${profile}.json $install/reports/bm-pls8-broadcast_hub-mem-embedded-${profile}.json || true"

collect() {
  local host="$1"
  local dest="$REPO_ROOT/photon-leptos-bench/reports/$profile"
  mkdir -p "$dest" "$REPO_ROOT/photon-leptos-bench/reports"
  fetch_reports "$host" "$profile" "$dest"
  cp -f "$dest"/*.json "$REPO_ROOT/photon-leptos-bench/reports/" 2>/dev/null || true
}
collect "$LG_PUB"
collect "$DUT_PUB"
echo "=== Resume t3 complete ==="
