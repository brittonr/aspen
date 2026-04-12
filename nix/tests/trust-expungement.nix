# NixOS VM integration test for trust domain node expungement.
#
# Tests two expungement scenarios:
#
#   6.4 — Direct expungement: 3-node cluster → expunge node 3 → verify node 3
#         can't restart (expunged marker blocks startup) → wipe data_dir → re-add
#         as fresh member.
#
#   6.5 — Peer enforcement: 3-node cluster → stop node 3 → expunge node 3 while
#         it's down (so it never receives the notification) → restart node 3 →
#         verify it gets expunged via peer enforcement when it tries to
#         communicate with cluster peers.
#
# Run:
#   nix build .#checks.x86_64-linux.trust-expungement-test --option sandbox false
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.trust-expungement-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";
  cookie = "trust-expunge-test";

  mkNodeConfig = {
    nodeId,
    secretKey,
  }: {
    imports = [../../nix/modules/aspen-node.nix];

    services.aspen.node = {
      enable = true;
      package = aspenNodePackage;
      inherit nodeId cookie secretKey;
      storageBackend = "redb";
      dataDir = "/var/lib/aspen";
      logLevel = "info,aspen=debug";
      relayMode = "disabled";
      enableWorkers = false;
      enableCi = false;
      enableSnix = false;
      features = [];
    };

    environment.systemPackages = [aspenCliPackage pkgs.jq];
    networking.firewall.enable = false;

    virtualisation = {
      memorySize = 2048;
      cores = 2;
    };
  };
in
  pkgs.testers.nixosTest {
    name = "trust-expungement";
    skipLint = true;
    skipTypeCheck = true;

    nodes = {
      node1 = mkNodeConfig {
        nodeId = 1;
        secretKey = secretKey1;
      };
      node2 = mkNodeConfig {
        nodeId = 2;
        secretKey = secretKey2;
      };
      node3 = mkNodeConfig {
        nodeId = 3;
        secretKey = secretKey3;
      };
    };

    testScript = ''
      import json, re, time

      start_all()
      node1.wait_for_unit("aspen-node.service")
      node2.wait_for_unit("aspen-node.service")
      node3.wait_for_unit("aspen-node.service")

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket(node):
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, ticket=None, check=True):
          if ticket is None:
              ticket = get_ticket(node)
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node.succeed(run)
          else:
              node.execute(run)
          raw = node.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(node, cmd, ticket=None):
          if ticket is None:
              ticket = get_ticket(node)
          return node.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} 2>/dev/null"
          ).strip()

      def get_endpoint_addr_json(node):
          output = node.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep 'cluster ticket generated'"
              " | tail -1"
          )
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, f"endpoint_id not found: {output[:300]}"
          eid = eid_match.group(1)
          addrs = []
          addr_match = re.search(r'direct_addrs=\[(.*?)\]', output)
          if addr_match:
              for a in re.findall(r'\d+\.\d+\.\d+\.\d+:\d+', addr_match.group(1)):
                  addrs.append(a)
          assert len(addrs) > 0, f"no IPv4 addresses found"
          return json.dumps({"id": eid, "addrs": [{"Ip": a} for a in addrs]})

      def wait_for_healthy(node, timeout=60):
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      # ── form 3-node cluster ─────────────────────────────────────────

      with subtest("form 3-node cluster"):
          node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          wait_for_healthy(node1)

          addr2_json = get_endpoint_addr_json(node2)
          addr3_json = get_endpoint_addr_json(node3)
          node1.log(f"node2 addr: {addr2_json}")
          node1.log(f"node3 addr: {addr3_json}")

          cli_text(node1, "cluster init")

          node1.wait_until_succeeds(
              f"aspen-cli --ticket '{get_ticket(node1)}'"
              f" cluster add-learner --node-id 2 --addr '{addr2_json}'"
              f" 2>/dev/null",
              timeout=30,
          )
          node1.wait_until_succeeds(
              f"aspen-cli --ticket '{get_ticket(node1)}'"
              f" cluster add-learner --node-id 3 --addr '{addr3_json}'"
              f" 2>/dev/null",
              timeout=30,
          )

          node1.wait_until_succeeds(
              f"aspen-cli --ticket '{get_ticket(node1)}'"
              f" cluster change-membership 1 2 3"
              f" 2>/dev/null",
              timeout=30,
          )

          for n in [node1, node2, node3]:
              wait_for_healthy(n, timeout=30)

          # Write some data to verify cluster is working
          ticket1 = get_ticket(node1)
          cli(node1, "kv set trust-test-key trust-test-value", ticket=ticket1)
          result = cli(node1, "kv get trust-test-key", ticket=ticket1)
          assert result.get("does_exist") is True, f"KV not written: {result}"
          node1.log("3-node cluster formed and verified")

      # ================================================================
      # 6.4 — Direct expungement flow
      # ================================================================

      with subtest("6.4: expunge node 3 from cluster"):
          ticket1 = get_ticket(node1)

          # Expunge node 3 via CLI on node 1 (the leader)
          result = cli(
              node1,
              "cluster expunge --node-id 3 --confirm",
              ticket=ticket1,
          )
          node1.log(f"expunge result: {result}")
          assert result.get("is_success") is True, f"expunge failed: {result}"
          assert result.get("node_id") == 3, f"wrong node_id: {result}"
          node1.log("node 3 expunged from cluster")

          # Give the trust reconfiguration time to propagate
          time.sleep(5)

      with subtest("6.4: verify node 3 detects expungement"):
          # Node 3 should either:
          # (a) have received the expungement notification and shut down, or
          # (b) have its service fail on next restart due to the marker.
          #
          # Check journal for expungement indicators.
          # The node may still be running (it was running when the expunge
          # happened), so stop it first, then try to restart.
          node3.succeed("systemctl stop aspen-node.service || true")
          time.sleep(2)

          # Try to restart node 3 — it should fail because the expunged marker
          # prevents Raft initialization.
          node3.succeed("systemctl start aspen-node.service || true")
          time.sleep(5)

          # Check if the service crashed with the expungement error
          journal = node3.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
          )
          node3.log(f"node3 journal length: {len(journal)} chars")

          # The service should have failed with the expungement message
          expunged = "PERMANENTLY EXPUNGED" in journal or "permanently expunged" in journal.lower()
          if expunged:
              node3.log("PASS: node 3 detected expungement marker on restart")
          else:
              # Also check if the service is in a failed state
              status = node3.execute("systemctl is-active aspen-node.service")[1].strip()
              node3.log(f"node3 service status: {status}")
              # If the node received the expungement notification while running,
              # the AtomicBool flag would have been set. On restart, the redb
              # expunged marker should block startup.
              assert expunged, (
                  f"Expected expungement message in journal. "
                  f"Service status: {status}. "
                  f"Journal tail: {journal[-500:]}"
              )

      with subtest("6.4: wipe data dir and re-add as fresh member"):
          # Stop node 3 (may already be stopped/failed)
          node3.succeed("systemctl stop aspen-node.service || true")
          time.sleep(1)

          # Wipe the data directory — factory reset
          node3.succeed("rm -rf /var/lib/aspen/*")
          node3.log("node 3 data directory wiped")

          # Restart node 3 — it should start fresh
          node3.succeed("systemctl start aspen-node.service")
          node3.wait_for_unit("aspen-node.service")
          node3.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          node3.log("node 3 restarted with clean state")

          # Get node 3's new endpoint address
          time.sleep(3)
          addr3_new = get_endpoint_addr_json(node3)
          node3.log(f"node 3 new address: {addr3_new}")

          # Re-add node 3 as a learner, then promote to voter
          ticket1 = get_ticket(node1)
          node1.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket1}'"
              f" cluster add-learner --node-id 3 --addr '{addr3_new}'"
              f" 2>/dev/null",
              timeout=30,
          )
          node1.log("node 3 re-added as learner")

          node1.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket1}'"
              f" cluster change-membership 1 2 3"
              f" 2>/dev/null",
              timeout=30,
          )
          node1.log("node 3 promoted back to voter")

          # Verify node 3 can read data
          wait_for_healthy(node3, timeout=60)
          ticket3 = get_ticket(node3)
          result = cli(node3, "kv get trust-test-key", ticket=ticket3)
          assert result.get("does_exist") is True, (
              f"node 3 cannot read replicated data: {result}"
          )
          assert result.get("value") == "trust-test-value", (
              f"wrong value on rejoined node: {result}"
          )
          node3.log("PASS: node 3 successfully rejoined as fresh member")

      # ================================================================
      # 6.5 — Peer enforcement (offline expungement)
      # ================================================================
      #
      # For this test, we expunge node 2 while it is stopped, so it never
      # receives the expungement notification. When it restarts and tries
      # to communicate with the cluster, peer enforcement rejects it.

      with subtest("6.5: stop node 2 before expungement"):
          # Verify node 2 is healthy first
          wait_for_healthy(node2, timeout=30)
          node2.log("node 2 healthy before shutdown")

          # Stop node 2 — it will miss the expungement notification
          node2.succeed("systemctl stop aspen-node.service")
          time.sleep(2)
          node2.log("node 2 stopped (will miss expungement)")

      with subtest("6.5: expunge node 2 while it is offline"):
          ticket1 = get_ticket(node1)

          # Expunge node 2 from the cluster while it's down
          result = cli(
              node1,
              "cluster expunge --node-id 2 --confirm",
              ticket=ticket1,
          )
          node1.log(f"expunge node 2 result: {result}")
          assert result.get("is_success") is True, f"expunge failed: {result}"

          # The trust reconfiguration will fire-and-forget the notification,
          # but since node 2 is down, it will fail silently.
          time.sleep(5)
          node1.log("node 2 expunged while offline")

          # Verify cluster is healthy with just nodes 1 and 3
          m = cli(node1, "cluster metrics", ticket=ticket1)
          node1.log(f"cluster metrics after expunge: {m}")

      with subtest("6.5: restart node 2 and verify peer enforcement"):
          # Restart node 2 — it doesn't know it's been expunged
          node2.succeed("systemctl start aspen-node.service")
          node2.wait_for_unit("aspen-node.service")
          node2.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          node2.log("node 2 restarted (doesn't know it's expunged)")

          # Node 2 will try to participate in the cluster. When it contacts
          # any peer for trust operations, the peer will respond with
          # TrustResponse::Expunged, triggering on_expunged() which marks
          # the node as permanently expunged.
          #
          # Wait for the expungement to be detected via peer enforcement.
          # This happens when:
          # 1. Node 2 tries a trust share request → gets Expunged response
          # 2. The trust reconfig watcher processes the error
          # 3. handle_peer_expungement() writes the expunged marker to redb
          #
          # After that, the node should stop accepting connections (AtomicBool)
          # and on next restart, refuse to start.

          # Wait for expungement indicators in the journal
          node2.wait_until_succeeds(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep -i 'expunged'",
              timeout=120,
          )
          node2.log("node 2 detected expungement via peer enforcement")

          # Stop and try to restart — should fail with expunged marker
          node2.succeed("systemctl stop aspen-node.service")
          time.sleep(2)
          node2.succeed("systemctl start aspen-node.service || true")
          time.sleep(5)

          journal2 = node2.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
          )
          expunged = "PERMANENTLY EXPUNGED" in journal2 or "permanently expunged" in journal2.lower()
          if expunged:
              node2.log("PASS: node 2 blocked from restarting after peer enforcement")
          else:
              # Check service status
              status = node2.execute("systemctl is-active aspen-node.service")[1].strip()
              node2.log(f"node 2 service status: {status}")
              node2.log(f"journal tail: {journal2[-1000:]}")
              assert expunged, (
                  f"Expected expungement marker after peer enforcement. "
                  f"Service status: {status}"
              )

      # ── done ─────────────────────────────────────────────────────────
      node1.log("=== Trust expungement tests PASSED ===")
    '';
  }
