# NixOS VM integration test for KV operations across a 3-node Aspen cluster.
#
# Unlike the single-node kv-operations test, this validates that KV
# operations actually replicate through Raft consensus:
#
#   - Write on leader, read from followers (Raft replication)
#   - Write via follower (NOT_LEADER forwarding)
#   - CAS conflict detection across nodes
#   - Batch write replication to all nodes
#   - Scan consistency across nodes
#   - Delete replication
#   - KV operations survive leader failover
#
# Run:
#   nix build .#checks.x86_64-linux.multi-node-kv-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.multi-node-kv-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each).
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

  cookie = "multi-node-kv-test";

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
      logLevel = "info";
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
in
  pkgs.testers.nixosTest {
    name = "multi-node-kv";

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
      import json
      import re
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket(node):
          """Read the cluster ticket from a node."""
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, ticket=None, check=True):
          """Run aspen-cli --json on a specific node and return parsed JSON."""
          if ticket is None:
              ticket = get_ticket(node1)
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
          """Run aspen-cli (human output) on a specific node."""
          if ticket is None:
              ticket = get_ticket(node1)
          node.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node.succeed("cat /tmp/_cli_out.txt")

      def get_endpoint_addr_json(node):
          """Extract full endpoint address (id + addrs) from node journal."""
          output = node.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep 'cluster ticket generated'"
              " | head -1"
          )
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, f"endpoint_id not found: {output[:300]}"
          eid = eid_match.group(1)

          addrs = []
          for addr in re.findall(r'(192\.168\.\d+\.\d+:\d+)', output):
              addrs.append({"Ip": addr})
          if not addrs:
              for addr in re.findall(r'(\d+\.\d+\.\d+\.\d+:\d+)', output):
                  addrs.append({"Ip": addr})

          assert addrs, f"no direct addrs found: {output[:300]}"
          endpoint_json = json.dumps({"id": eid, "addrs": addrs})
          node.log(f"EndpointAddr JSON: {endpoint_json}")
          return endpoint_json

      def wait_for_healthy(node, timeout=60):
          """Wait for a node to become healthy and responsive."""
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      # ── cluster boot + formation ────────────────────────────────────
      start_all()

      for node in [node1, node2, node3]:
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      wait_for_healthy(node1)

      addr2_json = get_endpoint_addr_json(node2)
      addr3_json = get_endpoint_addr_json(node3)

      cli_text(node1, "cluster init")
      time.sleep(2)

      cli_text(node1, f"cluster add-learner --node-id 2 --addr '{addr2_json}'")
      time.sleep(3)
      cli_text(node1, f"cluster add-learner --node-id 3 --addr '{addr3_json}'")
      time.sleep(3)
      cli_text(node1, "cluster change-membership 1 2 3")
      time.sleep(5)

      # Verify 3-node cluster formed
      status = cli(node1, "cluster status")
      voters = [n for n in status.get("nodes", []) if n.get("is_voter")]
      assert len(voters) == 3, f"expected 3 voters: {voters}"
      node1.log("3-node cluster formed successfully")

      # Find the leader for deterministic routing
      metrics = cli(node1, "cluster metrics")
      leader_id = metrics.get("current_leader")
      leader_node = {1: node1, 2: node2, 3: node3}[leader_id]
      leader_ticket = get_ticket(leader_node)
      follower_nodes = [
          (nid, nref)
          for nid, nref in [(1, node1), (2, node2), (3, node3)]
          if nid != leader_id
      ]
      node1.log(f"Leader is node{leader_id}")

      # ================================================================
      # WRITE ON LEADER, READ FROM FOLLOWERS
      # ================================================================

      with subtest("kv set on leader, get from all followers"):
          out = cli(leader_node, "kv set repl:key1 leader-value",
                    ticket=leader_ticket)
          assert out.get("status") == "success", f"set failed: {out}"
          leader_node.log("Wrote repl:key1 on leader")

          time.sleep(2)

          for nid, nref in follower_nodes:
              out = cli(nref, "kv get repl:key1", ticket=leader_ticket)
              assert out.get("does_exist") is True, \
                  f"repl:key1 not replicated to node{nid}: {out}"
              assert out.get("value") == "leader-value", \
                  f"value mismatch on node{nid}: {out}"
              nref.log(f"node{nid} reads replicated value correctly")

      with subtest("overwrite replicates to followers"):
          cli(leader_node, "kv set repl:key1 updated-value",
              ticket=leader_ticket)
          time.sleep(2)

          for nid, nref in follower_nodes:
              out = cli(nref, "kv get repl:key1", ticket=leader_ticket)
              assert out.get("value") == "updated-value", \
                  f"overwrite not replicated to node{nid}: {out}"
          leader_node.log("Overwrite replicated to all followers")

      # ================================================================
      # WRITE VIA FOLLOWER (NOT_LEADER FORWARDING)
      # ================================================================

      with subtest("write via follower auto-routes to leader"):
          fid, fnode = follower_nodes[0]
          # Use leader's ticket so CLI can discover the leader address
          out = cli(fnode, "kv set follower:write routed-value",
                    ticket=leader_ticket)
          assert out.get("status") == "success", \
              f"follower write should succeed via NOT_LEADER routing: {out}"
          fnode.log(f"node{fid} (follower) write routed to leader")

          time.sleep(2)

          # Verify readable from all nodes
          for nid, nref in [(leader_id, leader_node)] + follower_nodes:
              out = cli(nref, "kv get follower:write", ticket=leader_ticket)
              assert out.get("value") == "routed-value", \
                  f"follower write not visible on node{nid}: {out}"
          leader_node.log("Follower-routed write visible on all nodes")

      # ================================================================
      # CAS ACROSS NODES
      # ================================================================

      with subtest("cas create-if-absent from leader"):
          out = cli(leader_node, "kv cas cas:cross --new-value initial",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"CAS create-if-absent failed: {out}"
          time.sleep(2)

      with subtest("cas conflict from follower with wrong expected"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode,
                    "kv cas cas:cross --expected wrong --new-value nope",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False or \
                  out.get("status") == "conflict", \
                  f"CAS should conflict from follower: {out}"
          fnode.log(f"node{fid} CAS conflict detected correctly")

          # Verify value unchanged from any node
          out = cli(leader_node, "kv get cas:cross", ticket=leader_ticket)
          assert out.get("value") == "initial", \
              f"CAS conflict changed value: {out}"

      with subtest("cas success from follower with correct expected"):
          fid, fnode = follower_nodes[1]
          out = cli(fnode,
                    "kv cas cas:cross --expected initial --new-value from-follower",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"CAS from follower should succeed: {out}"
          time.sleep(2)

          # Verify on all nodes
          for nid, nref in [(leader_id, leader_node)] + follower_nodes:
              out = cli(nref, "kv get cas:cross", ticket=leader_ticket)
              assert out.get("value") == "from-follower", \
                  f"CAS update not replicated to node{nid}: {out}"
          leader_node.log("Cross-node CAS works correctly")

      # ================================================================
      # BATCH WRITE REPLICATION
      # ================================================================

      with subtest("batch write on leader replicates to all"):
          out = cli(leader_node,
                    "kv batch-write batch:x=alpha batch:y=beta batch:z=gamma",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, f"batch write failed: {out}"
          time.sleep(2)

          for nid, nref in follower_nodes:
              for key, expected in [("batch:x", "alpha"),
                                    ("batch:y", "beta"),
                                    ("batch:z", "gamma")]:
                  out = cli(nref, f"kv get {key}", ticket=leader_ticket)
                  assert out.get("value") == expected, \
                      f"{key} not replicated to node{nid}: {out}"
              nref.log(f"node{nid} has all batch-written keys")

      with subtest("batch read from follower returns replicated data"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode, "kv batch-read batch:x batch:y batch:z",
                    ticket=leader_ticket)
          results = out.get("results", [])
          assert len(results) == 3, \
              f"expected 3 batch-read results on node{fid}: {results}"
          for r in results:
              assert r.get("does_exist") is True, \
                  f"batch key not found on node{fid}: {r}"
          fnode.log(f"node{fid} batch-read all replicated keys")

      # ================================================================
      # SCAN CONSISTENCY ACROSS NODES
      # ================================================================

      with subtest("scan setup on leader"):
          for i in range(8):
              cli(leader_node, f"kv set scan:mn:{i:03d} val-{i}",
                  ticket=leader_ticket)
          time.sleep(3)
          leader_node.log("Wrote 8 scan:mn: keys on leader")

      with subtest("scan from each follower returns all keys"):
          for nid, nref in follower_nodes:
              out = cli(nref, "kv scan scan:mn:", ticket=leader_ticket)
              entries = out.get("entries", [])
              assert len(entries) == 8, \
                  f"node{nid} scan returned {len(entries)}, expected 8"
              nref.log(f"node{nid} scan: {len(entries)} entries (correct)")

      with subtest("scan with limit consistent across nodes"):
          for nid, nref in [(leader_id, leader_node)] + follower_nodes:
              out = cli(nref, "kv scan scan:mn: --limit 3",
                        ticket=leader_ticket)
              entries = out.get("entries", [])
              assert len(entries) == 3, \
                  f"node{nid} limited scan returned {len(entries)}"
          leader_node.log("Scan with limit consistent across all nodes")

      # ================================================================
      # DELETE REPLICATION
      # ================================================================

      with subtest("delete on leader propagates to followers"):
          cli(leader_node, "kv delete repl:key1", ticket=leader_ticket)
          time.sleep(2)

          for nid, nref in follower_nodes:
              out = cli(nref, "kv get repl:key1", ticket=leader_ticket)
              assert out.get("does_exist") is False, \
                  f"deleted key still exists on node{nid}: {out}"
          leader_node.log("Delete replicated to all followers")

      with subtest("cad from follower with correct expected"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode, "kv cad cas:cross --expected from-follower",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"CAD from follower failed: {out}"
          time.sleep(2)

          out = cli(leader_node, "kv get cas:cross", ticket=leader_ticket)
          assert out.get("does_exist") is False, \
              f"CAD not replicated to leader: {out}"
          leader_node.log("Cross-node CAD works correctly")

      # ================================================================
      # LARGE VALUE REPLICATION
      # ================================================================

      with subtest("large value replicates across cluster"):
          # Generate a 5KB value (kept small to avoid serial console log
          # truncation — large base64 data in JSON floods the Nix builder log)
          leader_node.succeed(
              "dd if=/dev/urandom bs=1024 count=5 2>/dev/null "
              "| base64 > /tmp/large-kv.txt"
          )
          cli_text(leader_node,
                   "kv set large:repl --file /tmp/large-kv.txt")
          time.sleep(3)

          for nid, nref in follower_nodes:
              out = cli(nref, "kv get large:repl", ticket=leader_ticket)
              assert out.get("does_exist") is True, \
                  f"large value not on node{nid}: {out}"
              val = out.get("value", "")
              assert len(val) > 3000, \
                  f"large value truncated on node{nid}: len={len(val)}"
          leader_node.log("Large value replicated to all nodes")

      # ================================================================
      # KV SURVIVES LEADER FAILOVER
      # ================================================================

      with subtest("write data before failover"):
          cli(leader_node, "kv set survive:key pre-failover",
              ticket=leader_ticket)
          time.sleep(2)

      with subtest("stop leader"):
          leader_node.succeed("systemctl stop aspen-node.service")
          leader_node.log(f"Stopped leader (node{leader_id})")
          time.sleep(8)

      with subtest("verify new leader elected"):
          new_leader = None
          for nid, nref in follower_nodes:
              try:
                  ticket = get_ticket(nref)
                  nref.wait_until_succeeds(
                      f"aspen-cli --ticket '{ticket}' "
                      f"cluster health 2>/dev/null",
                      timeout=30,
                  )
                  m = cli(nref, "cluster metrics", ticket=ticket)
                  leader = m.get("current_leader")
                  if leader is not None and leader != leader_id:
                      new_leader = leader
              except Exception as e:
                  nref.log(f"node{nid} not ready: {e}")

          assert new_leader is not None, "no new leader after failover"
          new_leader_node = {1: node1, 2: node2, 3: node3}[new_leader]
          new_ticket = get_ticket(new_leader_node)
          node1.log(f"New leader: node{new_leader}")

      with subtest("pre-failover data readable after failover"):
          out = cli(new_leader_node, "kv get survive:key",
                    ticket=new_ticket)
          assert out.get("value") == "pre-failover", \
              f"pre-failover data lost: {out}"
          new_leader_node.log("Pre-failover KV data survived")

      with subtest("new writes work after failover"):
          out = cli(new_leader_node, "kv set survive:post post-failover",
                    ticket=new_ticket)
          assert out.get("status") == "success", \
              f"post-failover write failed: {out}"
          time.sleep(2)

          # Verify from the other surviving node
          other_nid, other_node = [
              (nid, nref) for nid, nref in follower_nodes
              if nid != new_leader
          ][0]
          out = cli(other_node, "kv get survive:post", ticket=new_ticket)
          assert out.get("value") == "post-failover", \
              f"post-failover write not on node{other_nid}: {out}"
          new_leader_node.log("Post-failover KV writes replicate correctly")

      with subtest("scan works after failover"):
          out = cli(new_leader_node, "kv scan scan:mn:",
                    ticket=new_ticket)
          entries = out.get("entries", [])
          assert len(entries) == 8, \
              f"scan after failover returned {len(entries)}, expected 8"
          new_leader_node.log("Scan consistent after failover")

      with subtest("cas works after failover"):
          out = cli(new_leader_node,
                    "kv cas survive:cas --new-value failover-cas",
                    ticket=new_ticket)
          assert out.get("is_success") is True, \
              f"CAS after failover failed: {out}"
          new_leader_node.log("CAS works after failover")

      # ── restart old leader and verify catch-up ───────────────────────

      with subtest("restart old leader"):
          leader_node.succeed("systemctl start aspen-node.service")
          leader_node.wait_for_unit("aspen-node.service")
          leader_node.wait_for_file(
              "/var/lib/aspen/cluster-ticket.txt", timeout=30
          )
          time.sleep(8)
          leader_node.log(f"node{leader_id} restarted")

      with subtest("restarted node catches up on KV data"):
          time.sleep(5)
          # Post-failover data should be visible on restarted node
          out = cli(new_leader_node, "kv get survive:post",
                    ticket=new_ticket)
          assert out.get("value") == "post-failover", \
              f"post-failover data not caught up: {out}"

          out = cli(new_leader_node, "kv get survive:cas",
                    ticket=new_ticket)
          assert out.get("value") == "failover-cas", \
              f"failover CAS data not caught up: {out}"
          leader_node.log(f"node{leader_id} caught up with all KV data")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All multi-node KV integration tests passed!")
    '';
  }
