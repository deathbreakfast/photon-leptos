#!/usr/bin/env bash
set -euo pipefail

REGION="${REGION:-us-west-2}"
CAMPAIGN="${CAMPAIGN:-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/photon-leptos-bench.pem}"
SSH_USER="${SSH_USER:-ubuntu}"
INSTALL_DIR="${INSTALL_DIR:-/opt/photon-leptos-bench}"
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TARBALL="${TARBALL:-$REPO_ROOT/release.tar.gz}"

ssh_cmd() {
  local host="$1"; shift
  ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=15 \
    "${SSH_USER}@${host}" "$@"
}

scp_tar() {
  local host="$1"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$TARBALL" "${SSH_USER}@${host}:/tmp/release.tar.gz"
}

remote_bootstrap_server() {
  local host="$1"
  ssh_cmd "$host" "sudo mkdir -p '$INSTALL_DIR' && sudo tar -xzf /tmp/release.tar.gz -C '$INSTALL_DIR' && \
    sudo chmod +x '$INSTALL_DIR/photon-leptos-bench' '$INSTALL_DIR/photon-leptos-bench-server' && \
    (test -x '$INSTALL_DIR/photon-bench' && sudo chmod +x '$INSTALL_DIR/photon-bench' || true) && \
    sudo bash '$INSTALL_DIR/user-data/bootstrap.sh'"
}

remote_bootstrap_loadgen() {
  local host="$1"
  ssh_cmd "$host" "sudo mkdir -p '$INSTALL_DIR' && sudo tar -xzf /tmp/release.tar.gz -C '$INSTALL_DIR' && \
    sudo chmod +x '$INSTALL_DIR/photon-leptos-bench' && \
    (test -x '$INSTALL_DIR/photon-bench' && sudo chmod +x '$INSTALL_DIR/photon-bench' || true)"
}

wait_ssh() {
  local host="$1"
  for _ in $(seq 1 60); do
    if ssh_cmd "$host" "echo ok" >/dev/null 2>&1; then
      return 0
    fi
    sleep 10
  done
  return 1
}

run_substrate() {
  local host="$1" profile="$2"
  # photon-bench CLI (v0.1.1): storage selects adapter; no --backend flag.
  ssh_cmd "$host" "cd '$INSTALL_DIR' && sudo mkdir -p reports && \
    sudo ./photon-bench run --experiment bm-p2 --storage mem \
      --hardware '$profile' --report ./reports/substrate-bm-p2-${profile}.json && \
    sudo ./photon-bench run --experiment bm-pl2 --storage mem \
      --hardware '$profile' --ops 60 --report ./reports/substrate-bm-pl2-${profile}.json"
}

run_pls_connection() {
  local host="$1" profile="$2" server_url="$3" duration="$4"
  ssh_cmd "$host" "cd '$INSTALL_DIR' && sudo mkdir -p reports && \
    sudo env PHOTON_LEPTOS_BENCH_PROFILES='$INSTALL_DIR/profiles.json' \
    ./photon-leptos-bench matrix --slice pls-connection --hardware '$profile' \
      --server-url '$server_url' --reports-dir ./reports --duration-secs '$duration' --skip-existing"
}

fetch_reports() {
  local host="$1" profile="$2" dest="$3"
  mkdir -p "$dest"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no -r \
    "${SSH_USER}@${host}:${INSTALL_DIR}/reports/"* "$dest/" 2>/dev/null || true
}

restart_server() {
  local host="$1"
  ssh_cmd "$host" "sudo systemctl restart photon-leptos-bench-server && sleep 3 && curl -sf --max-time 5 http://127.0.0.1:8080/health"
}

run_pls_connection_campaign() {
  local lg_host="$1" server_host="$2" server_private="$3" profile="$4" duration="$5"
  local install="$INSTALL_DIR"
  restart_server "$server_host"
  ssh_cmd "$lg_host" "sudo bash -c 'cd $install && mkdir -p reports && env PHOTON_LEPTOS_BENCH_PROFILES=$install/profiles.json ./photon-leptos-bench run --experiment bm-pls0 --hardware $profile --server-url http://${server_private}:8080 --rate-per-sec 100 --duration-secs $duration --report ./reports/bm-pls0-rate100-${profile}.json'"
  restart_server "$server_host"
  ssh_cmd "$lg_host" "sudo bash -c 'cd $install && env PHOTON_LEPTOS_BENCH_PROFILES=$install/profiles.json ./photon-leptos-bench run --experiment bm-pls0 --hardware $profile --server-url http://${server_private}:8080 --rate-per-sec 1000 --duration-secs $duration --report ./reports/bm-pls0-rate1000-${profile}.json'"
  restart_server "$server_host"
  ssh_cmd "$lg_host" "sudo bash -c 'cd $install && env PHOTON_LEPTOS_BENCH_PROFILES=$install/profiles.json ./photon-leptos-bench run --experiment bm-pls1 --hardware $profile --server-url http://${server_private}:8080 --duration-secs $duration --report ./reports/bm-pls1-mem-embedded-${profile}.json'"
  ssh_cmd "$lg_host" "sudo python3 - <<'PY'
import json, pathlib
install = pathlib.Path('$install/reports')
profile = '$profile'
knees = []
for rate in (100, 1000):
    p = install / f'bm-pls0-rate{rate}-{profile}.json'
    if p.exists():
        knees.append(json.loads(p.read_text()).get('knee_connection_count') or 0)
knee = max(knees) if knees else 0
out = install / f'bm-pls0-mem-embedded-{profile}.json'
if (install / f'bm-pls0-rate100-{profile}.json').exists():
    base = json.loads((install / f'bm-pls0-rate100-{profile}.json').read_text())
else:
    base = {'experiment':'bm-pls0','matrix_slug':'mem-embedded-off-embedded-composite','scenario_id':'pls0-connection-sweep','hardware':profile,'profile_phase':1,'backend_id':'embedded','topology':'embedded-composite','telemetry':'off','storage':'mem','payload_bytes':256,'client_type':'synthetic'}
base['knee_connection_count'] = knee
base['pass'] = knee > 0
base['status'] = 'pass' if knee > 0 else 'fail'
out.write_text(json.dumps(base, indent=2))
print('merged pls0 knee', knee)
PY"
}

# Run a matrix slice from the loadgen against server private IP.
# Logs go to a remote file (never pipe through `tail` — that deadlocks on full pipes).
run_slice() {
  local lg_host="$1" server_private="$2" profile="$3" slice="$4" duration="$5"
  local ws_mode="${6:-}"
  local install="$INSTALL_DIR"
  local mode_arg=""
  if [[ -n "$ws_mode" ]]; then
    mode_arg="--ws-mode $ws_mode"
  fi
  local log="$install/reports/matrix-${slice}-${profile}.log"
  # Do not stream logs over SSH (pipe backpressure can stall the session).
  ssh_cmd "$lg_host" "sudo bash -c 'cd $install && mkdir -p reports && \
    env PHOTON_LEPTOS_BENCH_PROFILES=$install/profiles.json \
    ./photon-leptos-bench matrix --slice $slice --hardware $profile \
      --server-url http://${server_private}:8080 --reports-dir ./reports \
      --duration-secs $duration $mode_arg >$log 2>&1; \
    echo EXIT:\$? >>$log'"
  echo "slice $slice finished (see $log)"
}

# Single experiment helper (optional --ws-mode / --key-groups / --connections).
run_exp() {
  local lg_host="$1" server_private="$2" profile="$3" experiment="$4" duration="$5"
  shift 5
  local install="$INSTALL_DIR"
  local extra=("$@")
  local log="$install/reports/run-${experiment}-${profile}.log"
  ssh_cmd "$lg_host" "sudo bash -c 'cd $install && mkdir -p reports && \
    env PHOTON_LEPTOS_BENCH_PROFILES=$install/profiles.json \
    ./photon-leptos-bench run --experiment $experiment --hardware $profile \
      --server-url http://${server_private}:8080 --duration-secs $duration \
      --report ./reports/${experiment}-mem-embedded-${profile}.json ${extra[*]} \
      >$log 2>&1; echo EXIT:\$? >>$log'"
  echo "exp $experiment finished (see $log)"
}
