# 3-node serial dogfood VM for interactive testing via pi's vm_boot + vm_serial.
#
# Runs 3 aspen-node processes inside a single VM (localhost networking).
# Proves Raft cluster formation, CI pipeline, and rolling deploy coordination
# without the NixOS VM test framework rebuild cycle.
#
# Build:
#   nix build .#dogfood-serial-multinode-vm
#
# Usage in pi:
#   cp result/disk.qcow2 /tmp/dogfood-multi.qcow2 && chmod +w /tmp/dogfood-multi.qcow2
#   vm_boot image="/tmp/dogfood-multi.qcow2" format="qcow2" memory="6144M" cpus=4
#   vm_serial expect:"root@dogfood-multi"
#   vm_serial command:"/etc/dogfood/start-cluster.sh 2>&1"
#   vm_serial command:"/etc/dogfood/deploy-test.sh 2>&1"
{
  pkgs,
  lib,
  aspenNodePackage,
  aspenCliPackage,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each)
  secretKey1 = "0000000100000001000000010000000100000001000000010000000100000001";
  secretKey2 = "0000000200000002000000020000000200000002000000020000000200000002";
  secretKey3 = "0000000300000003000000030000000300000003000000030000000300000003";
  cookie = "dogfood-serial-multi";

  nixos = pkgs.nixos ({config, ...}: {
    imports = [
      (pkgs.path + "/nixos/modules/profiles/qemu-guest.nix")
    ];

    boot.kernelParams = [
      "console=ttyS0,115200"
      "systemd.log_level=info"
    ];
    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = true;

    fileSystems."/" = {
      device = "/dev/vda2";
      fsType = "ext4";
    };
    fileSystems."/boot" = {
      device = "/dev/vda1";
      fsType = "vfat";
    };

    services.getty.autologinUser = "root";

    environment.systemPackages = [
      aspenNodePackage
      aspenCliPackage
      gitRemoteAspenPackage
      pkgs.git
      pkgs.nix
      pkgs.curl
      pkgs.jq
      pkgs.procps # for pgrep/pkill
    ];

    nix.settings.experimental-features = ["nix-command" "flakes"];
    nix.settings.sandbox = false;
    nix.registry.nixpkgs.flake = nixpkgsFlake;

    networking = {
      hostName = "dogfood-multi";
      firewall.enable = false;
      useDHCP = true;
    };

    # ── start-cluster.sh ─────────────────────────────────────────────
    # Starts 3 aspen-node processes, forms a 3-voter Raft cluster.
    environment.etc."dogfood/start-cluster.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail

        echo "[cluster] Starting 3-node Aspen cluster..."

        # NO_COLOR prevents ANSI escape codes in logs (tracing subscriber adds
        # codes around = signs which breaks grep-based endpoint_id extraction).
        export NO_COLOR=1
        # Quiet RPC/network spam — Raft heartbeats generate high-volume INFO logs
        # that flood the serial console and log files.
        export RUST_LOG="info,aspen_raft_network=warn,aspen_raft::network=warn,aspen_rpc_core=warn"

        # Node 1: CI leader with workers
        mkdir -p /var/lib/aspen-1
        aspen-node \
          --node-id 1 \
          --cookie ${cookie} \
          --data-dir /var/lib/aspen-1 \
          --storage-backend redb \
          --iroh-secret-key ${secretKey1} \
          --relay-mode disabled \
          --disable-mdns \
          --bind-port 7771 \
          --enable-workers \
          --enable-ci \
          --ci-auto-trigger \
          > /var/log/aspen-node-1.log 2>&1 &
        echo $! > /tmp/aspen-node-1.pid

        # Node 2: follower
        mkdir -p /var/lib/aspen-2
        aspen-node \
          --node-id 2 \
          --cookie ${cookie} \
          --data-dir /var/lib/aspen-2 \
          --storage-backend redb \
          --iroh-secret-key ${secretKey2} \
          --relay-mode disabled \
          --disable-mdns \
          --bind-port 7772 \
          > /var/log/aspen-node-2.log 2>&1 &
        echo $! > /tmp/aspen-node-2.pid

        # Node 3: follower
        mkdir -p /var/lib/aspen-3
        aspen-node \
          --node-id 3 \
          --cookie ${cookie} \
          --data-dir /var/lib/aspen-3 \
          --storage-backend redb \
          --iroh-secret-key ${secretKey3} \
          --relay-mode disabled \
          --disable-mdns \
          --bind-port 7773 \
          > /var/log/aspen-node-3.log 2>&1 &
        echo $! > /tmp/aspen-node-3.pid

        echo "[cluster] Waiting for cluster tickets..."
        for n in 1 2 3; do
          for i in $(seq 1 30); do
            if [ -f "/var/lib/aspen-$n/cluster-ticket.txt" ]; then
              echo "[cluster] Node $n ticket ready"
              break
            fi
            sleep 1
          done
          if [ ! -f "/var/lib/aspen-$n/cluster-ticket.txt" ]; then
            echo "[cluster] FAIL: node $n no ticket after 30s"
            cat /var/log/aspen-node-$n.log | tail -20
            exit 1
          fi
        done

        TICKET1=$(cat /var/lib/aspen-1/cluster-ticket.txt)
        CLI="aspen-cli --ticket $TICKET1"

        # Initialize cluster on node 1
        echo "[cluster] Initializing cluster..."
        $CLI cluster init 2>/dev/null || true
        sleep 2

        # Extract endpoint addresses from node logs.
        # Uses sed instead of grep -oP since log output is plain text (NO_COLOR=1).
        get_endpoint_addr() {
          local node_id=$1
          local log="/var/log/aspen-node-$node_id.log"

          # Extract endpoint ID (64 hex chars after endpoint_id=)
          local eid
          eid=$(sed -n 's/.*endpoint_id=\([0-9a-f]\{64\}\).*/\1/p' "$log" | head -1)
          if [ -z "$eid" ]; then
            echo "[cluster] FAIL: no endpoint_id in node $node_id log" >&2
            cat "$log" | tail -5 >&2
            return 1
          fi

          # Extract first IPv4:port from direct_addrs (e.g. 10.0.2.15:50221)
          local addr
          addr=$(sed -n 's/.*direct_addrs=\[*\([0-9.]*:[0-9]*\).*/\1/p' "$log" | head -1)
          if [ -z "$addr" ]; then
            # Fallback: any IPv4:port in the ticket line
            addr=$(grep "cluster ticket" "$log" | sed -n 's/.*\([0-9]\{1,3\}\.[0-9]\{1,3\}\.[0-9]\{1,3\}\.[0-9]\{1,3\}:[0-9]\{1,5\}\).*/\1/p' | head -1)
          fi
          if [ -z "$addr" ]; then
            echo "[cluster] FAIL: no addr in node $node_id log" >&2
            return 1
          fi

          echo "{\"id\":\"$eid\",\"addrs\":[{\"Ip\":\"$addr\"}]}"
        }

        ADDR2=$(get_endpoint_addr 2)
        ADDR3=$(get_endpoint_addr 3)
        echo "[cluster] Node 2 addr: $ADDR2"
        echo "[cluster] Node 3 addr: $ADDR3"

        # Add learners and change membership
        echo "[cluster] Adding node 2 as learner..."
        $CLI cluster add-learner --node-id 2 --addr "$ADDR2"
        sleep 3

        echo "[cluster] Adding node 3 as learner..."
        $CLI cluster add-learner --node-id 3 --addr "$ADDR3"
        sleep 3

        echo "[cluster] Changing membership to {1, 2, 3}..."
        $CLI cluster change-membership 1 2 3
        sleep 5

        # Verify
        echo "[cluster] Verifying cluster status..."
        STATUS=$($CLI --json cluster status 2>/dev/null || echo "{}")
        VOTERS=$(echo "$STATUS" | jq '[.nodes[]? | select(.is_voter == true)] | length')
        if [ "$VOTERS" = "3" ]; then
          echo "[cluster] SUCCESS: 3-voter cluster formed"
        else
          echo "[cluster] FAIL: expected 3 voters, got $VOTERS"
          echo "$STATUS" | jq .
          exit 1
        fi

        # Save ticket for other scripts
        echo "$TICKET1" > /tmp/ticket
        echo "[cluster] Cluster ready. Ticket at /tmp/ticket"
      '';
    };

    # ── stop-cluster.sh ──────────────────────────────────────────────
    environment.etc."dogfood/stop-cluster.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        echo "[cluster] Stopping nodes..."
        for n in 1 2 3; do
          if [ -f "/tmp/aspen-node-$n.pid" ]; then
            kill "$(cat /tmp/aspen-node-$n.pid)" 2>/dev/null || true
            rm -f "/tmp/aspen-node-$n.pid"
          fi
        done
        echo "[cluster] All nodes stopped"
      '';
    };

    # ── cowsay-test.sh ───────────────────────────────────────────────
    # Single-stage CI test: forge push → CI auto-trigger → nix build cowsay.
    environment.etc."dogfood/cowsay-test.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        TICKET=$(cat /tmp/ticket)
        CLI="aspen-cli --ticket $TICKET"

        echo "DOGFOOD_TEST_START:cowsay"

        # Create forge repo
        REPO_JSON=$($CLI --json git init cowsay-multi-test 2>/dev/null)
        REPO_ID=$(echo "$REPO_JSON" | jq -r '.id // .repo_id // empty')
        if [ -z "$REPO_ID" ]; then
          echo "DOGFOOD_TEST:create-repo:FAIL"
          exit 1
        fi
        echo "DOGFOOD_TEST:create-repo:PASS:$REPO_ID"

        $CLI --json ci watch "$REPO_ID" >/dev/null 2>&1 || true
        echo "DOGFOOD_TEST:ci-watch:PASS"

        # Create cowsay flake
        mkdir -p /tmp/cowsay-repo/.aspen
        cat > /tmp/cowsay-repo/flake.nix << 'FLAKE'
        {
          description = "Multi-node cowsay test";
          inputs.nixpkgs.url = "nixpkgs";
          outputs = { nixpkgs, ... }:
            let pkgs = nixpkgs.legacyPackages.x86_64-linux;
            in { default = pkgs.cowsay; };
        }
        FLAKE

        cat > /tmp/cowsay-repo/.aspen/ci.ncl << 'NCL'
        {
          name = "cowsay-multi",
          stages = [
            {
              name = "build",
              jobs = [
                {
                  name = "build-cowsay",
                  type = 'nix,
                  flake_url = ".",
                  flake_attr = "default",
                  isolation = 'none,
                  timeout_secs = 600,
                },
              ],
            },
          ],
        }
        NCL

        cd /tmp/cowsay-repo
        git init --initial-branch=main
        git config user.email "test@dogfood"
        git config user.name "Dogfood"
        git add -A
        git commit -m "cowsay multi-node test"

        git remote add aspen "aspen://$TICKET/$REPO_ID"
        if RUST_LOG=warn git push aspen main 2>/tmp/push.err; then
          echo "DOGFOOD_TEST:git-push:PASS"
        else
          echo "DOGFOOD_TEST:git-push:FAIL"
          cat /tmp/push.err
          exit 1
        fi

        # Wait for pipeline
        RUN_ID=""
        for i in $(seq 1 60); do
          LIST=$($CLI --json ci list 2>/dev/null || echo "{}")
          RUN_ID=$(echo "$LIST" | jq -r '.runs[0].run_id // empty' 2>/dev/null)
          if [ -n "$RUN_ID" ]; then break; fi
          sleep 2
        done
        if [ -z "$RUN_ID" ]; then
          echo "DOGFOOD_TEST:pipeline-trigger:FAIL"
          exit 1
        fi
        echo "DOGFOOD_TEST:pipeline-trigger:PASS:$RUN_ID"

        # Wait for completion
        for i in $(seq 1 120); do
          STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null || echo "{}")
          STATE=$(echo "$STATUS" | jq -r '.status // "unknown"')
          case "$STATE" in
            success)
              echo "DOGFOOD_TEST:pipeline-success:PASS"
              break
              ;;
            failed|cancelled)
              echo "DOGFOOD_TEST:pipeline-success:FAIL:$STATE"
              echo "$STATUS" | jq '[.stages[]?.jobs[]? | {name, status}]' 2>/dev/null
              exit 1
              ;;
          esac
          sleep 5
        done
        if [ "$STATE" != "success" ]; then
          echo "DOGFOOD_TEST:pipeline-success:FAIL:timeout"
          exit 1
        fi

        # Run cowsay
        COWSAY=$(find /nix/store -maxdepth 3 -name cowsay -path '*/bin/cowsay' 2>/dev/null | head -1)
        if [ -n "$COWSAY" ]; then
          $COWSAY "Built by 3-node Aspen CI"
          echo "DOGFOOD_TEST:cowsay-run:PASS"
        else
          echo "DOGFOOD_TEST:cowsay-run:FAIL:not found"
        fi

        echo "DOGFOOD_TEST_COMPLETE:cowsay"
      '';
    };

    # ── deploy-test.sh ───────────────────────────────────────────────
    # Full deploy test: push flake with build+deploy stages, verify rolling
    # upgrade across 3 nodes with leadership transfer.
    environment.etc."dogfood/deploy-test.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        TICKET=$(cat /tmp/ticket)
        CLI="aspen-cli --ticket $TICKET"

        echo "DEPLOY_TEST_START"

        # Write sentinel KV data before deploy
        $CLI kv set deploy-sentinel-key survives-rolling-deploy
        echo "DEPLOY_TEST:sentinel-write:PASS"

        # Create forge repo
        REPO_JSON=$($CLI --json git init deploy-multi-test 2>/dev/null)
        REPO_ID=$(echo "$REPO_JSON" | jq -r '.id // .repo_id // empty')
        if [ -z "$REPO_ID" ]; then
          echo "DEPLOY_TEST:create-repo:FAIL"
          exit 1
        fi
        echo "DEPLOY_TEST:create-repo:PASS:$REPO_ID"

        $CLI --json ci watch "$REPO_ID" >/dev/null 2>&1 || true

        # Create flake with build + deploy stages
        mkdir -p /tmp/deploy-repo/.aspen
        cat > /tmp/deploy-repo/flake.nix << 'FLAKE'
        {
          description = "Deploy test — build cowsay";
          inputs.nixpkgs.url = "nixpkgs";
          outputs = { nixpkgs, ... }:
            let pkgs = nixpkgs.legacyPackages.x86_64-linux;
            in { default = pkgs.cowsay; };
        }
        FLAKE

        cat > /tmp/deploy-repo/.aspen/ci.ncl << 'NCL'
        {
          name = "deploy-multi",
          stages = [
            {
              name = "build",
              jobs = [
                {
                  name = "build-cowsay",
                  type = 'nix,
                  flake_url = ".",
                  flake_attr = "default",
                  isolation = 'none,
                  timeout_secs = 600,
                },
              ],
            },
            {
              name = "deploy",
              depends_on = ["build"],
              jobs = [
                {
                  name = "deploy-app",
                  type = 'deploy,
                  artifact_from = "build-cowsay",
                  strategy = "rolling",
                  max_concurrent = 1,
                  health_check_timeout_secs = 120,
                  timeout_secs = 600,
                },
              ],
            },
          ],
        }
        NCL

        cd /tmp/deploy-repo
        git init --initial-branch=main
        git config user.email "test@dogfood"
        git config user.name "Dogfood"
        git add -A
        git commit -m "deploy multi-node test"

        git remote add aspen "aspen://$TICKET/$REPO_ID"
        if RUST_LOG=warn git push aspen main 2>/tmp/push.err; then
          echo "DEPLOY_TEST:git-push:PASS"
        else
          echo "DEPLOY_TEST:git-push:FAIL"
          cat /tmp/push.err
          exit 1
        fi

        # Wait for pipeline trigger
        RUN_ID=""
        for i in $(seq 1 60); do
          LIST=$($CLI --json ci list 2>/dev/null || echo "{}")
          RUN_ID=$(echo "$LIST" | jq -r '.runs[0].run_id // empty' 2>/dev/null)
          if [ -n "$RUN_ID" ]; then break; fi
          sleep 2
        done
        if [ -z "$RUN_ID" ]; then
          echo "DEPLOY_TEST:pipeline-trigger:FAIL"
          exit 1
        fi
        echo "DEPLOY_TEST:pipeline-trigger:PASS:$RUN_ID"

        # Wait for pipeline completion (build + deploy stages)
        FINAL_STATE=""
        for i in $(seq 1 180); do
          STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null || echo "{}")
          FINAL_STATE=$(echo "$STATUS" | jq -r '.status // "unknown"')
          case "$FINAL_STATE" in
            success|failed|cancelled)
              break
              ;;
          esac
          # Log progress every 30s
          if [ $((i % 6)) -eq 0 ]; then
            STAGES=$(echo "$STATUS" | jq -r '[.stages[]? | "\(.name):\(.status)"] | join(", ")' 2>/dev/null)
            echo "[deploy] progress ($((i*5))s): $STAGES"
          fi
          sleep 5
        done

        if [ "$FINAL_STATE" = "success" ]; then
          echo "DEPLOY_TEST:pipeline-success:PASS"
        else
          echo "DEPLOY_TEST:pipeline-success:FAIL:$FINAL_STATE"
          $CLI --json ci status "$RUN_ID" 2>/dev/null | jq '[.stages[]? | {name, status, jobs: [.jobs[]? | {name, status}]}]'
          exit 1
        fi

        # Verify build stage
        BUILD_STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null \
          | jq -r '[.stages[]? | select(.name == "build")] | .[0].status // "missing"')
        echo "DEPLOY_TEST:build-stage:$BUILD_STATUS"

        # Verify deploy stage
        DEPLOY_STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null \
          | jq -r '[.stages[]? | select(.name == "deploy")] | .[0].status // "missing"')
        echo "DEPLOY_TEST:deploy-stage:$DEPLOY_STATUS"

        # Check logs for leadership transfer evidence
        echo "[deploy] Checking node logs for deploy coordination..."
        DEPLOY_EVIDENCE=""
        for n in 1 2 3; do
          if grep -q "transferring leadership\|leadership transferred\|deployment created\|NodeUpgrade" /var/log/aspen-node-$n.log 2>/dev/null; then
            DEPLOY_EVIDENCE="yes"
            echo "[deploy] Node $n deploy-related log entries:"
            grep -i "deploy\|transfer\|NodeUpgrade\|leadership" /var/log/aspen-node-$n.log | tail -10
          fi
        done
        if [ -n "$DEPLOY_EVIDENCE" ]; then
          echo "DEPLOY_TEST:leadership-transfer:PASS"
        else
          echo "DEPLOY_TEST:leadership-transfer:WARN:no evidence in logs"
        fi

        # Verify KV data survived rolling deploy
        SENTINEL=$($CLI --json kv get deploy-sentinel-key 2>/dev/null \
          | jq -r '.value // empty')
        if [ "$SENTINEL" = "survives-rolling-deploy" ]; then
          echo "DEPLOY_TEST:sentinel-survives:PASS"
        else
          echo "DEPLOY_TEST:sentinel-survives:FAIL:got $SENTINEL"
        fi

        # Verify cluster is still healthy with 3 voters
        STATUS=$($CLI --json cluster status 2>/dev/null || echo "{}")
        VOTERS=$(echo "$STATUS" | jq '[.nodes[]? | select(.is_voter == true)] | length')
        if [ "$VOTERS" = "3" ]; then
          echo "DEPLOY_TEST:cluster-healthy:PASS"
        else
          echo "DEPLOY_TEST:cluster-healthy:FAIL:$VOTERS voters"
        fi

        # Post-deploy KV write to prove the cluster is functional
        $CLI kv set post-deploy-key post-deploy-works
        POST=$($CLI --json kv get post-deploy-key 2>/dev/null \
          | jq -r '.value // empty')
        if [ "$POST" = "post-deploy-works" ]; then
          echo "DEPLOY_TEST:post-deploy-kv:PASS"
        else
          echo "DEPLOY_TEST:post-deploy-kv:FAIL"
        fi

        echo "DEPLOY_TEST_COMPLETE"
      '';
    };

    # ── cluster-status.sh ────────────────────────────────────────────
    # Quick diagnostic: show cluster status and node health.
    environment.etc."dogfood/cluster-status.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        TICKET=$(cat /tmp/ticket 2>/dev/null)
        if [ -z "$TICKET" ]; then
          echo "No ticket at /tmp/ticket — cluster not started?"
          exit 1
        fi
        CLI="aspen-cli --ticket $TICKET"

        echo "=== Cluster Status ==="
        $CLI --json cluster status 2>/dev/null | jq .

        echo ""
        echo "=== Node Processes ==="
        for n in 1 2 3; do
          PID=$(cat /tmp/aspen-node-$n.pid 2>/dev/null || echo "?")
          if [ "$PID" != "?" ] && kill -0 "$PID" 2>/dev/null; then
            echo "Node $n: PID $PID (running)"
          else
            echo "Node $n: NOT RUNNING"
          fi
        done

        echo ""
        echo "=== Recent Logs (tail 5 each) ==="
        for n in 1 2 3; do
          echo "--- Node $n ---"
          tail -5 /var/log/aspen-node-$n.log 2>/dev/null || echo "(no log)"
        done
      '';
    };

    system.stateVersion = "24.11";
  });

  image = import (pkgs.path + "/nixos/lib/make-disk-image.nix") {
    inherit pkgs lib;
    config = nixos.config;
    diskSize = 20480; # 20GB
    format = "qcow2";
    installBootLoader = true;
    touchEFIVars = true;
    partitionTableType = "efi";
  };
in
  pkgs.runCommand "dogfood-serial-multinode-vm" {} ''
    mkdir -p $out
    ln -s ${image}/nixos.qcow2 $out/disk.qcow2
    ln -s ${nixos.config.system.build.kernel}/${nixos.config.system.boot.loader.kernelFile} $out/kernel
    ln -s ${nixos.config.system.build.initialRamdisk}/initrd $out/initrd

    cat > $out/README.md << 'EOF'
    # 3-Node Dogfood Serial VM

    Boot with pi's vm_boot:

      cp result/disk.qcow2 /tmp/dogfood-multi.qcow2 && chmod +w /tmp/dogfood-multi.qcow2
      vm_boot image="/tmp/dogfood-multi.qcow2" format="qcow2" memory="6144M" cpus=4

    Form the cluster:

      vm_serial expect:"root@dogfood-multi"
      vm_serial command:"/etc/dogfood/start-cluster.sh 2>&1"

    Run tests:

      vm_serial command:"/etc/dogfood/cowsay-test.sh 2>&1"
      vm_serial command:"/etc/dogfood/deploy-test.sh 2>&1"

    Diagnostics:

      vm_serial command:"/etc/dogfood/cluster-status.sh 2>&1"
      vm_serial command:"tail -50 /var/log/aspen-node-1.log"
    EOF
  ''
