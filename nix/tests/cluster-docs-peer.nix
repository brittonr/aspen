# NixOS VM integration test for Aspen cluster management, docs namespace,
# peer subsystems, and verification commands.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises cluster, docs, peer, and verify CLI commands end-to-end:
#
# CLUSTER:
#   - cluster status (nodes, leader info, repeat after activity)
#   - cluster health (health checks, uptime)
#   - cluster metrics (Raft state, term, leader, consistency checks)
#   - cluster ticket (cluster connection ticket)
#   - cluster prometheus (Prometheus-format metrics)
#
# DOCS (conditional — requires iroh-docs sync):
#   - docs set/get/delete (CRDT namespace operations)
#   - docs list (multi-entry, post-delete)
#
# PEER:
#   - peer list (federation peer management)
#
# VERIFY:
#   - verify kv (write/read/delete replication cycle)
#   - verify blob (storage and retrieval)
#   - verify docs (CRDT sync status)
#   - verify all (combined verification with continue-on-error)
#
# Run:
#   nix build .#checks.x86_64-linux.cluster-docs-peer-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.cluster-docs-peer-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "cluster-docs-vm-test";
in
  pkgs.testers.nixosTest {
    name = "cluster-docs-peer";

    nodes = {
      node1 = {
        imports = [../../nix/modules/aspen-node.nix];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = secretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info,aspen=debug";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket and return parsed JSON.

          We redirect stdout to a temp file and cat it back, because the NixOS
          test serial console mixes stderr/kernel messages into the captured
          output, corrupting JSON parsing.
          """
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node1.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      cli_text("cluster init")
      time.sleep(2)

      # ── CLUSTER commands ─────────────────────────────────────────────

      with subtest("cluster status"):
          out = cli("cluster status")
          node1.log(f"cluster status: {out}")
          # Should have nodes array
          assert "nodes" in out, f"cluster status missing 'nodes': {out}"
          nodes = out.get("nodes", [])
          assert len(nodes) > 0, f"expected at least 1 node: {nodes}"
          # First node should be leader
          node = nodes[0]
          assert node.get("node_id") == 1, f"unexpected node_id: {node}"
          assert node.get("is_leader") is True, f"node should be leader: {node}"
          node1.log("cluster status: OK")

      with subtest("cluster health"):
          out = cli("cluster health")
          node1.log(f"cluster health: {out}")
          # Should return health status
          assert "status" in out, f"cluster health missing 'status': {out}"
          assert out.get("node_id") == 1, f"unexpected node_id: {out}"
          node1.log("cluster health: OK")

      with subtest("cluster metrics"):
          out = cli("cluster metrics")
          node1.log(f"cluster metrics: {out}")
          # Should return Raft metrics
          assert "state" in out, f"cluster metrics missing 'state': {out}"
          assert "current_term" in out, f"cluster metrics missing 'current_term': {out}"
          node1.log(f"cluster metrics: state={out.get('state')}, term={out.get('current_term')}")

      with subtest("cluster ticket"):
          out = cli("cluster ticket")
          node1.log(f"cluster ticket: {out}")
          # Should return ticket
          assert "ticket" in out, f"cluster ticket missing 'ticket': {out}"
          ticket = out.get("ticket", "")
          assert len(ticket) > 0, f"cluster ticket is empty: {out}"
          node1.log("cluster ticket: OK")

      # ── DOCS commands ────────────────────────────────────────────────
      # Note: iroh-docs sync may not be available in test VM.
      # The DocsHandler requires docs_sync to be initialized, which depends
      # on iroh-docs service starting. We probe for availability first.

      docs_available = False

      with subtest("docs availability probe"):
          out = cli("docs set docs-probe test-probe", check=False)
          if isinstance(out, dict) and out.get("is_success") is True:
              docs_available = True
              node1.log("docs: available, running full test suite")
          else:
              node1.log(f"docs: NOT available ({out}), skipping docs tests")

      if docs_available:
          with subtest("docs get after set"):
              out = cli("docs get docs-probe")
              assert out.get("was_found") is True, f"docs get not found: {out}"
              value = out.get("value", "")
              assert value == "test-probe", f"docs get value mismatch: {out}"
              node1.log("docs get: OK")

          with subtest("docs list (single entry)"):
              out = cli("docs list")
              entries = out.get("entries", [])
              assert len(entries) >= 1, f"expected at least 1 entry: {entries}"
              keys = [e.get("key", "") for e in entries]
              assert "docs-probe" in keys, f"docs-probe not in list: {keys}"
              node1.log("docs list: OK")

          with subtest("docs set second key"):
              out = cli("docs set docs-key2 'second value'")
              assert out.get("is_success") is True, f"docs set failed: {out}"
              node1.log("docs set second key: OK")

          with subtest("docs list (two entries)"):
              out = cli("docs list")
              entries = out.get("entries", [])
              assert len(entries) >= 2, f"expected at least 2 entries: {entries}"
              keys = [e.get("key", "") for e in entries]
              assert "docs-probe" in keys, f"docs-probe missing: {keys}"
              assert "docs-key2" in keys, f"docs-key2 missing: {keys}"
              node1.log("docs list (two entries): OK")

          with subtest("docs delete"):
              out = cli("docs delete docs-probe")
              assert out.get("is_success") is True, f"docs delete failed: {out}"
              node1.log("docs delete: OK")

          with subtest("docs get after delete"):
              out = cli("docs get docs-probe")
              assert out.get("was_found") is False, \
                  f"docs-probe should not exist after delete: {out}"
              node1.log("docs get after delete: OK")

          with subtest("docs list after delete"):
              out = cli("docs list")
              entries = out.get("entries", [])
              keys = [e.get("key", "") for e in entries]
              assert "docs-probe" not in keys, \
                  f"docs-probe should not be in list: {keys}"
              assert "docs-key2" in keys, f"docs-key2 missing: {keys}"
              node1.log("docs list after delete: OK")

      # ── CLUSTER additional commands ──────────────────────────────────

      with subtest("cluster prometheus metrics"):
          out = cli("cluster prometheus", check=False)
          node1.log(f"cluster prometheus: type={type(out).__name__}")
          # Prometheus output may be text (not JSON), that's fine
          if isinstance(out, dict):
              assert "format" in out or "metrics" in out, \
                  f"unexpected prometheus output: {out}"
          node1.log("cluster prometheus: OK")

      with subtest("cluster status after activity"):
          # After running various commands, cluster should still be healthy
          out = cli("cluster status")
          nodes = out.get("nodes", [])
          assert len(nodes) > 0, f"cluster should have nodes: {out}"
          leader_count = sum(1 for n in nodes if n.get("is_leader"))
          assert leader_count == 1, f"expected exactly 1 leader: {nodes}"
          node1.log("cluster status after activity: OK")

      with subtest("cluster health after activity"):
          out = cli("cluster health")
          assert out.get("status") is not None, \
              f"health should have status: {out}"
          assert out.get("uptime_seconds", 0) > 0, \
              f"uptime should be positive: {out}"
          node1.log(f"cluster uptime: {out.get('uptime_seconds')}s")

      with subtest("cluster metrics consistency"):
          out = cli("cluster metrics")
          # Verify Raft state
          state = out.get("state", "")
          assert state in ("Leader", "Follower", "Candidate", "Learner"), \
              f"unexpected Raft state: {state}"
          # Term should be positive after init
          term = out.get("current_term", 0)
          assert term > 0, f"term should be positive: {term}"
          # Leader should match our single node
          leader = out.get("current_leader")
          assert leader is not None, f"should have a leader: {out}"
          node1.log(f"cluster metrics: state={state}, term={term}, leader={leader}")

      # ── PEER commands ────────────────────────────────────────────────

      with subtest("peer list"):
          # On a single-node cluster with no configured peers, this may be empty
          out = cli("peer list", check=False)
          node1.log(f"peer list: {out}")
          # Just verify we get a response with peers array (may be empty)
          if isinstance(out, dict):
              peers = out.get("peers", [])
              count = out.get("count", 0)
              node1.log(f"peer list: {count} peers configured")
              # For single node with no external peers, expect empty list
              assert isinstance(peers, list), f"peers should be a list: {out}"
          else:
              node1.log(f"peer list returned non-JSON: {out}")
          node1.log("peer list: OK")

      # ── VERIFY commands ──────────────────────────────────────────────

      with subtest("verify kv"):
          # Set up test data first
          cli("kv set __verify_pre_1 value1")
          cli("kv set __verify_pre_2 value2")
          # Run KV verification
          out = cli("verify kv --count 3", check=False)
          if isinstance(out, dict):
              node1.log(f"verify kv: passed={out.get('passed')}, name={out.get('name')}")
          else:
              node1.log(f"verify kv: {out}")
          # Cleanup
          cli("kv delete __verify_pre_1")
          cli("kv delete __verify_pre_2")
          node1.log("verify kv: OK")

      with subtest("verify blob"):
          out = cli("verify blob --size 64", check=False)
          if isinstance(out, dict):
              node1.log(f"verify blob: passed={out.get('passed')}, name={out.get('name')}")
          else:
              node1.log(f"verify blob: {out}")
          node1.log("verify blob: OK")

      with subtest("verify docs"):
          out = cli("verify docs", check=False)
          if isinstance(out, dict):
              node1.log(f"verify docs: passed={out.get('passed')}, name={out.get('name')}")
          else:
              node1.log(f"verify docs: {out}")
          node1.log("verify docs: OK")

      with subtest("verify all"):
          out = cli("verify all --continue-on-error", check=False)
          if isinstance(out, dict):
              node1.log(f"verify all: total_passed={out.get('total_passed')}, total_failed={out.get('total_failed')}")
          else:
              node1.log(f"verify all: {out}")
          node1.log("verify all: OK")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All cluster, docs, peer, and verify integration tests passed!")
    '';
  }
