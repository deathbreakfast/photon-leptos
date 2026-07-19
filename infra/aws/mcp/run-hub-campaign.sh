#!/usr/bin/env bash
# Full hub + missing-slice campaign (sequential DUTs; respects 16 vCPU cap).
# Usage: CAMPAIGN=<uuid> ./run-hub-campaign.sh
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=campaign-ssh.sh
source "$SCRIPT_DIR/campaign-ssh.sh"

CAMPAIGN="${CAMPAIGN:?set CAMPAIGN uuid}"
REGION="${REGION:-us-west-2}"
AMI="${AMI:?set AMI to a pinned Ubuntu AMI id}"
SUBNET="${SUBNET:?set SUBNET to your bench subnet id}"
SERVER_SG="${SERVER_SG:?set SERVER_SG to your bench security group id}"
KEY="${KEY:-photon-leptos-bench}"
DURATION="${DURATION:-60}"
SOAK_DURATION="${SOAK_DURATION:-3600}"
WAVE="${WAVE:-all}" # all | t3 | c7i | pls9

launch_instance() {
  local role="$1" itype="$2" profile="$3"
  aws ec2 run-instances --region "$REGION" \
    --image-id "$AMI" --instance-type "$itype" --key-name "$KEY" \
    --subnet-id "$SUBNET" --security-group-ids "$SERVER_SG" \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Project,Value=photon-leptos-bench},{Key=Campaign,Value=$CAMPAIGN},{Key=Role,Value=$role},{Key=Profile,Value=$profile},{Key=Name,Value=plbench-$profile-$role}]" \
    --query 'Instances[0].InstanceId' --output text
}

wait_ips() {
  local id="$1"
  aws ec2 wait instance-running --region "$REGION" --instance-ids "$id"
  local pub priv
  pub=$(aws ec2 describe-instances --region "$REGION" --instance-ids "$id" \
    --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
  priv=$(aws ec2 describe-instances --region "$REGION" --instance-ids "$id" \
    --query 'Reservations[0].Instances[0].PrivateIpAddress' --output text)
  echo "$pub $priv"
}

terminate() {
  aws ec2 terminate-instances --region "$REGION" --instance-ids "$@" --output text >/dev/null || true
}

collect() {
  local host="$1" profile="$2"
  local dest="$REPO_ROOT/photon-leptos-bench/reports/$profile"
  mkdir -p "$dest" "$REPO_ROOT/photon-leptos-bench/reports"
  fetch_reports "$host" "$profile" "$dest"
  cp -f "$dest"/*.json "$REPO_ROOT/photon-leptos-bench/reports/" 2>/dev/null || true
}

run_wave_t3() {
  local profile="aws-t3-medium"
  echo "=== Wave t3.medium ==="
  local lg_id dut_id
  lg_id=$(launch_instance loadgen c7i.large aws-c7i-large)
  dut_id=$(launch_instance server t3.medium "$profile")
  echo "LG=$lg_id DUT=$dut_id"
  read -r LG_PUB LG_PRIV < <(wait_ips "$lg_id")
  read -r DUT_PUB DUT_PRIV < <(wait_ips "$dut_id")
  echo "LG $LG_PUB / DUT $DUT_PUB ($DUT_PRIV)"

  wait_ssh "$LG_PUB"
  wait_ssh "$DUT_PUB"
  scp_tar "$LG_PUB"
  scp_tar "$DUT_PUB"
  remote_bootstrap_loadgen "$LG_PUB"
  remote_bootstrap_server "$DUT_PUB"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" \
    "${SSH_USER}@${LG_PUB}:/tmp/profiles.json"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" \
    "${SSH_USER}@${DUT_PUB}:/tmp/profiles.json"
  ssh_cmd "$LG_PUB" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"
  ssh_cmd "$DUT_PUB" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"

  run_substrate "$DUT_PUB" "$profile" || echo "WARN: substrate failed (continuing)"
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
  # Soak at ~80% of Phase-1 knee (512 → 410)
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls8 "$SOAK_DURATION" --connections 410 --ws-mode per_subscribe
  ssh_cmd "$LG_PUB" "sudo mv $INSTALL_DIR/reports/bm-pls8-mem-embedded-${profile}.json $INSTALL_DIR/reports/bm-pls8-per_subscribe-mem-embedded-${profile}.json || true"
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls8 "$SOAK_DURATION" --connections 410 --ws-mode broadcast_hub
  ssh_cmd "$LG_PUB" "sudo mv $INSTALL_DIR/reports/bm-pls8-mem-embedded-${profile}.json $INSTALL_DIR/reports/bm-pls8-broadcast_hub-mem-embedded-${profile}.json || true"

  collect "$LG_PUB" "$profile"
  collect "$DUT_PUB" "$profile"
  terminate "$lg_id" "$dut_id"
  echo "=== Done t3.medium ==="
}

run_wave_c7i() {
  local profile="aws-c7i-large"
  echo "=== Wave c7i.large ==="
  local lg_id dut_id
  lg_id=$(launch_instance loadgen c7i.large aws-c7i-large-lg)
  dut_id=$(launch_instance server c7i.large "$profile")
  echo "LG=$lg_id DUT=$dut_id"
  read -r LG_PUB LG_PRIV < <(wait_ips "$lg_id")
  read -r DUT_PUB DUT_PRIV < <(wait_ips "$dut_id")

  wait_ssh "$LG_PUB"
  wait_ssh "$DUT_PUB"
  scp_tar "$LG_PUB"
  scp_tar "$DUT_PUB"
  remote_bootstrap_loadgen "$LG_PUB"
  remote_bootstrap_server "$DUT_PUB"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" \
    "${SSH_USER}@${LG_PUB}:/tmp/profiles.json"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" \
    "${SSH_USER}@${DUT_PUB}:/tmp/profiles.json"
  ssh_cmd "$LG_PUB" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"
  ssh_cmd "$DUT_PUB" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"

  run_substrate "$DUT_PUB" "$profile" || echo "WARN: substrate failed (continuing)"
  restart_server "$DUT_PUB"

  run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-hub "$DURATION" per_subscribe
  restart_server "$DUT_PUB"
  run_slice "$LG_PUB" "$DUT_PRIV" "$profile" pls-connection "$DURATION" per_subscribe
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls1 "$DURATION" --ws-mode broadcast_hub
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls5 "$DURATION" --connections 256 --key-groups 1 --ws-mode per_subscribe
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls5 "$DURATION" --connections 256 --key-groups 256 --ws-mode per_subscribe \
    --report "./reports/bm-pls5-g256-mem-embedded-${profile}.json" || true
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls5-hub "$DURATION" --connections 256 --key-groups 1
  restart_server "$DUT_PUB"
  run_exp "$LG_PUB" "$DUT_PRIV" "$profile" bm-pls5-hub "$DURATION" --connections 256 --key-groups 256

  collect "$LG_PUB" "$profile"
  collect "$DUT_PUB" "$profile"
  terminate "$lg_id" "$dut_id"
  echo "=== Done c7i.large ==="
}

run_wave_pls9() {
  local profile="aws-c7i-large"
  echo "=== Wave PLS9 (2× DUT + LG) ==="
  local lg_id dut_a dut_b
  lg_id=$(launch_instance loadgen c7i.large aws-c7i-large-lg)
  dut_a=$(launch_instance server c7i.large "${profile}-a")
  dut_b=$(launch_instance server c7i.large "${profile}-b")
  read -r LG_PUB LG_PRIV < <(wait_ips "$lg_id")
  read -r A_PUB A_PRIV < <(wait_ips "$dut_a")
  read -r B_PUB B_PRIV < <(wait_ips "$dut_b")

  wait_ssh "$LG_PUB"; wait_ssh "$A_PUB"; wait_ssh "$B_PUB"
  scp_tar "$LG_PUB"; scp_tar "$A_PUB"; scp_tar "$B_PUB"
  remote_bootstrap_loadgen "$LG_PUB"
  remote_bootstrap_server "$A_PUB"
  remote_bootstrap_server "$B_PUB"
  scp -i "$SSH_KEY" -o StrictHostKeyChecking=no "$REPO_ROOT/infra/aws/mcp/profiles.json" \
    "${SSH_USER}@${LG_PUB}:/tmp/profiles.json"
  ssh_cmd "$LG_PUB" "sudo mv /tmp/profiles.json $INSTALL_DIR/profiles.json"

  restart_server "$A_PUB"
  restart_server "$B_PUB"
  ssh_cmd "$LG_PUB" "sudo bash -c 'cd $INSTALL_DIR && mkdir -p reports && \
    env PHOTON_LEPTOS_BENCH_PROFILES=$INSTALL_DIR/profiles.json \
    ./photon-leptos-bench run --experiment bm-pls9 --hardware $profile \
      --server-urls http://${A_PRIV}:8080,http://${B_PRIV}:8080 \
      --connections 256 --duration-secs $DURATION --ws-mode per_subscribe \
      --report ./reports/bm-pls9-mem-embedded-${profile}.json'"

  collect "$LG_PUB" "pls9"
  terminate "$lg_id" "$dut_a" "$dut_b"
  echo "=== Done PLS9 ==="
}

case "$WAVE" in
  all) run_wave_t3; run_wave_c7i; run_wave_pls9 ;;
  t3) run_wave_t3 ;;
  c7i) run_wave_c7i ;;
  pls9) run_wave_pls9 ;;
  *) echo "unknown WAVE=$WAVE"; exit 1 ;;
esac

echo "Campaign $CAMPAIGN complete"
