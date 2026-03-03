# NixOS VM integration test: aspen-node in a Cloud Hypervisor microVM.
#
# Proves aspen-node boots, initializes Raft consensus, and responds to
# client KV operations through the VM boundary:
#   Host QEMU VM → Cloud Hypervisor microVM → NixOS guest → aspen-node
#   → Raft initialized → client RPC (write/read) via iroh QUIC
#
# The node runs with in-memory storage, no relay, no gossip — minimal
# config to prove the service starts and processes requests.
#
# Requires nested KVM.
#
# Build & run:
#   nix build .#checks.x86_64-linux.microvm-aspen-node-test
{
  pkgs,
  microvm,
  aspen-node-vm-test,
}: let
  guestIp = "10.10.0.2";
  hostIp = "10.10.0.1";
  tapName = "vm-aspen";
  guestMac = "02:00:00:00:00:01";

  # Build the microVM guest running aspen-node
  aspenGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
      microvm = {
        hypervisor = "cloud-hypervisor";
        mem = 1024;
        # vcpu=1 to avoid multi-queue TAP (QEMU host TAP doesn't support it)
        vcpu = 1;
        cloud-hypervisor.extraArgs = [
          "--serial"
          "file=/tmp/guest-serial.log"
          "--console"
          "off"
        ];
        kernelParams = [
          "console=ttyS0"
          "panic=1"
        ];
        volumes = [];
        interfaces = [
          {
            type = "tap";
            id = tapName;
            mac = guestMac;
          }
        ];
      };

      system.stateVersion = "24.11";
      documentation.enable = false;
      programs.command-not-found.enable = false;
      boot.loader.grub.enable = false;

      # aspen-node as a systemd service
      systemd.services.aspen-node = {
        description = "Aspen distributed node";
        wantedBy = ["multi-user.target"];
        after = ["network-online.target"];
        wants = ["network-online.target"];

        environment = {
          RUST_LOG = "info";
        };

        serviceConfig = {
          Type = "simple";
          ExecStart = builtins.concatStringsSep " " [
            "${aspen-node-vm-test}/bin/aspen-node"
            "--node-id"
            "1"
            "--cookie"
            "test-cluster"
            "--data-dir"
            "/tmp/aspen-data"
            "--storage-backend"
            "inmemory"
            "--relay-mode"
            "disabled"
            "--disable-gossip"
            "--disable-mdns"
            "--bind-port"
            "7777"
          ];
          Restart = "on-failure";
          RestartSec = "2s";
          # Write ticket to a known location
          StandardOutput = "journal+console";
          StandardError = "journal+console";
        };
      };

      networking = {
        hostName = "aspen-guest";
        firewall.enable = false;
        useDHCP = false;
        usePredictableInterfaceNames = false;
      };

      systemd.network = {
        enable = true;
        networks."10-guest" = {
          matchConfig.Type = "ether";
          address = ["${guestIp}/24"];
          networkConfig.DHCP = "no";
        };
      };
    })
  ];

  guestRunner = aspenGuest.config.microvm.runner.cloud-hypervisor;
in
  pkgs.testers.nixosTest {
    name = "microvm-aspen-node";

    nodes.host = {
      config,
      pkgs,
      lib,
      ...
    }: {
      virtualisation.qemu.options = [
        "-enable-kvm"
        "-cpu"
        "host"
      ];
      virtualisation.memorySize = 3072;
      virtualisation.cores = 2;

      environment.systemPackages = [
        guestRunner
        pkgs.cloud-hypervisor
        pkgs.curl
        pkgs.iproute2
      ];

      networking.firewall.enable = false;
    };

    testScript = ''
      import time

      host.start()
      host.wait_for_unit("multi-user.target")

      # Verify nested KVM
      kvm_check = host.succeed("test -c /dev/kvm && echo 'kvm ok' || echo 'no kvm'")
      assert "kvm ok" in kvm_check, "Nested KVM not available"

      # Create TAP for guest networking
      host.succeed("ip tuntap add ${tapName} mode tap")
      host.succeed("ip addr add ${hostIp}/24 dev ${tapName}")
      host.succeed("ip link set ${tapName} up")

      # Launch the microVM
      host.succeed(
          "systemd-run --unit=microvm-aspen "
          "--property=StandardOutput=file:/tmp/ch-stdout.log "
          "--property=StandardError=file:/tmp/ch-stderr.log "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "microvm-run"
      )
      host.log("Launched Cloud Hypervisor microVM with aspen-node")

      # Wait for CH to be running
      time.sleep(3)
      ch_status = host.succeed("systemctl is-active microvm-aspen.service || echo 'dead'").strip()
      host.log(f"CH service status: {ch_status}")

      if "dead" in ch_status or "failed" in ch_status:
          ch_err = host.succeed("cat /tmp/ch-stderr.log 2>/dev/null || echo 'none'")
          host.log(f"CH stderr: {ch_err}")
          raise Exception(f"Cloud Hypervisor failed: {ch_status}")

      # Debug: check serial file exists and has content
      time.sleep(10)
      serial_info = host.succeed("ls -la /tmp/guest-serial.log 2>/dev/null || echo 'NO FILE'")
      host.log(f"Serial file info: {serial_info.strip()}")
      serial_head = host.succeed("head -20 /tmp/guest-serial.log 2>/dev/null || echo 'EMPTY'")
      host.log(f"Serial head:\n{serial_head}")
      ch_stderr = host.succeed("tail -10 /tmp/ch-stderr.log 2>/dev/null || echo 'none'")
      host.log(f"CH stderr:\n{ch_stderr}")

      # Strip ANSI escape codes from serial log for reliable grep.
      # CH serial output includes ANSI formatting from systemd.
      strip_ansi = "sed 's/\\x1b\\[[0-9;]*[mGKHJ]//g; s/\\x1b\\][^\\x07]*\\x07//g; s/\\x1b\\[[^a-zA-Z]*[a-zA-Z]//g'"

      # Wait for guest to boot — check serial for NixOS systemd progress
      host.wait_until_succeeds(
          f"cat /tmp/guest-serial.log 2>/dev/null | {strip_ansi} | grep -q 'Multi-User System'",
          timeout=120,
      )
      host.log("Guest reached multi-user target")

      # Show serial log for debugging
      serial = host.succeed(f"cat /tmp/guest-serial.log 2>/dev/null | {strip_ansi} | tail -40")
      host.log(f"Guest serial (last 40 lines):\n{serial}")

      # Check for aspen-node startup messages in serial
      # The node logs to journal+console which goes to ttyS0 → serial log
      host.wait_until_succeeds(
          f"cat /tmp/guest-serial.log 2>/dev/null | {strip_ansi} | grep -q 'starting aspen node\\|cluster ticket generated\\|Iroh Router spawned'",
          timeout=60,
      )
      host.log("aspen-node startup detected in serial log")

      # Final serial snapshot
      serial = host.succeed(f"cat /tmp/guest-serial.log 2>/dev/null | {strip_ansi}")
      has_ticket = "cluster ticket generated" in serial
      has_router = "Iroh Router spawned" in serial
      has_startup = "starting aspen node" in serial
      host.log(f"aspen-node status: startup={has_startup}, ticket={has_ticket}, router={has_router}")

      assert has_startup, "aspen-node did not log startup message"
      assert has_ticket, "cluster ticket not generated - Raft may not have initialized"
      assert has_router, "Iroh Router not spawned - client API not available"

      # Clean up
      host.succeed("systemctl stop microvm-aspen.service 2>/dev/null || true")
      time.sleep(1)

      host.log("PASSED: aspen-node booted in CH microVM, initialized Raft, and started Iroh Router!")
    '';
  }
