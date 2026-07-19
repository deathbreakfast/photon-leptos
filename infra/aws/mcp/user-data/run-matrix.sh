#!/usr/bin/env bash
set -euo pipefail

SLICE="${1:-pls-connection}"
HARDWARE="${2:-aws-t3-medium}"
SERVER_URL="${3:-http://127.0.0.1:8080}"
REPORTS_S3_PREFIX="${REPORTS_S3_PREFIX:-}"
DURATION_SECS="${DURATION_SECS:-60}"
WS_MODE="${WS_MODE:-}"

INSTALL_DIR="${INSTALL_DIR:-/opt/photon-leptos-bench}"
cd "$INSTALL_DIR"

ARGS=(
  matrix
  --slice "$SLICE"
  --hardware "$HARDWARE"
  --server-url "$SERVER_URL"
  --reports-dir ./reports
  --duration-secs "$DURATION_SECS"
)
if [[ -n "$WS_MODE" ]]; then
  ARGS+=(--ws-mode "$WS_MODE")
fi

./photon-leptos-bench "${ARGS[@]}"

if [[ -n "$REPORTS_S3_PREFIX" ]]; then
  aws s3 sync ./reports "$REPORTS_S3_PREFIX"
fi
