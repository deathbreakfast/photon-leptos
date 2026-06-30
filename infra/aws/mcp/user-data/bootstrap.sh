#!/usr/bin/env bash
set -euo pipefail

ulimit -n "${ULIMIT_NOFILE:-65535}" || true

INSTALL_DIR="${INSTALL_DIR:-/opt/photon-leptos-bench}"
mkdir -p "$INSTALL_DIR"

if [[ -n "${RELEASE_S3_URI:-}" ]]; then
  aws s3 cp "$RELEASE_S3_URI" /tmp/release.tar.gz
  tar -xzf /tmp/release.tar.gz -C "$INSTALL_DIR"
fi

chmod +x "$INSTALL_DIR/photon-leptos-bench-server" || true
chmod +x "$INSTALL_DIR/photon-leptos-bench" || true

cat >/etc/systemd/system/photon-leptos-bench-server.service <<EOF
[Unit]
Description=photon-leptos bench server
After=network.target

[Service]
Type=simple
Environment=BENCH_ADDR=0.0.0.0:8080
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/photon-leptos-bench-server
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable photon-leptos-bench-server
systemctl restart photon-leptos-bench-server
