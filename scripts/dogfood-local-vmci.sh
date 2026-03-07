#!/usr/bin/env bash
# Self-hosted dogfood with VM-isolated CI: Aspen cluster runs locally,
# CI jobs execute in Cloud Hypervisor microVMs.
#
# Usage:
#   nix run .#dogfood-local-vmci           # Full pipeline
#   nix run .#dogfood-local-vmci -- start  # Just start the cluster
#   nix run .#dogfood-local-vmci -- push   # Push source to existing cluster
#   nix run .#dogfood-local-vmci -- build  # Trigger CI build on existing cluster
#   nix run .#dogfood-local-vmci -- stop   # Stop the cluster
#   nix run .#dogfood-local-vmci -- status # Show cluster status
#
# Prerequisites:
#   1. KVM available (/dev/kvm)
#   2. Network bridge configured: sudo nix run .#setup-ci-network
#   3. Set by flake.nix wrapper: ASPEN_NODE_BIN, ASPEN_CLI_BIN,
#      GIT_REMOTE_ASPEN_BIN, ASPEN_CI_KERNEL_PATH, ASPEN_CI_INITRD_PATH,
#      ASPEN_CI_TOPLEVEL_PATH, CLOUD_HYPERVISOR_BIN, VIRTIOFSD_BIN, PROJECT_DIR
set -euo pipefail

CLUSTER_DIR="/tmp/aspen-dogfood-vmci"
COOKIE="dogfood-vmci-$(date +%Y%m%d)"
NODE_COUNT="${ASPEN_NODE_COUNT:-1}"

# VM configuration (overridable)
VM_MEMORY_MIB="${ASPEN_CI_VM_MEMORY_MIB:-24576}"
VM_VCPUS="${ASPEN_CI_VM_VCPUS:-4}"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log() { echo -e "${BLUE}[dogfood-vmci]${NC} $*"; }
ok()  { echo -e "${GREEN}  ✅ $*${NC}"; }
err() { echo -e "${RED}  ❌ $*${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $*${NC}"; }

# ── Binaries ──────────────────────────────────────────────────────────

ASPEN_NODE="${ASPEN_NODE_BIN:-aspen-node}"
ASPEN_CLI="${ASPEN_CLI_BIN:-aspen-cli}"
PROJECT="${PROJECT_DIR:-$(pwd)}"

cli() {
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt" 2>/dev/null) || {
    err "No cluster ticket found. Is the cluster running?"
    return 1
  }
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

cli_json() {
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt" 2>/dev/null) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --json "$@"
}

parse_json() {
  local expr="$1"
  PARSE_EXPR="$expr" python3 -c "
import json, sys, re, os
raw = sys.stdin.read()
expr = os.environ['PARSE_EXPR']
# Try each '{' or '[' position - log lines (e.g. 'Os { code: 101 }')
# can contain braces before the real JSON, so skip invalid starts.
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        result = eval(expr)
        print(result if result else '')
    except:
        print('')
    sys.exit(0)
print('')
" 2>/dev/null
}

# ── Preflight Checks ─────────────────────────────────────────────────

check_prerequisites() {
  local ok=true

  # KVM
  if [ ! -e /dev/kvm ]; then
    err "KVM not available (/dev/kvm missing)"
    echo "    Enable virtualization in BIOS and ensure kvm module is loaded"
    ok=false
  elif [ ! -w /dev/kvm ]; then
    err "No write access to /dev/kvm (add user to kvm group)"
    ok=false
  else
    ok "KVM available"
  fi

  # Network bridge
  if ip link show aspen-ci-br0 &>/dev/null; then
    ok "Network bridge aspen-ci-br0 exists"
  else
    err "Network bridge aspen-ci-br0 not found"
    echo "    Run: sudo nix run .#setup-ci-network"
    ok=false
  fi

  # NAT rules (nftables masquerade for VM internet access)
  # Check actual rules if possible. nft/iptables need root to query rules,
  # so fall back to the marker file if we can't check directly.
  if nft list table ip aspen-ci-nat &>/dev/null; then
    ok "NAT configured (nftables rule verified)"
  elif iptables -t nat -C POSTROUTING -s 10.200.0.0/24 ! -o aspen-ci-br0 -j MASQUERADE &>/dev/null; then
    ok "NAT configured (iptables rule verified)"
  elif [ -f /tmp/aspen-ci-network-configured ]; then
    # Can't query rules without root — trust the marker if present.
    # The marker is set by setup-ci-network.sh which runs as root.
    ok "NAT configured (marker present, run 'sudo nix run .#setup-ci-network' if VMs lack internet)"
  else
    err "NAT not configured — VMs cannot reach internet"
    echo "    Run: sudo nix run .#setup-ci-network"
    ok=false
  fi

  # VM image
  if [ -z "${ASPEN_CI_KERNEL_PATH:-}" ]; then
    err "ASPEN_CI_KERNEL_PATH not set (flake wrapper should provide this)"
    ok=false
  elif [ ! -e "$ASPEN_CI_KERNEL_PATH" ]; then
    err "CI VM kernel not found: $ASPEN_CI_KERNEL_PATH"
    ok=false
  else
    ok "CI VM kernel: $(basename "$ASPEN_CI_KERNEL_PATH")"
  fi

  if [ -z "${ASPEN_CI_INITRD_PATH:-}" ]; then
    err "ASPEN_CI_INITRD_PATH not set"
    ok=false
  elif [ ! -e "$ASPEN_CI_INITRD_PATH" ]; then
    err "CI VM initrd not found: $ASPEN_CI_INITRD_PATH"
    ok=false
  else
    ok "CI VM initrd: $(basename "$ASPEN_CI_INITRD_PATH")"
  fi

  if [ -z "${ASPEN_CI_TOPLEVEL_PATH:-}" ]; then
    err "ASPEN_CI_TOPLEVEL_PATH not set"
    ok=false
  elif [ ! -e "$ASPEN_CI_TOPLEVEL_PATH" ]; then
    err "CI VM toplevel not found: $ASPEN_CI_TOPLEVEL_PATH"
    ok=false
  else
    ok "CI VM toplevel: $(basename "$ASPEN_CI_TOPLEVEL_PATH")"
  fi

  # cloud-hypervisor
  if command -v cloud-hypervisor &>/dev/null || [ -n "${CLOUD_HYPERVISOR_BIN:-}" ]; then
    ok "cloud-hypervisor available"
  else
    err "cloud-hypervisor not found in PATH"
    ok=false
  fi

  # virtiofsd
  if command -v virtiofsd &>/dev/null || [ -n "${VIRTIOFSD_BIN:-}" ]; then
    ok "virtiofsd available"
  else
    err "virtiofsd not found in PATH"
    ok=false
  fi

  if [ "$ok" = false ]; then
    echo ""
    err "Prerequisites not met. Fix the above issues and retry."
    exit 1
  fi
}

# ── Stop ──────────────────────────────────────────────────────────────

do_stop() {
  log "Stopping cluster..."
  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      ok "Node $i (PID $pid) stopped"
    fi
  done
  sleep 1

  # Clean up any orphaned virtiofsd processes
  pkill -f "virtiofsd.*aspen-dogfood-vmci" 2>/dev/null || true

  rm -rf "$CLUSTER_DIR"
  ok "Cluster directory cleaned up"
}

# ── Start ─────────────────────────────────────────────────────────────

do_start() {
  check_prerequisites

  log "Starting ${NODE_COUNT}-node cluster with VM CI..."
  echo ""
  echo -e "  ${BOLD}VM Configuration:${NC}"
  echo "    Memory:  ${VM_MEMORY_MIB} MiB"
  echo "    vCPUs:   ${VM_VCPUS}"
  echo "    Kernel:  $(basename "${ASPEN_CI_KERNEL_PATH}")"
  echo "    Network: TAP (aspen-ci-br0)"
  echo ""

  # Kill any existing cluster
  if [ -d "$CLUSTER_DIR" ]; then
    warn "Existing cluster found, cleaning up..."
    do_stop
  fi

  mkdir -p "$CLUSTER_DIR"
  echo "$COOKIE" > "$CLUSTER_DIR/cookie"

  # Start nodes
  for i in $(seq 1 "$NODE_COUNT"); do
    local key
    key=$(printf '%064x' $((3000 + i)))
    mkdir -p "$CLUSTER_DIR/node$i"

    # CI VM environment — CloudHypervisorWorker picks these up
    ASPEN_CI_KERNEL_PATH="${ASPEN_CI_KERNEL_PATH}" \
    ASPEN_CI_INITRD_PATH="${ASPEN_CI_INITRD_PATH}" \
    ASPEN_CI_TOPLEVEL_PATH="${ASPEN_CI_TOPLEVEL_PATH}" \
    ASPEN_CI_VM_MEMORY_MIB="${VM_MEMORY_MIB}" \
    ASPEN_CI_VM_VCPUS="${VM_VCPUS}" \
    ASPEN_CI_NETWORK_MODE="tap" \
    CLOUD_HYPERVISOR_PATH="${CLOUD_HYPERVISOR_BIN:-$(command -v cloud-hypervisor)}" \
    VIRTIOFSD_PATH="${VIRTIOFSD_BIN:-$(command -v virtiofsd)}" \
    ASPEN_DOCS_ENABLED=true \
    ASPEN_DOCS_IN_MEMORY=true \
    ASPEN_HOOKS_ENABLED=true \
    "$ASPEN_NODE" \
      --node-id "$i" \
      --data-dir "$CLUSTER_DIR/node$i" \
      --cookie "$COOKIE" \
      --iroh-secret-key "$key" \
      --disable-mdns \
      --heartbeat-interval-ms 500 \
      --election-timeout-min-ms 1500 \
      --election-timeout-max-ms 3000 \
      --enable-workers \
      --enable-ci \
      --ci-auto-trigger \
      > "$CLUSTER_DIR/node$i.log" 2>&1 &
    echo $! > "$CLUSTER_DIR/node$i.pid"
  done

  log "Waiting for nodes to start..."
  local retries=30
  while [ $retries -gt 0 ]; do
    if [ -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
      break
    fi
    sleep 1
    retries=$((retries - 1))
  done

  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "Node 1 failed to start within 30s"
    cat "$CLUSTER_DIR/node1.log" 2>/dev/null | tail -20
    exit 1
  fi

  # Check all nodes are alive
  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid")
    if kill -0 "$pid" 2>/dev/null; then
      ok "Node $i (PID $pid): running"
    else
      err "Node $i failed — check $CLUSTER_DIR/node$i.log"
      exit 1
    fi
  done

  # Initialize cluster
  log "Initializing cluster..."
  cli cluster init 2>/dev/null || true
  sleep 2

  # Add learners and form cluster if multi-node
  if [ "$NODE_COUNT" -gt 1 ]; then
    local strip_ansi='sed s/\x1b\[[0-9;]*m//g'
    for i in $(seq 2 "$NODE_COUNT"); do
      local endpoint addr
      endpoint=$(grep "created Iroh endpoint" "$CLUSTER_DIR/node$i.log" \
        | $strip_ansi | grep -oP 'endpoint_id=\K[a-f0-9]+')
      addr=$(grep "direct_addrs" "$CLUSTER_DIR/node$i.log" \
        | $strip_ansi | grep -oP '\d+\.\d+\.\d+\.\d+:\d+' | head -1)
      log "Adding node $i as learner..."
      cli cluster add-learner --node-id "$i" \
        --addr "{\"id\":\"$endpoint\",\"addrs\":[{\"Ip\":\"$addr\"}]}" 2>/dev/null
      sleep 1
    done

    log "Promoting all nodes to voters..."
    local members
    members=$(seq -s' ' 1 "$NODE_COUNT")
    cli cluster change-membership $members 2>/dev/null
    sleep 2
  fi

  # Verify cluster health
  if cli cluster health 2>/dev/null | grep -q healthy; then
    ok "Cluster healthy"
  else
    warn "Cluster health check inconclusive (single-node is OK)"
  fi

  # Verify VM CI worker registration
  log "Checking for Cloud Hypervisor VM pool..."
  sleep 2
  if grep -q "Cloud Hypervisor VM pool manager initialized" "$CLUSTER_DIR/node1.log" 2>/dev/null; then
    ok "VM pool initialized"
    local pool_info
    pool_info=$(grep "Cloud Hypervisor VM pool" "$CLUSTER_DIR/node1.log" | tail -1 | sed 's/.*pool_size=/pool_size=/')
    echo -e "    ${pool_info}"
  else
    warn "VM pool not detected in logs — checking for LocalExecutor fallback..."
    if grep -q "Local Executor worker registered" "$CLUSTER_DIR/node1.log" 2>/dev/null; then
      warn "LocalExecutor registered (ASPEN_CI_LOCAL_EXECUTOR may be set)"
    elif grep -q "ci-vm-executor.*not enabled" "$CLUSTER_DIR/node1.log" 2>/dev/null; then
      err "ci-vm-executor feature not compiled in. Node needs ci-vm-executor feature."
      echo "    The aspen-node binary must be built with --features ci-vm-executor"
      exit 1
    else
      warn "No CI executor detected — check $CLUSTER_DIR/node1.log"
    fi
  fi

  log "Cluster started at $CLUSTER_DIR"
  echo ""
  echo -e "${BOLD}Cluster ticket:${NC}"
  cat "$CLUSTER_DIR/node1/cluster-ticket.txt"
  echo ""
}

# ── Status ────────────────────────────────────────────────────────────

do_status() {
  if [ ! -d "$CLUSTER_DIR" ]; then
    err "No cluster running"
    exit 1
  fi

  echo -e "${BOLD}╔══════════════════════════════════════════════════╗${NC}"
  echo -e "${BOLD}║     Aspen Self-Hosted Cluster (VM CI) Status     ║${NC}"
  echo -e "${BOLD}╚══════════════════════════════════════════════════╝${NC}"
  echo ""

  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || echo "?")
    if [ "$pid" != "?" ] && kill -0 "$pid" 2>/dev/null; then
      ok "Node $i (PID $pid): running"
    else
      err "Node $i: not running"
    fi
  done

  echo ""

  # VM pool status
  echo -e "${BOLD}VM Pool:${NC}"
  if grep -q "Cloud Hypervisor VM pool" "$CLUSTER_DIR/node1.log" 2>/dev/null; then
    grep "Cloud Hypervisor VM pool\|VM boot\|VM shutdown\|ci_vm.*job" "$CLUSTER_DIR/node1.log" \
      | tail -5 | sed 's/^/  /'
  else
    echo "  No VM pool detected"
  fi
  echo ""

  # Network bridge
  echo -e "${BOLD}Network:${NC}"
  if ip link show aspen-ci-br0 &>/dev/null; then
    echo -e "  ${GREEN}aspen-ci-br0: UP${NC}"
    ip addr show aspen-ci-br0 | grep 'inet ' | sed 's/^/  /'
  else
    echo -e "  ${RED}aspen-ci-br0: DOWN${NC}"
  fi
  echo ""

  cli cluster status 2>/dev/null || warn "Could not get cluster status"
  echo ""
  cli cluster metrics 2>/dev/null || true
}

# ── Push ──────────────────────────────────────────────────────────────

do_push() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  local ticket repo_id

  log "Creating Forge repository..."
  local create_out
  create_out=$(cli_json git init aspen 2>&1)
  repo_id=$(parse_json "d.get('id') or d.get('repo_id','')" <<< "$create_out")

  if [ -z "$repo_id" ]; then
    log "Repo may already exist, checking..."
    local list_out
    list_out=$(cli_json git list 2>&1)
    repo_id=$(parse_json "next((r.get('id') or r.get('repo_id','') for r in (d.get('repos') or d.get('repositories') or []) if r.get('name')=='aspen'), '')" <<< "$list_out")
  fi

  if [ -z "$repo_id" ]; then
    err "Failed to create or find Forge repository"
    echo "Create output: $create_out"
    exit 1
  fi
  ok "Forge repo: $repo_id"

  # Enable CI watch on the repo
  log "Enabling CI auto-trigger on repo..."
  cli_json ci watch "$repo_id" 2>/dev/null || warn "CI watch may already be enabled"

  # Push source to Forge
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
  local remote_url="aspen://${ticket}/${repo_id}"

  log "Pushing Aspen source to Forge..."
  cd "$PROJECT"

  if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
    log "Clean working tree, pushing directly..."
  else
    warn "Working tree has uncommitted changes (pushing HEAD)"
  fi

  git remote remove aspen-dogfood 2>/dev/null || true
  git remote add aspen-dogfood "$remote_url"

  log "Pushing to aspen://<ticket>/${repo_id}..."
  if RUST_LOG=warn git push aspen-dogfood HEAD:main 2>&1 | tail -5; then
    ok "Source pushed to Forge"
  else
    err "Push failed"
    exit 1
  fi

  git remote remove aspen-dogfood 2>/dev/null || true
  echo ""
  echo -e "${BOLD}Repo ID:${NC} $repo_id"
  echo "$repo_id" > "$CLUSTER_DIR/repo_id"
}

# ── Build ─────────────────────────────────────────────────────────────

do_build() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  local repo_id
  repo_id=$(cat "$CLUSTER_DIR/repo_id" 2>/dev/null || true)

  if [ -z "$repo_id" ]; then
    err "No repo ID found. Push source first: $0 push"
    exit 1
  fi

  log "Checking for auto-triggered pipeline..."
  local deadline=$((SECONDS + 120))
  local run_id=""

  while [ $SECONDS -lt $deadline ]; do
    local list_out
    list_out=$(cli_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")

    if [ -n "$run_id" ]; then
      ok "Pipeline found: $run_id"
      break
    fi
    sleep 3
  done

  if [ -z "$run_id" ]; then
    warn "No auto-triggered pipeline found, manually triggering..."
    local trigger_out
    trigger_out=$(cli_json ci run "$repo_id" 2>&1) || {
      err "CI trigger command failed"
      echo "Output: $trigger_out"
      exit 1
    }
    run_id=$(parse_json "d.get('run_id','')" <<< "$trigger_out")

    if [ -z "$run_id" ]; then
      err "Failed to trigger pipeline"
      echo "Output: $trigger_out"
      exit 1
    fi
    ok "Pipeline triggered: $run_id"
  fi

  echo "$run_id" > "$CLUSTER_DIR/run_id"

  log "Streaming pipeline progress..."
  echo ""
  stream_pipeline "$run_id"
}

# ── Stream Pipeline ──────────────────────────────────────────────────

stream_pipeline() {
  local run_id="$1"
  local streaming_job=""
  local stream_pid=""
  local seen_jobs=""

  cleanup_stream() {
    if [ -n "$stream_pid" ] && kill -0 "$stream_pid" 2>/dev/null; then
      kill "$stream_pid" 2>/dev/null || true
      wait "$stream_pid" 2>/dev/null || true
    fi
  }
  trap cleanup_stream EXIT

  while true; do
    local status_out
    status_out=$(cli_json ci status "$run_id" 2>&1) || true
    local pipeline_status
    pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")

    # Find currently running job
    local active_job_id active_job_name
    read -r active_job_id active_job_name < <(python3 -c "
import json, sys, re, os
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    for stage in d.get('stages', []):
        for job in stage.get('jobs', []):
            if job.get('status') == 'running' and job.get('id'):
                print(job['id'], job.get('name', ''))
                sys.exit(0)
    print(' ')
    sys.exit(0)
print(' ')
" <<< "$status_out" 2>/dev/null)

    # Switch log stream to new job
    if [ -n "$active_job_id" ] && [ "$active_job_id" != "$streaming_job" ]; then
      case " $seen_jobs " in
        *" $active_job_id "*) ;;
        *)
          cleanup_stream
          stream_pid=""

          echo ""
          echo -e "${BOLD}━━━ ${active_job_name:-$active_job_id} ━━━${NC}"
          echo ""

          cli ci logs --follow "$run_id" "$active_job_id" 2>/dev/null &
          stream_pid=$!
          streaming_job="$active_job_id"
          seen_jobs="$seen_jobs $active_job_id"
          ;;
      esac
    fi

    # Check terminal state
    case "$pipeline_status" in
      success)
        cleanup_stream; stream_pid=""
        trap - EXIT
        echo ""
        print_pipeline_summary "$status_out"
        ok "Pipeline completed successfully! 🎉"
        return 0
        ;;
      failed|cancelled)
        cleanup_stream; stream_pid=""
        trap - EXIT
        echo ""
        print_pipeline_summary "$status_out"
        err "Pipeline $pipeline_status"

        # Dump VM logs for debugging
        log "Checking for VM serial logs..."
        for serial_log in /tmp/aspen-ci-*-serial.log; do
          if [ -f "$serial_log" ]; then
            echo ""
            echo -e "${BOLD}VM serial log: $(basename "$serial_log")${NC}"
            tail -30 "$serial_log"
          fi
        done
        return 1
        ;;
    esac

    sleep 2
  done
}

print_pipeline_summary() {
  local status_out="$1"
  python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    icons = {'success': '✅', 'failed': '❌', 'cancelled': '⏹️', 'running': '🔄', 'pending': '⏳'}
    for stage in d.get('stages', []):
        si = icons.get(stage.get('status', ''), '❓')
        print(f'  {si} {stage[\"name\"]}')
        for job in stage.get('jobs', []):
            ji = icons.get(job.get('status', ''), '❓')
            dur = ''
            if job.get('duration_secs'):
                dur = f' ({job[\"duration_secs\"]}s)'
            print(f'      {ji} {job[\"name\"]}{dur}')
    break
" <<< "$status_out" 2>/dev/null
  echo ""
}

# ── Verify ────────────────────────────────────────────────────────────

do_verify() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  log "Verifying CI-built binary against local binary..."

  local run_id
  run_id=$(cat "$CLUSTER_DIR/run_id" 2>/dev/null || true)

  if [ -z "$run_id" ]; then
    local list_out
    list_out=$(cli_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")
  fi

  if [ -z "$run_id" ]; then
    err "No pipeline runs found"
    exit 1
  fi

  local status_out
  status_out=$(cli_json ci status "$run_id" 2>&1) || true

  local pipeline_status
  pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")
  if [ "$pipeline_status" != "success" ]; then
    err "Pipeline $run_id is not successful (status: $pipeline_status)"
    exit 1
  fi

  # Extract build-node job ID
  local build_node_job_id
  build_node_job_id=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    for stage in d.get('stages', []):
        if stage.get('name') == 'build':
            for job in stage.get('jobs', []):
                if job.get('name') == 'build-node' and job.get('id'):
                    print(job['id'])
                    sys.exit(0)
    print('')
    sys.exit(0)
print('')
" <<< "$status_out" 2>/dev/null)

  if [ -z "$build_node_job_id" ]; then
    err "Could not find build-node job ID in pipeline $run_id"
    exit 1
  fi
  ok "Found build-node job: $build_node_job_id"

  # Read job result from KV
  log "Reading job result from KV store..."
  local job_data
  job_data=$(cli_json kv get "__jobs:$build_node_job_id" 2>&1) || true

  local output_path
  output_path=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        job_str = d.get('value', '')
        job = json.loads(job_str)
        data = job.get('result', {}).get('Success', {}).get('data', {})
        paths = data.get('output_paths', [])
        if paths:
            print(paths[0])
        else:
            for sp in data.get('uploaded_store_paths', []):
                print(sp['store_path'])
                sys.exit(0)
        print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -n "$output_path" ] && [ -f "$output_path/bin/aspen-node" ]; then
    ok "Found CI-built binary at $output_path/bin/aspen-node"
    compare_binaries "$output_path/bin/aspen-node" "$ASPEN_NODE"
    return $?
  fi

  # Try blob download as fallback
  local blob_hash
  blob_hash=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        job_str = d.get('value', '')
        job = json.loads(job_str)
        data = job.get('result', {}).get('Success', {}).get('data', {})
        for artifact in data.get('artifacts', []):
            if 'aspen-node' in artifact.get('path', ''):
                print(artifact.get('blob_hash', ''))
                sys.exit(0)
        print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -n "$blob_hash" ]; then
    ok "Artifact blob hash: $blob_hash"
    local ci_binary="/tmp/aspen-node-ci-built"
    log "Downloading CI-built binary from blob store..."
    if cli blob get "$blob_hash" -o "$ci_binary" 2>/dev/null; then
      chmod +x "$ci_binary"
      compare_binaries "$ci_binary" "$ASPEN_NODE"
      local result=$?
      rm -f "$ci_binary"
      return $result
    fi
    warn "Blob download failed"
  fi

  err "Could not locate CI-built binary"
  return 1
}

compare_binaries() {
  local ci_bin="$1"
  local local_bin="$2"

  echo ""
  echo -e "${BOLD}Binary Comparison:${NC}"
  echo "  CI-built:    $ci_bin"
  echo "  Local-built: $local_bin"
  echo ""

  local ci_size local_size
  ci_size=$(stat -c %s "$ci_bin" 2>/dev/null || echo "0")
  local_size=$(stat -c %s "$local_bin" 2>/dev/null || echo "0")
  printf "  Size:  CI = %s bytes, Local = %s bytes\n" "$ci_size" "$local_size"

  local ci_sha local_sha
  ci_sha=$(sha256sum "$ci_bin" 2>/dev/null | cut -d' ' -f1)
  local_sha=$(sha256sum "$local_bin" 2>/dev/null | cut -d' ' -f1)
  printf "  SHA256 CI:    %s\n" "$ci_sha"
  printf "  SHA256 Local: %s\n" "$local_sha"

  if [ "$ci_sha" = "$local_sha" ]; then
    ok "Binaries are identical! (bit-for-bit reproducible build)"
  else
    warn "Binaries differ (expected — different build inputs/optimization)"
    log "Smoke-testing CI-built binary..."

    # Test with --help (always works) and check exit code, not output content.
    # --version also works now, but --help is more reliable for smoke testing.
    if "$ci_bin" --help >/dev/null 2>&1; then
      ok "CI-built binary is functional (--help exits 0)"
    else
      err "CI-built binary failed to run (--help returned non-zero)"
      return 1
    fi

    # Show versions if available
    local ci_version local_version
    ci_version=$("$ci_bin" --version 2>/dev/null || echo "unknown")
    local_version=$("$local_bin" --version 2>/dev/null || echo "unknown")
    printf "  Version CI:    %s\n" "$ci_version"
    printf "  Version Local: %s\n" "$local_version"
  fi

  echo ""
  return 0
}

# ── Logs ──────────────────────────────────────────────────────────────

do_logs() {
  if [ ! -d "$CLUSTER_DIR" ]; then
    err "No cluster running"
    exit 1
  fi

  local target="${2:-node1}"
  case "$target" in
    node*)
      tail -f "$CLUSTER_DIR/${target}.log"
      ;;
    vm*)
      tail -f /tmp/aspen-ci-*-serial.log
      ;;
    *)
      echo "Usage: $0 logs {node1|node2|...|vm}"
      ;;
  esac
}

# ── Full Pipeline ─────────────────────────────────────────────────────

do_full() {
  echo -e "${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║    ASPEN SELF-HOSTED BUILD — VM-ISOLATED CI PIPELINE       ║"
  echo "║                                                            ║"
  echo "║  Cluster: local processes                                  ║"
  echo "║  CI jobs: Cloud Hypervisor microVMs                        ║"
  echo "║  Build:   nix build inside isolated VM                     ║"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
  echo ""

  # Trap to clean up on failure
  trap 'err "Pipeline failed. Cluster still running at $CLUSTER_DIR"; err "Logs: tail -f $CLUSTER_DIR/node1.log"' ERR

  do_start
  echo ""
  do_push
  echo ""
  do_build
  local result=$?
  echo ""

  trap - ERR

  if [ $result -eq 0 ]; then
    do_verify
    local verify_result=$?
    echo ""
    echo -e "${GREEN}${BOLD}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║           🎉 VM-ISOLATED SELF-BUILD SUCCEEDED 🎉           ║"
    echo "║                                                            ║"
    echo "║  Aspen built itself using its own infrastructure:          ║"
    echo "║  • Forge: hosted source code via git-remote-aspen          ║"
    echo "║  • CI: auto-triggered pipeline on push                     ║"
    echo "║  • VM: Cloud Hypervisor microVM execution                  ║"
    echo "║  • Nix: reproducible builds in sandboxed VM                ║"
    if [ $verify_result -eq 0 ]; then
    echo "║  • Verify: CI-built binary downloaded and validated        ║"
    fi
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
  fi
  return $result
}

# ── Main ──────────────────────────────────────────────────────────────

CMD="${1:-full}"

case "$CMD" in
  start)  do_start ;;
  stop)   do_stop ;;
  status) do_status ;;
  push)   do_push ;;
  build)  do_build ;;
  verify) do_verify ;;
  logs)   do_logs "$@" ;;
  full)   do_full ;;
  *)
    echo "Usage: $0 {start|stop|status|push|build|verify|logs|full}"
    echo ""
    echo "Commands:"
    echo "  full    Full pipeline: start → push → build → verify (default)"
    echo "  start   Start the Aspen cluster (with VM CI pool)"
    echo "  stop    Stop the cluster and clean up VMs"
    echo "  status  Show cluster and VM pool status"
    echo "  push    Push Aspen source to Forge"
    echo "  build   Wait for / trigger CI build (runs in VM)"
    echo "  verify  Download CI-built binary and compare"
    echo "  logs    Tail logs (node1|node2|vm)"
    echo ""
    echo "Prerequisites:"
    echo "  1. KVM: ls -la /dev/kvm"
    echo "  2. Network: sudo nix run .#setup-ci-network"
    echo ""
    echo "Environment:"
    echo "  ASPEN_NODE_COUNT       Number of cluster nodes (default: 1)"
    echo "  ASPEN_CI_VM_MEMORY_MIB VM memory in MiB (default: 24576)"
    echo "  ASPEN_CI_VM_VCPUS      VM vCPUs (default: 4)"
    exit 1
    ;;
esac
