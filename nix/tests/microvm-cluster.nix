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
  aspen-cli-vm-test,
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
        aspen-cli-vm-test
        pkgs.python3
      ];

      networking.firewall.enable = false;
    };

    testScript = ''
      import json
      import re
      import time

      host.start()
      host.wait_for_unit("multi-user.target")

      def get_ticket(node_id):
          return host.succeed(f"cat /tmp/aspen-{node_id}/cluster-ticket.txt").strip()

      def cli_text(cmd, ticket=None):
          if ticket is None:
              ticket = get_ticket(1)
          host.succeed(f"aspen-cli --ticket '{ticket}' {cmd} >/tmp/_aspen_cli.out 2>/tmp/_aspen_cli.err")
          return host.succeed("cat /tmp/_aspen_cli.out")

      def cli_json(cmd, ticket=None):
          if ticket is None:
              ticket = get_ticket(1)
          host.succeed(f"aspen-cli --ticket '{ticket}' --json {cmd} >/tmp/_aspen_cli.json 2>/tmp/_aspen_cli.err")
          return json.loads(host.succeed("cat /tmp/_aspen_cli.json"))

      def endpoint_addr_json(unit):
          output = host.succeed(
              f"journalctl -u {unit} --no-pager 2>/dev/null | "
              "grep 'cluster ticket generated with direct addresses' | head -1"
          )
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, f"endpoint_id not found for {unit}: {output[:500]}"
          addrs = [{"Ip": addr} for addr in re.findall(r'(\d+\.\d+\.\d+\.\d+:\d+)', output)]
          assert addrs, f"no direct IPv4 addrs found for {unit}: {output[:500]}"
          return json.dumps({"id": eid_match.group(1), "addrs": addrs})

      # ════════════════════════════════════════════════════════════
      # Phase 1: Bootstrap 3-node Raft cluster
      # ════════════════════════════════════════════════════════════

      host.log("=== Starting 3-node Raft cluster ===")

      # Create data dirs
      host.succeed("mkdir -p /tmp/aspen-{1,2,3}")

      # Start three independent Aspen nodes, then form the Raft cluster
      # explicitly through the client control plane. This matches the current
      # product path: services generate tickets at startup, but membership is
      # initialized and changed by `aspen-cli cluster ...` commands.
      for i, port in [(1, 7001), (2, 7002), (3, 7003)]:
          host.succeed(
              f"systemd-run --unit=aspen-node-{i} "
              f"bash -c 'export RUST_LOG=info PATH=/run/current-system/sw/bin; "
              f"exec aspen-node "
              f"--node-id {i} "
              f"--cookie ${cookie} "
              f"--data-dir /tmp/aspen-{i} "
              f"--storage-backend inmemory "
              f"--relay-mode disabled "
              f"--disable-gossip "
              f"--disable-mdns "
              f"--bind-port {port}'"
          )
          host.wait_until_succeeds(f"test -f /tmp/aspen-{i}/cluster-ticket.txt", timeout=120)
          host.log(f"Node {i} started and wrote local ticket")

      # Verify all 3 nodes are running before control-plane formation.
      for i in [1, 2, 3]:
          status = host.succeed(f"systemctl is-active aspen-node-{i}.service || echo dead").strip()
          assert status == "active", f"Node {i} not active: {status}"
      host.log("All 3 nodes active")

      ticket = get_ticket(1)
      host.log("Cluster ticket: [REDACTED]")

      with subtest("initialize single-node cluster"):
          cli_text("cluster init", ticket=ticket)
          host.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' --json cluster status > /tmp/cluster-status.json 2>/tmp/cluster-status.err && "
              "python3 - <<'PY'\n"
              "import json\n"
              "status=json.load(open('/tmp/cluster-status.json'))\n"
              "assert len(status.get('nodes', [])) == 1, status\n"
              "PY",
              timeout=60,
          )

      with subtest("add learners and promote three voters"):
          addr2 = endpoint_addr_json("aspen-node-2")
          addr3 = endpoint_addr_json("aspen-node-3")
          cli_text(f"cluster add-learner --node-id 2 --addr '{addr2}'", ticket=ticket)
          cli_text(f"cluster add-learner --node-id 3 --addr '{addr3}'", ticket=ticket)
          cli_text("cluster change-membership 1 2 3", ticket=ticket)
          host.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' --json cluster status > /tmp/cluster-status.json 2>/tmp/cluster-status.err && "
              "python3 - <<'PY'\n"
              "import json\n"
              "status=json.load(open('/tmp/cluster-status.json'))\n"
              "voters=[node for node in status.get('nodes', []) if node.get('is_voter') is True]\n"
              "assert len(voters) == 3, status\n"
              "PY",
              timeout=120,
          )
          host.log("Promoted cluster to three voters")

      with subtest("cross-node health"):
          for i in [1, 2, 3]:
              local_ticket = get_ticket(i)
              host.wait_until_succeeds(
                  f"aspen-cli --ticket '{local_ticket}' cluster health >/tmp/health-{i}.txt 2>&1",
                  timeout=60,
              )
              host.log(f"Node {i} local health passed")

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
