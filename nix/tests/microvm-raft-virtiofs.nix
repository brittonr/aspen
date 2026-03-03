# NixOS VM integration test: Raft cluster → AspenFs VirtioFS → Cloud Hypervisor → nginx
#
# This is the ultimate end-to-end integration test for Aspen's storage stack.
# It proves the complete data path:
#
#   1. Three aspen-node processes form a Raft consensus cluster
#   2. Test data (index.html, status.json) is written to the cluster's KV store
#   3. aspen-cluster-virtiofs-server connects to the cluster and serves VirtioFS
#   4. Cloud Hypervisor launches a NixOS guest with a virtiofs mount
#   5. nginx inside the guest serves files from the VirtioFS mount
#   6. curl from the QEMU host verifies the content through the entire chain
#
# Data flow:
#   curl → TAP → CH guest → nginx → VirtioFS mount → vhost-user socket →
#   AspenFs daemon → iroh QUIC → aspen-node Raft leader → KV store
#
# Requires nested KVM.
#
# Build & run:
#   nix build .#checks.x86_64-linux.microvm-raft-virtiofs-test
{
  pkgs,
  microvm,
  aspen-node-vm-test,
  aspen-cluster-virtiofs-server,
}: let
  cookie = "raft-virtiofs-test";
  guestIp = "10.10.0.2";
  hostIp = "10.10.0.1";
  tapName = "vm-nginx";
  guestMac = "02:00:00:00:00:01";
  virtiofsTag = "aspenfs";
  virtiofsSocket = "/tmp/aspenfs.sock";

  # Build the nginx guest microVM
  nginxGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
      microvm = {
        hypervisor = "cloud-hypervisor";
        mem = 512;
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
        shares = [
          {
            source = "/tmp/aspenfs"; # not used by our daemon, required by microvm.nix
            mountPoint = "/var/www/aspen";
            tag = virtiofsTag;
            proto = "virtiofs";
            socket = virtiofsSocket;
          }
        ];
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

      services.nginx = {
        enable = true;
        virtualHosts.default = {
          default = true;
          root = "/var/www/aspen";
        };
      };

      networking = {
        hostName = "nginx-raft-guest";
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

  guestRunner = nginxGuest.config.microvm.runner.cloud-hypervisor;
in
  pkgs.testers.nixosTest {
    name = "microvm-raft-virtiofs";

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
      virtualisation.memorySize = 6144;
      virtualisation.cores = 4;

      environment.systemPackages = [
        aspen-node-vm-test
        aspen-cluster-virtiofs-server
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

      # ════════════════════════════════════════════════════════════
      # Phase 1: Bootstrap 3-node Raft cluster
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 1: Starting 3-node Raft cluster ===")
      host.succeed("mkdir -p /tmp/aspen-{1,2,3}")

      # Start node 1 (auto-bootstraps)
      host.succeed(
          "systemd-run --unit=aspen-node-1 "
          "bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          "exec aspen-node "
          "--node-id 1 "
          "--cookie ${cookie} "
          "--data-dir /tmp/aspen-1 "
          "--storage-backend inmemory "
          "--relay-mode disabled "
          "--disable-gossip "
          "--disable-mdns "
          "--bind-port 7001'"
      )
      host.log("Node 1 started")

      host.wait_until_succeeds(
          "journalctl -u aspen-node-1 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=120,
      )
      host.log("Node 1 bootstrapped")

      host.wait_until_succeeds("test -f /tmp/aspen-1/cluster-ticket.txt", timeout=10)
      ticket = host.succeed("cat /tmp/aspen-1/cluster-ticket.txt").strip()
      host.log(f"Cluster ticket: {ticket[:50]}...")

      # Start nodes 2 and 3
      for node_id in [2, 3]:
          host.succeed(
              f"systemd-run --unit=aspen-node-{node_id} "
              f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
              f"exec aspen-node "
              f"--node-id {node_id} "
              f"--cookie ${cookie} "
              f"--data-dir /tmp/aspen-{node_id} "
              f"--storage-backend inmemory "
              f"--relay-mode disabled "
              f"--disable-gossip "
              f"--disable-mdns "
              f"--bind-port 700{node_id} "
              f"--ticket {ticket}'"
          )
          host.log(f"Node {node_id} started")

      # Wait for all nodes to join
      for node_id in [2, 3]:
          host.wait_until_succeeds(
              f"journalctl -u aspen-node-{node_id} --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
              timeout=60,
          )
          host.log(f"Node {node_id} joined cluster")

      # Verify all nodes active
      for i in [1, 2, 3]:
          status = host.succeed(f"systemctl is-active aspen-node-{i}.service || echo dead").strip()
          assert status == "active", f"Node {i} not active: {status}"
      host.log("=== Phase 1 PASSED: 3-node Raft cluster running ===")

      # ════════════════════════════════════════════════════════════
      # Phase 2: AspenFs VirtioFS daemon backed by Raft cluster
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 2: Starting VirtioFS daemon (Raft-backed) ===")

      # The aspen-cluster-virtiofs-server binary:
      # 1. Connects to the cluster via ticket
      # 2. Seeds test data (index.html, status.json) into the KV store
      # 3. Starts VirtioFS daemon serving from the cluster
      host.succeed(
          "systemd-run --unit=aspenfs-virtiofs "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-cluster-virtiofs-server "
          f"--socket ${virtiofsSocket} "
          f"--ticket {ticket}'"
      )

      # Verify connection and data seeding
      host.wait_until_succeeds(
          "journalctl -u aspenfs-virtiofs --no-pager 2>/dev/null | grep -q 'test data seeded'",
          timeout=30,
      )
      host.log("Test data seeded into Raft cluster via VirtioFS daemon")

      host.wait_until_succeeds("test -S ${virtiofsSocket}", timeout=10)
      host.log("=== Phase 2 PASSED: VirtioFS daemon connected to Raft cluster ===")

      # ════════════════════════════════════════════════════════════
      # Phase 3: Launch CH microVM with nginx + VirtioFS mount
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 3: Launching Cloud Hypervisor microVM ===")

      # Create TAP for guest networking
      host.succeed("ip tuntap add ${tapName} mode tap")
      host.succeed("ip addr add ${hostIp}/24 dev ${tapName}")
      host.succeed("ip link set ${tapName} up")

      # Launch CH microVM
      host.succeed(
          "systemd-run --unit=microvm-nginx "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "microvm-run"
      )
      host.log("CH microVM launched")

      # Wait for nginx to respond (guest boot + VirtioFS mount + nginx start)
      # Each FS operation goes: guest → VirtioFS → vhost-user → AspenFs → iroh → Raft
      host.wait_until_succeeds(
          "curl -sf --connect-timeout 2 http://${guestIp}/index.html",
          timeout=180,
      )
      host.log("=== Phase 3 PASSED: nginx responding in CH microVM ===")

      # ════════════════════════════════════════════════════════════
      # Phase 4: Verify end-to-end data path
      # ════════════════════════════════════════════════════════════

      host.log("=== Phase 4: Verifying data served from Raft cluster ===")

      # Verify index.html content (written by VirtioFS server to Raft cluster)
      output = host.succeed("curl -sf http://${guestIp}/index.html")
      assert "hello from aspen raft cluster" in output, f"wrong index.html: {output!r}"
      host.log(f"index.html: {output.strip()}")

      # Verify status.json
      status = host.succeed("curl -sf http://${guestIp}/status.json")
      assert '"source":"aspen-raft"' in status, f"wrong status.json: {status!r}"
      host.log(f"status.json: {status.strip()}")

      # Verify nginx headers
      headers = host.succeed("curl -sfI http://${guestIp}/index.html")
      assert "200" in headers, f"expected 200: {headers!r}"
      assert "nginx" in headers.lower(), f"expected nginx: {headers!r}"
      host.log("HTTP headers OK")

      # ════════════════════════════════════════════════════════════
      # Cleanup
      # ════════════════════════════════════════════════════════════

      host.succeed("systemctl stop microvm-nginx.service 2>/dev/null || true")
      host.succeed("systemctl stop aspenfs-virtiofs.service 2>/dev/null || true")
      for i in [3, 2, 1]:
          host.succeed(f"systemctl stop aspen-node-{i}.service 2>/dev/null || true")
      time.sleep(1)

      host.log("=== ALL PHASES PASSED ===")
      host.log("  Phase 1: 3-node Raft cluster bootstrapped and running")
      host.log("  Phase 2: VirtioFS daemon connected, data seeded via Raft consensus")
      host.log("  Phase 3: CH microVM booted with VirtioFS mount, nginx responding")
      host.log("  Phase 4: End-to-end data path verified (Raft → VirtioFS → nginx → curl)")
    '';
  }
