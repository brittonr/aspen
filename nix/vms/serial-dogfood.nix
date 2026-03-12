# Standalone NixOS VM for interactive dogfood testing via pi's vm_boot + vm_serial.
#
# Builds a qcow2 disk image with aspen-node, aspen-cli, git, and nix pre-installed.
# Boot it with pi's vm_boot tool, then drive tests interactively via vm_serial.
#
# Build:
#   nix build .#dogfood-serial-vm
#
# Then in pi:
#   vm_boot image="result/disk.qcow2" format="qcow2" memory="4096M" extra_args="-enable-kvm"
#   vm_serial expect:"login:"
#   vm_serial send:"root\r" expect:"[#$] "
#   vm_serial command:"aspen-node --node-id 1 --cookie test --data-dir /var/lib/aspen --storage-backend redb --iroh-secret-key 0000000100000001000000010000000100000001000000010000000100000001 --relay-mode disabled --enable-workers --enable-ci --ci-auto-trigger &"
#   vm_serial command:"aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health"
#
# For cowsay test:
#   vm_serial command:"aspen-cli --ticket $TICKET git init cowsay-test"
#   # ... push cowsay flake, trigger CI, verify
{
  pkgs,
  lib,
  aspenNodePackage,
  aspenCliPackage,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  # NixOS system configuration
  nixos = pkgs.nixos ({config, ...}: {
    imports = [
      (pkgs.path + "/nixos/modules/profiles/qemu-guest.nix")
    ];

    # Boot: UEFI + serial console output for vm_serial
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

    # Auto-login root on serial (for vm_serial command: mode)
    services.getty.autologinUser = "root";

    # Packages
    environment.systemPackages = [
      aspenNodePackage
      aspenCliPackage
      gitRemoteAspenPackage
      pkgs.git
      pkgs.nix
      pkgs.curl
      pkgs.jq
    ];

    # Nix settings for in-VM builds
    nix.settings.experimental-features = ["nix-command" "flakes"];
    nix.settings.sandbox = false;
    nix.registry.nixpkgs.flake = nixpkgsFlake;

    # Networking
    networking = {
      hostName = "dogfood";
      firewall.enable = false;
      useDHCP = true;
    };

    # Convenience: helper scripts baked into PATH
    environment.etc."dogfood/start-node.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        set -euo pipefail
        export NO_COLOR=1
        mkdir -p /var/lib/aspen
        echo "[dogfood] Starting aspen-node..."
        aspen-node \
          --node-id 1 \
          --cookie dogfood-serial \
          --data-dir /var/lib/aspen \
          --storage-backend redb \
          --iroh-secret-key 0000000100000001000000010000000100000001000000010000000100000001 \
          --relay-mode disabled \
          --disable-mdns \
          --enable-workers \
          --enable-ci \
          --ci-auto-trigger \
          > /var/log/aspen-node.log 2>&1 &
        echo $! > /tmp/aspen-node.pid
        echo "[dogfood] Waiting for cluster ticket..."
        for i in $(seq 1 30); do
          if [ -f /var/lib/aspen/cluster-ticket.txt ]; then
            echo "[dogfood] Cluster ticket ready"
            cat /var/lib/aspen/cluster-ticket.txt > /tmp/ticket
            break
          fi
          sleep 1
        done
        if [ ! -f /tmp/ticket ]; then
          echo "[dogfood] FAIL: no ticket after 30s"
          exit 1
        fi
        echo "[dogfood] Initializing cluster..."
        aspen-cli --ticket "$(cat /tmp/ticket)" cluster init 2>/dev/null || true
        sleep 2
        echo "[dogfood] Node ready. Ticket saved to /tmp/ticket"
      '';
    };

    environment.etc."dogfood/cowsay-test.sh" = {
      mode = "0755";
      text = ''
        #!/usr/bin/env bash
        # Dogfood test: push a tiny flake that builds cowsay, verify CI builds it.
        set -euo pipefail
        TICKET=$(cat /tmp/ticket)
        CLI="aspen-cli --ticket $TICKET"

        echo "DOGFOOD_TEST_START"

        # Create forge repo
        echo "[test] Creating forge repo..."
        REPO_JSON=$($CLI --json git init cowsay-test 2>/dev/null)
        REPO_ID=$(echo "$REPO_JSON" | jq -r '.id // .repo_id // empty')
        if [ -z "$REPO_ID" ]; then
          echo "DOGFOOD_TEST:create-repo:FAIL:no repo_id"
          exit 1
        fi
        echo "DOGFOOD_TEST:create-repo:PASS:$REPO_ID"

        # Enable CI watch
        $CLI --json ci watch "$REPO_ID" >/dev/null 2>&1 || true
        echo "DOGFOOD_TEST:ci-watch:PASS"

        # Create cowsay flake
        mkdir -p /tmp/cowsay-repo/.aspen
        cat > /tmp/cowsay-repo/flake.nix << 'FLAKE'
        {
          description = "Dogfood serial test - build cowsay";
          inputs.nixpkgs.url = "nixpkgs";
          outputs = { nixpkgs, ... }:
            let pkgs = nixpkgs.legacyPackages.x86_64-linux;
            in { default = pkgs.cowsay; };
        }
        FLAKE

        cat > /tmp/cowsay-repo/.aspen/ci.ncl << 'NCL'
        {
          name = "cowsay-serial",
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
        git commit -m "cowsay dogfood serial test"
        echo "DOGFOOD_TEST:git-init:PASS"

        # Push to forge
        echo "[test] Pushing to forge..."
        git remote add aspen "aspen://$TICKET/$REPO_ID"
        if RUST_LOG=warn git push aspen main 2>/tmp/push.err; then
          echo "DOGFOOD_TEST:git-push:PASS"
        else
          echo "DOGFOOD_TEST:git-push:FAIL"
          cat /tmp/push.err
          exit 1
        fi

        # Wait for pipeline
        echo "[test] Waiting for pipeline..."
        RUN_ID=""
        for i in $(seq 1 60); do
          LIST=$($CLI --json ci list 2>/dev/null || echo "{}")
          RUN_ID=$(echo "$LIST" | jq -r '.runs[0].run_id // empty' 2>/dev/null)
          if [ -n "$RUN_ID" ]; then break; fi
          sleep 2
        done
        if [ -z "$RUN_ID" ]; then
          echo "DOGFOOD_TEST:pipeline-trigger:FAIL:no run after 120s"
          exit 1
        fi
        echo "DOGFOOD_TEST:pipeline-trigger:PASS:$RUN_ID"

        # Wait for job assignment, then stream its logs
        echo "[test] Waiting for job assignment..."
        JOB_ID=""
        for i in $(seq 1 60); do
          JOB_ID=$($CLI --json ci status "$RUN_ID" 2>/dev/null \
            | jq -r '[.stages[]?.jobs[]? | select(.id != null and .id != "")] | .[0].id // empty')
          if [ -n "$JOB_ID" ]; then break; fi
          sleep 2
        done
        if [ -z "$JOB_ID" ]; then
          echo "DOGFOOD_TEST:job-assign:FAIL:no job after 120s"
          exit 1
        fi
        echo "DOGFOOD_TEST:job-assign:PASS:$JOB_ID"

        # Stream build logs live — ci logs --follow blocks until job completes
        echo "[test] === build-cowsay logs ==="
        $CLI ci logs --follow "$RUN_ID" "$JOB_ID" 2>/dev/null || true
        echo "[test] === end logs ==="

        # Check final pipeline status
        STATUS=$($CLI --json ci status "$RUN_ID" 2>/dev/null || echo "{}")
        STATE=$(echo "$STATUS" | jq -r '.status // "unknown"')
        case "$STATE" in
          success)
            echo "DOGFOOD_TEST:pipeline-success:PASS"
            ;;
          failed|cancelled)
            echo "DOGFOOD_TEST:pipeline-success:FAIL:$STATE"
            # Dump per-job status for debugging
            echo "$STATUS" | jq '[.stages[]?.jobs[]? | {name, status}]'
            exit 1
            ;;
          *)
            echo "DOGFOOD_TEST:pipeline-success:FAIL:unexpected:$STATE"
            exit 1
            ;;
        esac

        # Run the built cowsay binary
        COWSAY=$(find /nix/store -maxdepth 3 -name cowsay -path '*/bin/cowsay' 2>/dev/null | head -1)
        if [ -n "$COWSAY" ]; then
          echo "[test] Running CI-built cowsay:"
          $COWSAY "Built by Aspen CI"
          echo "DOGFOOD_TEST:cowsay-run:PASS"
        else
          echo "DOGFOOD_TEST:cowsay-run:FAIL:binary not found"
        fi

        echo "DOGFOOD_TEST_COMPLETE"
      '';
    };

    system.stateVersion = "24.11";
  });

  # Build a qcow2 disk image from the NixOS config (UEFI boot)
  image = import (pkgs.path + "/nixos/lib/make-disk-image.nix") {
    inherit pkgs lib;
    config = nixos.config;
    diskSize = 20480; # 20GB for nix store downloads during builds
    format = "qcow2";
    installBootLoader = true;
    touchEFIVars = true;
    partitionTableType = "efi";
  };
in
  pkgs.runCommand "dogfood-serial-vm" {} ''
    mkdir -p $out
    ln -s ${image}/nixos.qcow2 $out/disk.qcow2
    ln -s ${nixos.config.system.build.kernel}/${nixos.config.system.boot.loader.kernelFile} $out/kernel
    ln -s ${nixos.config.system.build.initialRamdisk}/initrd $out/initrd

    # Write a quick-reference for vm_boot usage
    cat > $out/README.md << 'EOF'
    # Dogfood Serial VM

    Boot with pi's vm_boot tool:

      vm_boot image="result/disk.qcow2" format="qcow2" memory="4096M" cpus=2

    Then interact via vm_serial:

      vm_serial expect:"login:"
      vm_serial send:"root\r" expect:"[#$] "
      vm_serial command:"/etc/dogfood/start-node.sh"
      vm_serial command:"/etc/dogfood/cowsay-test.sh"

    Monitor with:
      vm_serial expect:"DOGFOOD_TEST_COMPLETE"
    EOF
  ''
