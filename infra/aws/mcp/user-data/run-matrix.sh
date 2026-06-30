#!/usr/bin/env bash
set -euo pipefail

SLICE="${1:-pls-connection}"
HARDWARE="${2:-aws-t3-medium}"
SERVER_URL="${3:-http://127.0.0.1:8080}"
REPORTS_S3_PREFIX="${REPORTS_S3_PREFIX:-}"

INSTALL_DIR="${INSTALL_DIR:-/opt/photon-leptos-bench}"
cd "$INSTALL_DIR"

./photon-leptos-bench matrix \
  --slice "$SLICE" \
  --hardware "$HARDWARE" \
  --server-url "$SERVER_URL" \
  --reports-dir ./reports \
  --duration-secs 30

if [[ -n "$REPORTS_S3_PREFIX" ]]; then
  aws s3 sync ./reports "$REPORTS_S3_PREFIX"
fi
