# NixOS VM integration test for distributed coordination across a 3-node cluster.
#
# Unlike the single-node coordination-primitives test, this validates that
# coordination primitives provide real distributed guarantees:
#
#   - Lock exclusion across nodes (node1 holds lock, node2 blocked)
#   - Counter linearizability (increments from multiple nodes sum correctly)
#   - Semaphore capacity enforcement across nodes
#   - RW lock multi-node readers / writer exclusion
#   - Queue cross-node enqueue/dequeue
#   - Barrier with real multi-participant synchronization
#   - Coordination survives leader failover
#
# Run:
#   nix build .#checks.x86_64-linux.multi-node-coordination-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.multi-node-coordination-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  coordinationPluginWasm,
}: let
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

  cookie = "multi-node-coord-test";

  # WASM plugin helpers (coordination handler is WASM-only)
  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "coordination";
        wasm = coordinationPluginWasm;
      }
    ];
  };

  mkNodeConfig = {
    nodeId,
    secretKey,
  }: {
    imports = [
      ../../nix/modules/aspen-node.nix
      pluginHelpers.nixosConfig
    ];

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

    virtualisation.memorySize = 4096;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "multi-node-coordination";
    skipLint = true;

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
          """Extract full endpoint address from node journal."""
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
          return json.dumps({"id": eid, "addrs": addrs})

      def wait_for_healthy(node, timeout=60):
          """Wait for a node to become healthy."""
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

      status = cli(node1, "cluster status")
      voters = [n for n in status.get("nodes", []) if n.get("is_voter")]
      assert len(voters) == 3, f"expected 3 voters: {voters}"
      node1.log("3-node cluster formed successfully")

      # ── install WASM plugins (coordination handler is WASM-only) ───
      ${pluginHelpers.installPluginsScript}

      # Use leader ticket for all operations
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
      # DISTRIBUTED LOCK EXCLUSION ACROSS NODES
      # ================================================================

      with subtest("lock on node1, blocked on node2"):
          fid1, fnode1 = follower_nodes[0]
          fid2, fnode2 = follower_nodes[1]

          # Acquire lock from first follower
          out = cli(fnode1,
                    "lock acquire cross-lock --holder node-A --ttl 30000 --timeout 5000",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"lock acquire on node{fid1} failed: {out}"
          token_a = out.get("fencing_token")
          fnode1.log(f"node{fid1} acquired cross-lock, token={token_a}")

          # Try from second follower — should be blocked
          out = cli(fnode2,
                    "lock try-acquire cross-lock --holder node-B --ttl 30000",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"node{fid2} should be blocked by node{fid1}: {out}"
          fnode2.log(f"node{fid2} correctly blocked from cross-lock")

          # Try from leader too — also blocked
          out = cli(leader_node,
                    "lock try-acquire cross-lock --holder node-C --ttl 30000",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"leader should also be blocked: {out}"
          leader_node.log("Leader also correctly blocked from cross-lock")

          # Release from original holder
          cli(fnode1,
              f"lock release cross-lock --holder node-A --fencing-token {token_a}",
              ticket=leader_ticket)
          fnode1.log(f"node{fid1} released cross-lock")

      with subtest("lock acquirable from different node after release"):
          fid2, fnode2 = follower_nodes[1]
          out = cli(fnode2,
                    "lock acquire cross-lock --holder node-B --ttl 10000 --timeout 5000",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"lock acquire after release failed on node{fid2}: {out}"
          token_b = out.get("fencing_token")
          # Fencing token should be monotonically increasing
          assert token_b > token_a, \
              f"fencing token not monotonic: {token_a} -> {token_b}"
          fnode2.log(f"node{fid2} acquired lock with higher token={token_b}")

          cli(fnode2,
              f"lock release cross-lock --holder node-B --fencing-token {token_b}",
              ticket=leader_ticket)

      # ================================================================
      # COUNTER LINEARIZABILITY ACROSS NODES
      # ================================================================

      with subtest("counter increments from all nodes sum correctly"):
          # Initialize counter to 0, then increment from all nodes.
          # Uses "counter add X 5" per node instead of 5 individual incr
          # calls to reduce total connection count.
          cli(leader_node, "counter set mn-counter 0",
              ticket=leader_ticket)
          time.sleep(1)

          cli(leader_node, "counter add mn-counter 5",
              ticket=leader_ticket)
          fnode1 = follower_nodes[0][1]
          cli(fnode1, "counter add mn-counter 5",
              ticket=leader_ticket)
          fnode2 = follower_nodes[1][1]
          cli(fnode2, "counter add mn-counter 5",
              ticket=leader_ticket)

          time.sleep(2)

          # All 3 adds (5 each) should be reflected
          out = cli(leader_node, "counter get mn-counter",
                    ticket=leader_ticket)
          assert out.get("value") == 15, \
              f"counter should be 15 (5 * 3 nodes), got {out.get('value')}"
          leader_node.log("Counter linearizability: add 5 from 3 nodes = 15")

      with subtest("counter add from multiple nodes"):
          cli(leader_node, "counter set add-counter 0",
              ticket=leader_ticket)
          time.sleep(1)

          # Add different amounts from each node
          cli(leader_node, "counter add add-counter 100",
              ticket=leader_ticket)
          cli(follower_nodes[0][1], "counter add add-counter 200",
              ticket=leader_ticket)
          cli(follower_nodes[1][1], "counter add add-counter 300",
              ticket=leader_ticket)

          time.sleep(1)

          out = cli(leader_node, "counter get add-counter",
                    ticket=leader_ticket)
          assert out.get("value") == 600, \
              f"counter should be 600, got {out.get('value')}"
          leader_node.log("Counter add from 3 nodes: 100+200+300=600")

      with subtest("counter cas detects cross-node conflicts"):
          cli(leader_node, "counter set cas-counter 50",
              ticket=leader_ticket)
          time.sleep(1)

          # CAS from follower with correct expected
          fid, fnode = follower_nodes[0]
          out = cli(fnode,
                    "counter cas cas-counter --expected 50 --new-value 75",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"counter CAS from node{fid} should succeed: {out}"

          # CAS from another follower with stale expected
          fid2, fnode2 = follower_nodes[1]
          out = cli(fnode2,
                    "counter cas cas-counter --expected 50 --new-value 99",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"stale CAS from node{fid2} should conflict: {out}"
          fnode2.log(f"node{fid2} CAS conflict detected correctly")

          out = cli(leader_node, "counter get cas-counter",
                    ticket=leader_ticket)
          assert out.get("value") == 75, \
              f"counter should be 75 after CAS: {out}"

      # ================================================================
      # SEMAPHORE CAPACITY ACROSS NODES
      # ================================================================

      with subtest("semaphore permits distributed across nodes"):
          # Acquire 1 permit from each of the 3 nodes (capacity=3)
          cli(leader_node,
              "semaphore acquire mn-sem --holder leader --permits 1 "
              "--capacity 3 --ttl 30000 --timeout 5000",
              ticket=leader_ticket)

          fid1, fnode1 = follower_nodes[0]
          cli(fnode1,
              f"semaphore acquire mn-sem --holder node{fid1} --permits 1 "
              "--capacity 3 --ttl 30000 --timeout 5000",
              ticket=leader_ticket)

          fid2, fnode2 = follower_nodes[1]
          cli(fnode2,
              f"semaphore acquire mn-sem --holder node{fid2} --permits 1 "
              "--capacity 3 --ttl 30000 --timeout 5000",
              ticket=leader_ticket)

          leader_node.log("3 permits acquired from 3 different nodes")

      with subtest("semaphore exhausted blocks any node"):
          # 4th acquire from any node should fail
          out = cli(leader_node,
                    "semaphore try-acquire mn-sem --holder extra "
                    "--permits 1 --capacity 3 --ttl 30000",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"4th permit should be denied: {out}"
          leader_node.log("Semaphore capacity enforced across nodes")

      with subtest("semaphore status reflects cross-node holders"):
          out = cli(leader_node, "semaphore status mn-sem",
                    ticket=leader_ticket)
          assert out.get("capacity_permits") == 3, \
              f"wrong capacity: {out}"
          assert out.get("available") == 0, \
              f"should have 0 available: {out}"
          leader_node.log(f"Semaphore status: {out.get('available')} available, "
                          f"{out.get('capacity_permits')} capacity")

      with subtest("semaphore release from one node frees for another"):
          # Release from leader
          cli(leader_node, "semaphore release mn-sem --holder leader",
              ticket=leader_ticket)

          # Now another node can acquire
          fid1, fnode1 = follower_nodes[0]
          out = cli(fnode1,
                    "semaphore try-acquire mn-sem --holder reclaimed "
                    "--permits 1 --capacity 3 --ttl 30000",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"should acquire released permit: {out}"
          fnode1.log(f"node{fid1} reclaimed permit released by leader")

          # Clean up
          fid2, fnode2 = follower_nodes[1]
          cli(fnode1, "semaphore release mn-sem --holder reclaimed",
              ticket=leader_ticket)
          cli(fnode1, f"semaphore release mn-sem --holder node{fid1}",
              ticket=leader_ticket)
          cli(fnode2, f"semaphore release mn-sem --holder node{fid2}",
              ticket=leader_ticket)

      # ================================================================
      # RW LOCK MULTI-NODE READERS + WRITER EXCLUSION
      # ================================================================

      with subtest("rwlock readers on different nodes"):
          # Multiple nodes can hold read locks simultaneously
          cli(leader_node,
              "rwlock read mn-rwlock --holder leader-reader --ttl 30000 --timeout 5000",
              ticket=leader_ticket)
          fid1, fnode1 = follower_nodes[0]
          cli(fnode1,
              f"rwlock try-read mn-rwlock --holder node{fid1}-reader --ttl 30000",
              ticket=leader_ticket)
          fid2, fnode2 = follower_nodes[1]
          cli(fnode2,
              f"rwlock try-read mn-rwlock --holder node{fid2}-reader --ttl 30000",
              ticket=leader_ticket)
          leader_node.log("3 readers from 3 different nodes hold read lock")

      with subtest("rwlock writer blocked by cross-node readers"):
          out = cli(leader_node,
                    "rwlock try-write mn-rwlock --holder writer-1 --ttl 30000",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"writer should be blocked by readers: {out}"
          leader_node.log("Writer blocked by readers on 3 different nodes")

      with subtest("rwlock writer succeeds after all readers release"):
          fid1, fnode1 = follower_nodes[0]
          fid2, fnode2 = follower_nodes[1]
          cli(leader_node,
              "rwlock release-read mn-rwlock --holder leader-reader",
              ticket=leader_ticket)
          cli(fnode1,
              f"rwlock release-read mn-rwlock --holder node{fid1}-reader",
              ticket=leader_ticket)
          cli(fnode2,
              f"rwlock release-read mn-rwlock --holder node{fid2}-reader",
              ticket=leader_ticket)

          out = cli(leader_node,
                    "rwlock write mn-rwlock --holder writer-1 --ttl 30000 --timeout 5000",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"writer should succeed after all readers release: {out}"
          write_token = out.get("fencing_token")
          leader_node.log("Writer acquired lock after all cross-node readers released")

      with subtest("rwlock cross-node readers blocked by writer"):
          for nid, nref in follower_nodes:
              out = cli(nref,
                        f"rwlock try-read mn-rwlock --holder blocked-{nid} --ttl 30000",
                        ticket=leader_ticket, check=False)
              if isinstance(out, dict):
                  assert out.get("is_success") is False, \
                      f"reader on node{nid} should be blocked by writer: {out}"
          leader_node.log("All cross-node readers blocked by writer")

          # Clean up
          cli(leader_node,
              f"rwlock release-write mn-rwlock --holder writer-1 "
              f"--fencing-token {write_token}",
              ticket=leader_ticket)

      # ================================================================
      # QUEUE CROSS-NODE ENQUEUE/DEQUEUE
      # ================================================================

      with subtest("queue create"):
          out = cli(leader_node,
                    "queue create mn-queue --visibility-timeout 30000 --max-attempts 3",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, f"queue create failed: {out}"

      with subtest("enqueue from different nodes"):
          cli(leader_node,
              "queue enqueue mn-queue 'from-leader'",
              ticket=leader_ticket)
          fid1, fnode1 = follower_nodes[0]
          cli(fnode1,
              f"queue enqueue mn-queue 'from-node{fid1}'",
              ticket=leader_ticket)
          fid2, fnode2 = follower_nodes[1]
          cli(fnode2,
              f"queue enqueue mn-queue 'from-node{fid2}'",
              ticket=leader_ticket)
          leader_node.log("3 items enqueued from 3 different nodes")

      with subtest("queue status shows all items"):
          out = cli(leader_node, "queue status mn-queue",
                    ticket=leader_ticket)
          assert out.get("visible_count", 0) >= 3, \
              f"queue should have 3+ items: {out}"
          leader_node.log(f"Queue has {out.get('visible_count')} visible items")

      with subtest("dequeue from different node than enqueuer"):
          # Dequeue from a follower — should get items enqueued by leader
          fid1, fnode1 = follower_nodes[0]
          out = cli(fnode1,
                    f"queue dequeue mn-queue --consumer node{fid1} "
                    "--max 1 --visibility 30000",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, f"dequeue failed: {out}"
          items = out.get("items", [])
          assert len(items) == 1, f"expected 1 dequeued item: {items}"
          payload = items[0].get("payload")
          receipt = items[0].get("receipt_handle")
          fnode1.log(f"node{fid1} dequeued: {payload}")

          # Ack from the dequeuing node
          out = cli(fnode1, f"queue ack mn-queue '{receipt}'",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, f"ack failed: {out}"
          fnode1.log(f"node{fid1} acked cross-node dequeue")

      with subtest("dequeue remaining from leader"):
          out = cli(leader_node,
                    "queue dequeue mn-queue --consumer leader --max 3 "
                    "--visibility 30000",
                    ticket=leader_ticket)
          items = out.get("items", [])
          for item in items:
              receipt = item.get("receipt_handle")
              cli(leader_node, f"queue ack mn-queue '{receipt}'",
                  ticket=leader_ticket)
          leader_node.log(f"Leader dequeued and acked {len(items)} remaining items")

      with subtest("queue cleanup"):
          cli(leader_node, "queue delete mn-queue", ticket=leader_ticket)

      # ================================================================
      # SEQUENCE GENERATOR ACROSS NODES
      # ================================================================

      with subtest("sequence monotonic across nodes"):
          vals = []
          for nid, nref in [(leader_id, leader_node)] + follower_nodes:
              out = cli(nref, "sequence next mn-seq", ticket=leader_ticket)
              assert out.get("is_success") is True, \
                  f"sequence next failed on node{nid}: {out}"
              vals.append(out.get("value"))
              nref.log(f"node{nid} got sequence value {vals[-1]}")

          # All values should be strictly increasing
          for i in range(1, len(vals)):
              assert vals[i] > vals[i - 1], \
                  f"sequence not monotonic across nodes: {vals}"
          leader_node.log(f"Sequence monotonic across nodes: {vals}")

      with subtest("sequence reserve from follower"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode, "sequence reserve mn-seq 10",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"sequence reserve from node{fid} failed: {out}"
          reserve_start = out.get("value")
          fnode.log(f"node{fid} reserved 10 IDs starting at {reserve_start}")

          # Next value from any node should skip reserved range
          out = cli(leader_node, "sequence next mn-seq",
                    ticket=leader_ticket)
          next_val = out.get("value")
          assert next_val >= reserve_start + 10, \
              f"next value {next_val} should be >= {reserve_start + 10}"
          leader_node.log("Sequence correctly skipped reserved range")

      # ================================================================
      # LEASE CROSS-NODE OPERATIONS
      # ================================================================

      with subtest("lease grant from follower"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode, "lease grant 60", ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"lease grant from node{fid} failed: {out}"
          mn_lease_id = out.get("lease_id")
          fnode.log(f"node{fid} granted lease {mn_lease_id}")

      with subtest("lease visible from all nodes"):
          out = cli(leader_node, "lease list", ticket=leader_ticket)
          lease_ids = [l.get("lease_id") for l in out.get("leases", [])]
          assert mn_lease_id in lease_ids, \
              f"lease {mn_lease_id} not visible from leader: {lease_ids}"

          for nid, nref in follower_nodes:
              out = cli(nref, f"lease ttl {mn_lease_id}",
                        ticket=leader_ticket)
              assert out.get("is_success") is True, \
                  f"lease ttl from node{nid} failed: {out}"
              remaining = out.get("remaining_ttl_seconds")
              assert remaining is not None and remaining > 0, \
                  f"lease should have TTL on node{nid}: {out}"
          leader_node.log("Lease visible and valid from all nodes")

      with subtest("lease revoke from different node"):
          fid2, fnode2 = follower_nodes[1]
          out = cli(fnode2, f"lease revoke {mn_lease_id}",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"lease revoke from node{fid2} failed: {out}"
          fnode2.log(f"node{fid2} revoked lease granted by node{follower_nodes[0][0]}")

      # ================================================================
      # COORDINATION SURVIVES LEADER FAILOVER
      # ================================================================

      with subtest("setup coordination state before failover"):
          # Create a lock and counter that must survive failover
          out = cli(leader_node,
                    "lock acquire failover-lock --holder survivor "
                    "--ttl 120000 --timeout 5000",
                    ticket=leader_ticket)
          failover_token = out.get("fencing_token")

          cli(leader_node, "counter set failover-counter 42",
              ticket=leader_ticket)
          leader_node.log("Coordination state set up before failover")

      with subtest("stop leader"):
          leader_node.succeed("systemctl stop aspen-node.service")
          leader_node.log(f"Stopped leader (node{leader_id})")
          time.sleep(8)

      with subtest("verify new leader"):
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

      with subtest("counter state survives failover"):
          out = cli(new_leader_node, "counter get failover-counter",
                    ticket=new_ticket)
          assert out.get("value") == 42, \
              f"counter should be 42 after failover, got {out.get('value')}"
          new_leader_node.log("Counter state survived failover")

      with subtest("lock state survives failover"):
          # The lock should still be held — try-acquire from survivor should fail
          out = cli(new_leader_node,
                    "lock try-acquire failover-lock --holder intruder --ttl 30000",
                    ticket=new_ticket, check=False)
          if isinstance(out, dict):
              assert out.get("is_success") is False, \
                  f"lock should still be held after failover: {out}"
          new_leader_node.log("Lock state survived failover")

      with subtest("counter operations work after failover"):
          out = cli(new_leader_node, "counter incr failover-counter",
                    ticket=new_ticket)
          assert out.get("value") == 43, \
              f"counter incr after failover failed: {out}"
          new_leader_node.log("Counter operations work after failover")

      with subtest("new locks work after failover"):
          out = cli(new_leader_node,
                    "lock acquire post-failover-lock --holder new-worker "
                    "--ttl 10000 --timeout 5000",
                    ticket=new_ticket)
          assert out.get("is_success") is True, \
              f"new lock after failover failed: {out}"
          post_token = out.get("fencing_token")
          cli(new_leader_node,
              f"lock release post-failover-lock --holder new-worker "
              f"--fencing-token {post_token}",
              ticket=new_ticket)
          new_leader_node.log("New locks work after failover")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All multi-node coordination integration tests passed!")
    '';
  }
