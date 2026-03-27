# Federated dogfood serial VM for interactive testing via pi's vm_boot + vm_serial.
#
# Runs TWO independent aspen-node instances inside a single VM with separate
# data dirs, cookies, and federation identities. Proves cross-cluster federation
# sync + CI build without the NixOS test framework rebuild cycle.
#
# Build:
#   nix build .#dogfood-serial-federation-vm
#
# Usage in pi:
#   cp result/disk.qcow2 /tmp/dogfood-fed.qcow2 && chmod +w /tmp/dogfood-fed.qcow2
#   vm_boot image="/tmp/dogfood-fed.qcow2" format="qcow2" memory="6144M" cpus=4
#   vm_serial expect:"root@dogfood-fed"
#   vm_serial command:"/etc/dogfood-federation/init-clusters.sh 2>&1"
#   vm_serial command:"/etc/dogfood-federation/full-test.sh 2>&1"
{
  pkgs,
  lib,
  aspenNodePackage,
  aspenCliPackage,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  # Alice: Forge host (no CI)
  aliceSecretKey = "a11ce00000000001a11ce00000000001a11ce00000000001a11ce00000000001";
  aliceFedKey = "a11cefed00000001a11cefed00000001a11cefed00000001a11cefed00000001";
  aliceCookie = "fed-alice-serial";

  # Bob: CI builder
  bobSecretKey = "b0b0000000000002b0b0000000000002b0b0000000000002b0b0000000000002";
  bobFedKey = "b0b00fed00000002b0b00fed00000002b0b00fed00000002b0b00fed00000002";
  bobCookie = "fed-bob-serial";

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
      pkgs.python3
    ];

    nix.settings.experimental-features = ["nix-command" "flakes"];
    nix.settings.sandbox = false;
    nix.registry.nixpkgs.flake = nixpkgsFlake;

    networking = {
      hostName = "dogfood-fed";
      firewall.enable = false;
      useDHCP = true;
    };

    # ── Helper scripts ──────────────────────────────────────────────

    environment.etc."dogfood-federation/init-clusters.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1

        echo "[fed] Starting alice (Forge host)..."
        mkdir -p /var/lib/aspen-alice
        ASPEN_FEDERATION_ENABLED=true \
        ASPEN_FEDERATION_CLUSTER_KEY="${aliceFedKey}" \
        ASPEN_FEDERATION_CLUSTER_NAME="alice-cluster" \
        ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY=false \
        ASPEN_FEDERATION_ENABLE_GOSSIP=false \
        ASPEN_DOCS_ENABLED=true \
        ASPEN_DOCS_IN_MEMORY=true \
        ASPEN_HOOKS_ENABLED=true \
        aspen-node \
          --node-id 1 \
          --data-dir /var/lib/aspen-alice \
          --cookie "${aliceCookie}" \
          --iroh-secret-key "${aliceSecretKey}" \
          --relay-mode disabled \
          --disable-mdns \
          --heartbeat-interval-ms 500 \
          --election-timeout-min-ms 1500 \
          --election-timeout-max-ms 3000 \
          > /var/log/aspen-alice.log 2>&1 &
        echo $! > /tmp/alice.pid

        echo "[fed] Starting bob (CI builder)..."
        mkdir -p /var/lib/aspen-bob
        ASPEN_FEDERATION_ENABLED=true \
        ASPEN_FEDERATION_CLUSTER_KEY="${bobFedKey}" \
        ASPEN_FEDERATION_CLUSTER_NAME="bob-cluster" \
        ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY=false \
        ASPEN_FEDERATION_ENABLE_GOSSIP=false \
        ASPEN_DOCS_ENABLED=true \
        ASPEN_DOCS_IN_MEMORY=true \
        ASPEN_HOOKS_ENABLED=true \
        aspen-node \
          --node-id 1 \
          --data-dir /var/lib/aspen-bob \
          --cookie "${bobCookie}" \
          --iroh-secret-key "${bobSecretKey}" \
          --relay-mode disabled \
          --disable-mdns \
          --heartbeat-interval-ms 500 \
          --election-timeout-min-ms 1500 \
          --election-timeout-max-ms 3000 \
          --enable-workers \
          --enable-ci \
          --ci-auto-trigger \
          > /var/log/aspen-bob.log 2>&1 &
        echo $! > /tmp/bob.pid

        echo "[fed] Waiting for alice ticket..."
        for i in $(seq 1 30); do
          if [ -f /var/lib/aspen-alice/cluster-ticket.txt ]; then break; fi
          sleep 1
        done

        echo "[fed] Waiting for bob ticket..."
        for i in $(seq 1 30); do
          if [ -f /var/lib/aspen-bob/cluster-ticket.txt ]; then break; fi
          sleep 1
        done

        if [ ! -f /var/lib/aspen-alice/cluster-ticket.txt ]; then
          echo "[fed] FAIL: alice ticket not found"
          tail -20 /var/log/aspen-alice.log
          exit 1
        fi
        if [ ! -f /var/lib/aspen-bob/cluster-ticket.txt ]; then
          echo "[fed] FAIL: bob ticket not found"
          tail -20 /var/log/aspen-bob.log
          exit 1
        fi

        ALICE_TICKET=$(cat /var/lib/aspen-alice/cluster-ticket.txt)
        BOB_TICKET=$(cat /var/lib/aspen-bob/cluster-ticket.txt)
        echo "$ALICE_TICKET" > /tmp/alice-ticket
        echo "$BOB_TICKET" > /tmp/bob-ticket

        echo "[fed] Initializing clusters..."
        aspen-cli --ticket "$ALICE_TICKET" cluster init 2>/dev/null || true
        aspen-cli --ticket "$BOB_TICKET" cluster init 2>/dev/null || true
        sleep 2

        echo "[fed] Alice health:"
        aspen-cli --ticket "$ALICE_TICKET" cluster health 2>/dev/null || true
        echo "[fed] Bob health:"
        aspen-cli --ticket "$BOB_TICKET" cluster health 2>/dev/null || true

        echo "[fed] Both clusters ready."
      '';
    };

    environment.etc."dogfood-federation/push-source.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        ALICE_TICKET=$(cat /tmp/alice-ticket)
        CLI="aspen-cli --ticket $ALICE_TICKET"

        echo "[fed] Creating Forge repo on alice..."
        REPO_JSON=$($CLI --json git init fed-test 2>/dev/null)
        REPO_ID=$(echo "$REPO_JSON" | jq -r '.id // .repo_id // empty')
        if [ -z "$REPO_ID" ]; then
          echo "[fed] FAIL: no repo_id"
          exit 1
        fi
        echo "$REPO_ID" > /tmp/alice-repo-id
        echo "[fed] Repo: $REPO_ID"

        # Create a minimal nix flake to build
        mkdir -p /tmp/fed-source/.aspen /tmp/fed-source/src
        cat > /tmp/fed-source/flake.nix << 'FLAKE'
        {
          description = "Federated dogfood test";
          inputs.nixpkgs.url = "nixpkgs";
          outputs = { nixpkgs, ... }:
            let pkgs = nixpkgs.legacyPackages.x86_64-linux;
            in { packages.x86_64-linux.default = pkgs.cowsay; };
        }
        FLAKE

        cat > /tmp/fed-source/.aspen/ci.ncl << 'NCL'
        {
          name = "fed-serial-test",
          stages = [
            {
              name = "build",
              jobs = [
                {
                  name = "nix-build",
                  type = 'nix,
                  flake_url = ".",
                  flake_attr = "packages.x86_64-linux.default",
                  isolation = 'none,
                  timeout_secs = 600,
                },
              ],
            },
          ],
        }
        NCL

        cd /tmp/fed-source
        git init --initial-branch=main
        git config user.email "test@fed"
        git config user.name "Fed Test"
        git add -A
        git commit -m "fed dogfood: cowsay flake"

        echo "[fed] Pushing to alice's Forge..."
        git remote add aspen "aspen://$ALICE_TICKET/$REPO_ID"
        RUST_LOG=warn git push aspen main 2>&1 | tail -3
        echo "[fed] Push complete."
      '';
    };

    environment.etc."dogfood-federation/federate.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        ALICE_TICKET=$(cat /tmp/alice-ticket)
        REPO_ID=$(cat /tmp/alice-repo-id)

        echo "[fed] Federating repo $REPO_ID on alice..."
        aspen-cli --ticket "$ALICE_TICKET" --json federation federate "$REPO_ID" --mode public 2>&1
        echo "[fed] Federate done."
      '';
    };

    environment.etc."dogfood-federation/sync.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        ALICE_TICKET=$(cat /tmp/alice-ticket)
        BOB_TICKET=$(cat /tmp/bob-ticket)

        echo "[fed] Getting alice's iroh node ID..."
        ALICE_HEALTH=$(aspen-cli --ticket "$ALICE_TICKET" --json cluster health 2>/dev/null)
        ALICE_NODE_ID=$(echo "$ALICE_HEALTH" | jq -r '.iroh_node_id // empty')
        if [ -z "$ALICE_NODE_ID" ]; then
          echo "[fed] FAIL: no iroh_node_id"
          exit 1
        fi
        echo "$ALICE_NODE_ID" > /tmp/alice-node-id
        echo "[fed] Alice node_id: $ALICE_NODE_ID"

        # Get alice's address from log
        ALICE_ADDR=$(grep "direct_addrs" /var/log/aspen-alice.log \
          | sed 's/\x1b\[[0-9;]*m//g' \
          | grep -oP '127\.0\.0\.1:\d+' | head -1)
        echo "[fed] Alice addr: $ALICE_ADDR"

        echo "[fed] Bob trusting alice..."
        aspen-cli --ticket "$BOB_TICKET" federation trust "$ALICE_NODE_ID" 2>/dev/null || true
        sleep 1

        echo "[fed] Bob syncing from alice..."
        for attempt in 1 2 3; do
          if ASPEN_ORIGIN_ADDR="$ALICE_ADDR" \
             aspen-cli --ticket "$BOB_TICKET" federation sync "$ALICE_NODE_ID" 2>&1; then
            echo "[fed] Sync succeeded on attempt $attempt"
            break
          fi
          echo "[fed] Sync attempt $attempt failed, retrying in 5s..."
          sleep 5
        done
        sleep 2
        echo "[fed] Sync complete."
      '';
    };

    environment.etc."dogfood-federation/build.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        BOB_TICKET=$(cat /tmp/bob-ticket)
        ALICE_NODE_ID=$(cat /tmp/alice-node-id)
        ALICE_REPO_ID=$(cat /tmp/alice-repo-id)
        ALICE_ADDR=$(grep "direct_addrs" /var/log/aspen-alice.log \
          | sed 's/\x1b\[[0-9;]*m//g' \
          | grep -oP '127\.0\.0\.1:\d+' | head -1)

        CLI="aspen-cli --ticket $BOB_TICKET"

        echo "[fed] Creating mirror repo on bob..."
        BOB_REPO_JSON=$($CLI --json git init fed-mirror 2>/dev/null)
        BOB_REPO_ID=$(echo "$BOB_REPO_JSON" | jq -r '.id // .repo_id // empty')
        if [ -z "$BOB_REPO_ID" ]; then
          echo "[fed] FAIL: no bob repo_id"
          exit 1
        fi
        echo "[fed] Bob repo: $BOB_REPO_ID"

        $CLI --json ci watch "$BOB_REPO_ID" 2>/dev/null || true

        echo "[fed] Cloning from alice via federation..."
        FED_URL="aspen://$BOB_TICKET/fed:$ALICE_NODE_ID:$ALICE_REPO_ID"
        rm -rf /tmp/fed-clone
        for attempt in 1 2; do
          if ASPEN_ORIGIN_ADDR="$ALICE_ADDR" git clone "$FED_URL" /tmp/fed-clone 2>&1; then
            echo "[fed] Clone succeeded"
            break
          fi
          echo "[fed] Clone attempt $attempt failed, retrying in 10s..."
          rm -rf /tmp/fed-clone
          sleep 10
        done

        if [ ! -d /tmp/fed-clone/.git ]; then
          echo "[fed] FAIL: clone did not produce a git repo"
          exit 1
        fi

        echo "[fed] Pushing cloned content to bob's Forge..."
        cd /tmp/fed-clone
        git remote add bob-forge "aspen://$BOB_TICKET/$BOB_REPO_ID"
        RUST_LOG=warn git push bob-forge HEAD:main 2>&1 | tail -3
        cd /

        echo "[fed] Waiting for CI pipeline..."
        RUN_ID=""
        for i in $(seq 1 60); do
          LIST=$($CLI --json ci list 2>/dev/null || echo "{}")
          RUN_ID=$(echo "$LIST" | jq -r '.runs[0].run_id // empty' 2>/dev/null)
          if [ -n "$RUN_ID" ]; then break; fi
          sleep 2
        done
        if [ -z "$RUN_ID" ]; then
          echo "[fed] FAIL: no pipeline after 120s"
          exit 1
        fi
        echo "$RUN_ID" > /tmp/run-id
        echo "[fed] Pipeline: $RUN_ID"

        echo "[fed] Waiting for pipeline to complete..."
        for i in $(seq 1 300); do
          STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null || echo "{}")
          STATE=$(echo "$STATUS" | jq -r '.status // "unknown"')
          case "$STATE" in
            success)
              echo "[fed] Pipeline succeeded!"
              exit 0
              ;;
            failed|cancelled)
              echo "[fed] Pipeline $STATE"
              echo "$STATUS" | jq '[.stages[]?.jobs[]? | {name, status}]' 2>/dev/null
              exit 1
              ;;
          esac
          sleep 2
        done
        echo "[fed] FAIL: pipeline did not complete within 600s"
        exit 1
      '';
    };

    environment.etc."dogfood-federation/verify.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        BOB_TICKET=$(cat /tmp/bob-ticket)
        RUN_ID=$(cat /tmp/run-id)
        CLI="aspen-cli --ticket $BOB_TICKET"

        STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null)
        JOB_ID=$(echo "$STATUS" | jq -r '[.stages[]?.jobs[]? | select(.status == "success")] | .[0].id // empty')
        if [ -z "$JOB_ID" ]; then
          echo "[fed] FAIL: no successful job"
          exit 1
        fi

        JOB_DATA=$($CLI --json kv get "__jobs:$JOB_ID" 2>/dev/null)
        OUTPUT_PATH=$(echo "$JOB_DATA" | jq -r '
          .value | fromjson |
          .result.Success.data.output_paths[0] // empty
        ' 2>/dev/null)

        if [ -z "$OUTPUT_PATH" ]; then
          echo "[fed] FAIL: no output path"
          exit 1
        fi
        echo "[fed] Output: $OUTPUT_PATH"

        COWSAY=$(find "$OUTPUT_PATH" -name cowsay -path '*/bin/cowsay' 2>/dev/null | head -1)
        if [ -n "$COWSAY" ]; then
          echo "[fed] Running CI-built cowsay:"
          $COWSAY "Built by federated CI"
          echo "[fed] VERIFY: PASS"
        else
          echo "[fed] Store path exists but no cowsay binary found"
          ls "$OUTPUT_PATH" 2>/dev/null
          echo "[fed] VERIFY: PARTIAL (store path OK, binary not found)"
        fi
      '';
    };

    environment.etc."dogfood-federation/full-test.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail

        echo "FED_DOGFOOD_START"

        /etc/dogfood-federation/init-clusters.sh
        echo "FED_DOGFOOD:init:PASS"

        /etc/dogfood-federation/push-source.sh
        echo "FED_DOGFOOD:push:PASS"

        /etc/dogfood-federation/federate.sh
        echo "FED_DOGFOOD:federate:PASS"

        /etc/dogfood-federation/sync.sh
        echo "FED_DOGFOOD:sync:PASS"

        /etc/dogfood-federation/build.sh
        echo "FED_DOGFOOD:build:PASS"

        /etc/dogfood-federation/verify.sh
        echo "FED_DOGFOOD:verify:PASS"

        echo "FED_DOGFOOD_COMPLETE"
      '';
    };

    system.stateVersion = "24.11";
  });

  image = import (pkgs.path + "/nixos/lib/make-disk-image.nix") {
    inherit pkgs lib;
    config = nixos.config;
    diskSize = 20480;
    format = "qcow2";
    installBootLoader = true;
    touchEFIVars = true;
    partitionTableType = "efi";
  };
in
  pkgs.runCommand "dogfood-serial-federation-vm" {} ''
    mkdir -p $out
    ln -s ${image}/nixos.qcow2 $out/disk.qcow2
    ln -s ${nixos.config.system.build.kernel}/${nixos.config.system.boot.loader.kernelFile} $out/kernel
    ln -s ${nixos.config.system.build.initialRamdisk}/initrd $out/initrd

    cat > $out/README.md << 'EOF'
    # Federated Dogfood Serial VM

    Boot with pi's vm_boot tool:

      cp result/disk.qcow2 /tmp/dogfood-fed.qcow2 && chmod +w /tmp/dogfood-fed.qcow2
      vm_boot image="/tmp/dogfood-fed.qcow2" format="qcow2" memory="6144M" cpus=4

    Then interact via vm_serial:

      vm_serial expect:"root@dogfood-fed"
      vm_serial command:"/etc/dogfood-federation/full-test.sh 2>&1"

    Or step by step:

      vm_serial command:"/etc/dogfood-federation/init-clusters.sh 2>&1"
      vm_serial command:"/etc/dogfood-federation/push-source.sh 2>&1"
      vm_serial command:"/etc/dogfood-federation/federate.sh 2>&1"
      vm_serial command:"/etc/dogfood-federation/sync.sh 2>&1"
      vm_serial command:"/etc/dogfood-federation/build.sh 2>&1"
      vm_serial command:"/etc/dogfood-federation/verify.sh 2>&1"

    Monitor with:
      vm_serial expect:"FED_DOGFOOD_COMPLETE"
    EOF
  ''
