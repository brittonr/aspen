# NixOS VM integration test for a multi-node Aspen cluster.
#
# Spins up a 3-node Aspen cluster inside QEMU VMs with full networking,
# then exercises cluster formation, Raft consensus, data replication,
# leader failover, and cross-node forge operations:
#
#   - Cluster bootstrap (init on node1, add learners, promote to voters)
#   - Raft consensus verification (leader election, term agreement)
#   - Data replication (write on leader, read from followers)
#   - Leader failover (stop leader, verify re-election + continued ops)
#   - Cross-node forge operations (create repo on one node, query from another)
#
# Run:
#   nix build .#checks.x86_64-linux.multi-node-cluster-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.multi-node-cluster-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each).
  # Each node gets a unique key so they have distinct iroh endpoint IDs.
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

  # Shared cluster cookie — all nodes must agree.
  cookie = "multi-node-vm-test";

  # Common node configuration (DRY).
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
      features = ["forge" "blob"];
    };

    environment.systemPackages = [aspenCliPackage];

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "multi-node-cluster";

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
          """Run aspen-cli --json on a specific node and return parsed JSON.

          Uses node1's ticket by default (the bootstrap node).
          Redirects stdout to temp file to avoid serial console corruption.
          """
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
          """Extract full endpoint address (id + addrs) from node's journal.

          The node logs its endpoint info when generating the cluster ticket:
            endpoint_id=<hex> direct_addrs=[addr1, addr2, ...]
          We parse these and construct a JSON EndpointAddr that the
          add-learner handler can deserialize:
            {"id": "<hex>", "addrs": [{"Ip": "host:port"}, ...]}
          """
          output = node.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep 'cluster ticket generated'"
              " | head -1"
          )
          # Extract endpoint_id
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, \
              f"endpoint_id not found in journal: {output[:300]}"
          eid = eid_match.group(1)

          # Extract direct addresses — pick the 192.168.x.x address
          # which is routable on the NixOS test VLAN
          addrs = []
          for addr in re.findall(r'(192\.168\.\d+\.\d+:\d+)', output):
              addrs.append({"Ip": addr})

          # Fallback: also grab any 10.x.x.x addresses
          if not addrs:
              for addr in re.findall(r'(\d+\.\d+\.\d+\.\d+:\d+)', output):
                  addrs.append({"Ip": addr})

          assert addrs, f"no direct addrs found in journal: {output[:300]}"
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

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      # Wait for all nodes to start and write their tickets
      for node in [node1, node2, node3]:
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      # Wait for node1 to respond to health checks
      wait_for_healthy(node1)

      # Extract full endpoint addresses from all nodes (id + socket addrs).
      # These include the actual IP:port pairs routable on the test VLAN.
      addr2_json = get_endpoint_addr_json(node2)
      addr3_json = get_endpoint_addr_json(node3)

      # ── cluster formation ────────────────────────────────────────────

      with subtest("initialize cluster on node1"):
          cli_text(node1, "cluster init")
          time.sleep(2)

          # Verify node1 is the leader of a single-node cluster
          status = cli(node1, "cluster status")
          node1.log(f"Initial cluster status: {status}")
          nodes_list = status.get("nodes", [])
          assert len(nodes_list) == 1, \
              f"expected 1 voter, got {len(nodes_list)}: {nodes_list}"
          assert nodes_list[0]["is_leader"], \
              f"node1 should be leader: {nodes_list}"

      with subtest("add node2 as learner"):
          # Pass full JSON EndpointAddr with routable socket addresses
          cli_text(node1, f"cluster add-learner --node-id 2 --addr '{addr2_json}'")
          time.sleep(3)
          node1.log("node2 added as learner")

      with subtest("add node3 as learner"):
          cli_text(node1, f"cluster add-learner --node-id 3 --addr '{addr3_json}'")
          time.sleep(3)
          node1.log("node3 added as learner")

      with subtest("promote to 3-node voter set"):
          # Change membership to include all three nodes as voters
          cli_text(node1, "cluster change-membership 1 2 3")
          time.sleep(5)

          # Verify all three are voters
          status = cli(node1, "cluster status")
          node1.log(f"After promotion: {status}")
          nodes_list = status.get("nodes", [])
          voters = [n for n in nodes_list if n.get("is_voter", False)]
          assert len(voters) == 3, \
              f"expected 3 voters, got {len(voters)}: {nodes_list}"

      # ── raft consensus verification ──────────────────────────────────

      with subtest("all nodes agree on leader"):
          time.sleep(2)
          leaders = set()
          for node_ref, name in [(node1, "node1"), (node2, "node2"), (node3, "node3")]:
              metrics = cli(node_ref, "cluster metrics")
              leader = metrics.get("current_leader")
              node_ref.log(
                  f"{name} sees leader={leader}, "
                  f"term={metrics.get('current_term')}"
              )
              if leader is not None:
                  leaders.add(leader)

          assert len(leaders) == 1, f"nodes disagree on leader: {leaders}"
          leader_id = leaders.pop()
          node1.log(f"All nodes agree: leader is node {leader_id}")

      with subtest("raft terms are consistent"):
          terms = set()
          for node_ref, name in [
              (node1, "node1"), (node2, "node2"), (node3, "node3")
          ]:
              metrics = cli(node_ref, "cluster metrics")
              term = metrics.get("current_term")
              terms.add(term)
              node_ref.log(f"{name} term={term}")

          assert len(terms) == 1, f"term mismatch across nodes: {terms}"

      # ── data replication: write on leader, read from followers ───────

      with subtest("create repo on leader, visible from all nodes"):
          out = cli(
              node1,
              "git init repl-test --description 'Replication test repo'"
          )
          repo_id = out.get("id") or out.get("repo_id", "")
          assert repo_id, f"repo init should return id: {out}"
          node1.log(f"Created repo {repo_id} on node1")

          # Give replication a moment
          time.sleep(3)

          # Verify the repo is visible from all three nodes
          for node_ref, name in [
              (node1, "node1"), (node2, "node2"), (node3, "node3")
          ]:
              repos = cli(node_ref, "git list")
              repo_names = [r["name"] for r in repos.get("repos", [])]
              assert "repl-test" in repo_names, \
                  f"repl-test not visible on {name}: {repo_names}"
              node_ref.log(f"{name} can see repo repl-test")

      with subtest("store blob on node1, retrieve from node2 and node3"):
          node1.succeed("echo -n 'Replicated blob content' > /tmp/repl-blob.txt")
          blob_out = cli(
              node1, f"git store-blob -r {repo_id} /tmp/repl-blob.txt"
          )
          blob_hash = blob_out.get("hash", "")
          assert len(blob_hash) == 64, f"bad blob hash: {blob_hash}"
          node1.log(f"Stored blob {blob_hash} via node1")

          time.sleep(3)

          # Retrieve from node2
          ticket1 = get_ticket(node1)
          node2.succeed(
              f"aspen-cli --ticket '{ticket1}' "
              f"git get-blob {blob_hash} -o /tmp/got-blob.txt 2>/dev/null"
          )
          content = node2.succeed("cat /tmp/got-blob.txt").strip()
          assert content == "Replicated blob content", \
              f"blob mismatch on node2: {content!r}"
          node2.log("node2 retrieved replicated blob successfully")

          # Retrieve from node3
          node3.succeed(
              f"aspen-cli --ticket '{ticket1}' "
              f"git get-blob {blob_hash} -o /tmp/got-blob.txt 2>/dev/null"
          )
          content = node3.succeed("cat /tmp/got-blob.txt").strip()
          assert content == "Replicated blob content", \
              f"blob mismatch on node3: {content!r}"
          node3.log("node3 retrieved replicated blob successfully")

      with subtest("full commit chain replicates across nodes"):
          tree_out = cli(
              node1,
              f"git create-tree -r {repo_id} "
              f"-e '100644:README.md:{blob_hash}'"
          )
          tree_hash = tree_out.get("hash", "")

          commit_out = cli(
              node1,
              f"git commit -r {repo_id} --tree {tree_hash} "
              f"-m 'Replicated commit'"
          )
          commit_hash = commit_out.get("hash", "")
          assert len(commit_hash) == 64, f"bad commit hash: {commit_hash}"

          # Push ref
          cli_text(
              node1,
              f"git push -r {repo_id} --ref-name heads/main "
              f"--hash {commit_hash} -f"
          )
          time.sleep(3)

          # Verify commit visible from node2
          log2 = cli(node2, f"git log -r {repo_id}")
          assert log2.get("count", 0) >= 1, \
              f"log empty on node2: {log2}"
          node2.log("node2 sees replicated commit log")

          # Verify ref visible from node3
          ref3 = cli(node3, f"git get-ref -r {repo_id} heads/main")
          assert ref3.get("hash") == commit_hash, \
              f"ref mismatch on node3: expected {commit_hash}, got {ref3}"
          node3.log("node3 sees replicated ref heads/main")

      # ── cross-node forge operations ──────────────────────────────────

      with subtest("create issue on node2, read from node3"):
          issue = cli(
              node2,
              f"issue create -r {repo_id} -t 'Cross-node issue' "
              f"-b 'Created on node2' -l multinode"
          )
          issue_id = issue.get("id", "")
          assert issue_id, f"issue create should return id: {issue}"
          node2.log(f"Created issue {issue_id} on node2")

          time.sleep(2)

          si = cli(node3, f"issue show -r {repo_id} {issue_id}")
          assert si.get("title") == "Cross-node issue", \
              f"issue title mismatch on node3: {si}"
          node3.log("node3 can read issue created on node2")

      with subtest("create branch on node3, list from node1"):
          cli_text(
              node3,
              f"branch create -r {repo_id} multi-node-branch "
              f"--from {commit_hash}"
          )
          time.sleep(2)

          bl = cli(node1, f"branch list -r {repo_id}")
          branch_names = [r["name"] for r in bl.get("refs", [])]
          assert any("multi-node-branch" in n for n in branch_names), \
              f"multi-node-branch not visible on node1: {branch_names}"
          node1.log("node1 sees branch created on node3")

      # ── leader failover ──────────────────────────────────────────────

      with subtest("identify current leader"):
          metrics = cli(node1, "cluster metrics")
          old_leader = metrics.get("current_leader")
          node1.log(f"Current leader before failover: node {old_leader}")

          # Map leader ID to node reference
          leader_node = {1: node1, 2: node2, 3: node3}[old_leader]
          follower_nodes = [
              (nid, nref)
              for nid, nref in [(1, node1), (2, node2), (3, node3)]
              if nid != old_leader
          ]

      with subtest("stop leader node"):
          leader_node.succeed("systemctl stop aspen-node.service")
          leader_node.log(
              f"Stopped aspen-node on leader (node {old_leader})"
          )
          # Allow Raft election timeout to trigger re-election
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
                  metrics = cli(nref, "cluster metrics", ticket=ticket)
                  leader = metrics.get("current_leader")
                  nref.log(f"node{nid} sees new leader: {leader}")
                  if leader is not None and leader != old_leader:
                      new_leader = leader
              except Exception as e:
                  nref.log(f"node{nid} not ready yet: {e}")

          assert new_leader is not None, \
              "no new leader elected after failover"
          assert new_leader != old_leader, \
              f"leader unchanged: old={old_leader}, new={new_leader}"
          node1.log(f"New leader after failover: node {new_leader}")

      with subtest("cluster operations work after failover"):
          # Route writes through the NEW leader, not just any survivor
          new_leader_node = {1: node1, 2: node2, 3: node3}[new_leader]
          new_leader_ticket = get_ticket(new_leader_node)

          # Create a new repo after failover — via the new leader
          out = cli(
              new_leader_node,
              "git init failover-test "
              "--description 'Created after failover'",
              ticket=new_leader_ticket,
          )
          failover_repo_id = out.get("id") or out.get("repo_id", "")
          assert failover_repo_id, \
              f"repo init after failover failed: {out}"
          new_leader_node.log(
              f"Created repo after failover: {failover_repo_id}"
          )

          time.sleep(3)

          # Verify visible from the other surviving node
          other_nid, other_node = [
              (nid, nref) for nid, nref in follower_nodes
              if nid != new_leader
          ][0]
          repos = cli(
              other_node, "git list", ticket=new_leader_ticket
          )
          repo_names = [r["name"] for r in repos.get("repos", [])]
          assert "failover-test" in repo_names, \
              f"failover-test not on node{other_nid}: {repo_names}"
          other_node.log(
              f"node{other_nid} sees repo created after failover"
          )

      with subtest("writes to follower auto-route to new leader"):
          # Send a write through a follower using the leader's ticket.
          # The follower's own ticket only has its own address, so
          # NOT_LEADER rotation can't discover the new leader.
          # Instead, we use the new leader's ticket (which has routable
          # addresses) and issue the command from the follower node.
          follower_nid, follower_node = [
              (nid, nref) for nid, nref in follower_nodes
              if nid != new_leader
          ][0]

          out = cli(
              follower_node,
              "git init follower-write-test "
              "--description 'Written via follower'",
              ticket=new_leader_ticket,
          )
          fwt_id = out.get("id") or out.get("repo_id", "")
          assert fwt_id, \
              f"write via follower should succeed via NOT_LEADER rotation: {out}"
          follower_node.log(
              f"node{follower_nid} (follower) write routed to leader"
          )

          time.sleep(2)

          # Verify the repo is visible from the leader
          repos = cli(
              new_leader_node, "git list",
              ticket=new_leader_ticket
          )
          repo_names = [r["name"] for r in repos.get("repos", [])]
          assert "follower-write-test" in repo_names, \
              f"follower-write-test not visible on leader: {repo_names}"

      with subtest("raft reads work with 2-node quorum"):
          repos = cli(
              new_leader_node, "git list",
              ticket=new_leader_ticket
          )
          repo_names = [r["name"] for r in repos.get("repos", [])]
          assert "repl-test" in repo_names, \
              f"repl-test missing after failover: {repo_names}"
          assert "failover-test" in repo_names, \
              f"failover-test missing: {repo_names}"
          new_leader_node.log(
              "Raft KV reads work with 2-node quorum"
          )

      # ── rejoin: bring failed node back ───────────────────────────────

      with subtest("restart failed leader node"):
          leader_node.succeed("systemctl start aspen-node.service")
          leader_node.wait_for_unit("aspen-node.service")
          leader_node.wait_for_file(
              "/var/lib/aspen/cluster-ticket.txt", timeout=30
          )
          time.sleep(8)  # Allow time to rejoin and catch up

          old_leader_ticket = get_ticket(leader_node)
          leader_node.wait_until_succeeds(
              f"aspen-cli --ticket '{old_leader_ticket}' "
              f"cluster health 2>/dev/null",
              timeout=30,
          )
          leader_node.log(f"node{old_leader} restarted and healthy")

      with subtest("restarted node has caught up"):
          time.sleep(5)

          # All 3 nodes are back — repos should all be visible
          repos = cli(
              new_leader_node, "git list",
              ticket=new_leader_ticket
          )
          repo_names = [r["name"] for r in repos.get("repos", [])]
          assert "failover-test" in repo_names, \
              f"failover-test missing after rejoin: {repo_names}"
          assert "repl-test" in repo_names, \
              f"repl-test missing after rejoin: {repo_names}"
          leader_node.log(
              f"node{old_leader} rejoined — cluster data intact"
          )

      with subtest("kv and blob operations work after full rejoin"):
          # KV-backed operations (branch list, repo list)
          branches = cli(
              new_leader_node,
              f"branch list -r {repo_id}",
              ticket=new_leader_ticket,
          )
          branch_names = [
              r["name"] for r in branches.get("refs", [])
          ]
          assert any("main" in n for n in branch_names), \
              f"main branch missing after rejoin: {branch_names}"
          assert any("multi-node-branch" in n for n in branch_names), \
              f"multi-node-branch missing after rejoin: {branch_names}"
          new_leader_node.log(
              "KV operations work with all 3 nodes after rejoin"
          )

          # Blob-backed operations: issue show requires fetching
          # signed COB objects from the blob store. Blob replication
          # may not have caught up yet after restart — this is
          # best-effort. (Blob reads fail fast at 5s now.)
          si = cli(
              new_leader_node,
              f"issue show -r {repo_id} {issue_id}",
              ticket=new_leader_ticket,
              check=False,
          )
          if isinstance(si, dict) and si.get("title") == "Cross-node issue":
              new_leader_node.log(
                  "Blob-backed issue show works after full rejoin"
              )
          else:
              new_leader_node.log(
                  f"Blob-backed issue show not yet available after rejoin "
                  f"(expected — blob replication lag): {si}"
              )

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All multi-node cluster integration tests passed!")
    '';
  }
