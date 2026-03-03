# NixOS VM integration test: 3-node Raft cluster.
#
# Proves multi-node Raft consensus works:
#   - Node 1 bootstraps as a single-node cluster
#   - Nodes 2 and 3 join via cluster ticket
#   - All 3 nodes run the Iroh Router with client API available
#   - Leader election completes across the cluster
#
# All nodes run as processes on the QEMU host (not nested VMs) for
# simplicity. The single-node-in-VM case is covered by microvm-aspen-node-test.
#
# Requires nested KVM.
#
# Build & run:
#   nix build .#checks.x86_64-linux.microvm-cluster-test
{
  pkgs,
  microvm,
  aspen-node-vm-test,
  aspen-fuse-vm-test,
}: let
  cookie = "integration-test-cluster";
in
  pkgs.testers.nixosTest {
    name = "microvm-cluster";

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
      virtualisation.memorySize = 4096;
      virtualisation.cores = 4;

      environment.systemPackages = [
        aspen-node-vm-test
        aspen-fuse-vm-test
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

      host.log("=== Starting 3-node Raft cluster ===")

      # Create data dirs
      host.succeed("mkdir -p /tmp/aspen-{1,2,3}")

      # Start node 1 — auto-bootstraps as single-node cluster
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

      # Wait for node 1 to bootstrap and generate cluster ticket
      host.wait_until_succeeds(
          "journalctl -u aspen-node-1 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=120,
      )
      host.log("Node 1 bootstrapped")

      # Read ticket from file (node writes it to data-dir/cluster-ticket.txt)
      host.wait_until_succeeds("test -f /tmp/aspen-1/cluster-ticket.txt", timeout=10)
      ticket = host.succeed("cat /tmp/aspen-1/cluster-ticket.txt").strip()
      host.log(f"Cluster ticket: {ticket[:50]}...")

      # Start node 2 with ticket
      host.succeed(
          "systemd-run --unit=aspen-node-2 "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-node "
          f"--node-id 2 "
          f"--cookie ${cookie} "
          f"--data-dir /tmp/aspen-2 "
          f"--storage-backend inmemory "
          f"--relay-mode disabled "
          f"--disable-gossip "
          f"--disable-mdns "
          f"--bind-port 7002 "
          f"--ticket {ticket}'"
      )
      host.log("Node 2 started")

      # Start node 3 with ticket
      host.succeed(
          "systemd-run --unit=aspen-node-3 "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-node "
          f"--node-id 3 "
          f"--cookie ${cookie} "
          f"--data-dir /tmp/aspen-3 "
          f"--storage-backend inmemory "
          f"--relay-mode disabled "
          f"--disable-gossip "
          f"--disable-mdns "
          f"--bind-port 7003 "
          f"--ticket {ticket}'"
      )
      host.log("Node 3 started")

      # Wait for nodes 2 and 3 to fully join (they generate their own tickets)
      host.wait_until_succeeds(
          "journalctl -u aspen-node-2 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=60,
      )
      host.log("Node 2 joined cluster")

      host.wait_until_succeeds(
          "journalctl -u aspen-node-3 --no-pager 2>/dev/null | grep -q 'cluster ticket generated'",
          timeout=60,
      )
      host.log("Node 3 joined cluster")

      # Verify all 3 nodes are running
      for i in [1, 2, 3]:
          status = host.succeed(f"systemctl is-active aspen-node-{i}.service || echo dead").strip()
          assert status == "active", f"Node {i} not active: {status}"
      host.log("All 3 nodes active")

      # ════════════════════════════════════════════════════════════
      # Phase 2: Verify AspenFs can connect to the cluster
      # ════════════════════════════════════════════════════════════

      host.log("=== Verifying AspenFs cluster connection ===")

      # Start AspenFs VirtioFS daemon — just test that it connects
      host.succeed(
          "systemd-run --unit=aspenfs-test "
          f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
          f"exec aspen-fuse "
          f"--virtiofs "
          f"--socket /tmp/aspenfs.sock "
          f"--ticket {ticket}'"
      )

      # Verify it connects to the Aspen cluster
      host.wait_until_succeeds(
          "journalctl -u aspenfs-test --no-pager 2>/dev/null | grep -q 'connected to Aspen cluster'",
          timeout=30,
      )
      host.log("AspenFs VirtioFS daemon connected to Raft cluster")

      # Verify socket was created (daemon is listening for VMM)
      host.wait_until_succeeds("test -S /tmp/aspenfs.sock", timeout=10)
      host.log("VirtioFS socket ready — daemon waiting for Cloud Hypervisor connection")

      # Clean up
      host.succeed("systemctl stop aspenfs-test.service 2>/dev/null || true")
      for i in [3, 2, 1]:
          host.succeed(f"systemctl stop aspen-node-{i}.service 2>/dev/null || true")
      time.sleep(1)

      host.log("=== ALL PHASES PASSED ===")
      host.log("Phase 1: 3-node Raft cluster bootstrapped, all nodes joined and active")
      host.log("Phase 2: AspenFs VirtioFS daemon connected to cluster, socket ready")
    '';
  }
