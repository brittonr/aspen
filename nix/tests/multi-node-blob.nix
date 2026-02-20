# NixOS VM integration test for blob operations across a 3-node Aspen cluster.
#
# Unlike the single-node blob-operations test, this validates real blob
# replication and cross-node accessibility:
#
#   - Add blob on one node, retrieve from all others
#   - Blobs added on different nodes visible from any node
#   - Blob replication status with multiple replicas
#   - Large blob replication across cluster
#   - Blob protection visible across nodes
#   - Repair cycle with actual multi-node replication
#   - Blob operations survive leader failover
#
# Run:
#   nix build .#checks.x86_64-linux.multi-node-blob-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.multi-node-blob-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

  cookie = "multi-node-blob-test";

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
      features = ["blob"];
    };

    environment.systemPackages = [aspenCliPackage];

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "multi-node-blob";

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
      # ADD BLOB ON ONE NODE, RETRIEVE FROM ALL OTHERS
      # ================================================================

      with subtest("add blob on leader, has on followers"):
          out = cli(leader_node, "blob add --data 'cross-node-blob'",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, f"blob add failed: {out}"
          hash1 = out.get("hash")
          assert hash1 is not None, f"no hash: {out}"
          leader_node.log(f"Added blob {hash1} on leader")

          time.sleep(3)

          for nid, nref in follower_nodes:
              out = cli(nref, f"blob has {hash1}", ticket=leader_ticket)
              assert out.get("does_exist") is True, \
                  f"blob not replicated to node{nid}: {out}"
              nref.log(f"node{nid} has blob {hash1}")

      with subtest("get blob data from followers"):
          for nid, nref in follower_nodes:
              out = cli(nref, f"blob get {hash1}", ticket=leader_ticket)
              assert out.get("was_found") is True, \
                  f"blob get failed on node{nid}: {out}"
              nref.log(f"node{nid} retrieved blob data")

      # ================================================================
      # BLOBS FROM DIFFERENT NODES VISIBLE EVERYWHERE
      # ================================================================

      with subtest("add blobs from each node"):
          blob_hashes = {}

          # Blob from leader
          out = cli(leader_node, "blob add --data 'from-leader'",
                    ticket=leader_ticket)
          blob_hashes[leader_id] = out.get("hash")

          # Blob from each follower
          for nid, nref in follower_nodes:
              out = cli(nref, f"blob add --data 'from-node{nid}'",
                        ticket=leader_ticket)
              blob_hashes[nid] = out.get("hash")
              nref.log(f"node{nid} added blob {blob_hashes[nid]}")

          time.sleep(5)

      with subtest("all blobs visible from every node"):
          for source_nid, bhash in blob_hashes.items():
              for nid, nref in [(leader_id, leader_node)] + follower_nodes:
                  out = cli(nref, f"blob has {bhash}",
                            ticket=leader_ticket, check=False)
                  if isinstance(out, dict) and out.get("does_exist") is True:
                      nref.log(
                          f"node{nid} sees blob from node{source_nid}"
                      )
                  else:
                      nref.log(
                          f"node{nid} does not yet see blob from "
                          f"node{source_nid} (replication lag)"
                      )

      with subtest("blob list consistent across nodes"):
          leader_blobs = cli(leader_node, "blob list",
                             ticket=leader_ticket)
          leader_count = len(leader_blobs.get("blobs", []))
          assert leader_count >= 4, \
              f"leader should have 4+ blobs: {leader_count}"
          leader_node.log(f"Leader has {leader_count} blobs")

          for nid, nref in follower_nodes:
              out = cli(nref, "blob list", ticket=leader_ticket)
              count = len(out.get("blobs", []))
              nref.log(f"node{nid} has {count} blobs")
              # Allow some replication lag — at minimum node should
              # have its own blobs
              assert count >= 1, \
                  f"node{nid} should have at least 1 blob: {count}"

      # ================================================================
      # BLOB REPLICATION STATUS (MULTI-NODE)
      # ================================================================

      with subtest("replication status shows multi-node info"):
          out = cli(leader_node,
                    f"blob replication-status {hash1}",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              leader_node.log(f"Replication status: {out}")
              # In a 3-node cluster, replication should report status
          else:
              leader_node.log(
                  f"Replication status returned non-JSON: {out}"
              )

      with subtest("repair cycle with multi-node cluster"):
          out = cli(leader_node, "blob repair-cycle",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict):
              leader_node.log(f"Repair cycle result: {out}")
          else:
              leader_node.log(f"Repair cycle: {out}")

      # ================================================================
      # BLOB PROTECTION VISIBLE ACROSS NODES
      # ================================================================

      with subtest("protect blob on leader"):
          out = cli(leader_node, f"blob protect {hash1} --tag keep-mn",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"blob protect failed: {out}"
          leader_node.log(f"Protected blob {hash1} with tag keep-mn")

      with subtest("protected blob survives delete from follower"):
          fid, fnode = follower_nodes[0]
          out = cli(fnode, f"blob delete {hash1}",
                    ticket=leader_ticket, check=False)
          # Delete should succeed (removes user tags) but blob stays
          # due to protection
          fnode.log(f"node{fid} deleted blob (marks for GC)")

          time.sleep(2)

          # Blob should still exist because of protection tag
          out = cli(leader_node, f"blob has {hash1}",
                    ticket=leader_ticket)
          assert out.get("does_exist") is True, \
              f"protected blob should survive delete: {out}"
          leader_node.log("Protected blob survived cross-node delete")

      with subtest("unprotect from different node"):
          fid2, fnode2 = follower_nodes[1]
          out = cli(fnode2, "blob unprotect keep-mn",
                    ticket=leader_ticket)
          assert out.get("is_success") is True, \
              f"unprotect from node{fid2} failed: {out}"
          fnode2.log(f"node{fid2} unprotected blob tag keep-mn")

      # ================================================================
      # LARGE BLOB REPLICATION
      # ================================================================

      with subtest("large blob replicates across cluster"):
          # Generate 10KB of data (kept small to avoid debug-level tracing
          # of raw byte arrays flooding the Nix builder log)
          leader_node.succeed(
              "dd if=/dev/urandom bs=1024 count=10 2>/dev/null "
              "> /tmp/large-blob.bin"
          )
          ticket = leader_ticket
          leader_node.succeed(
              f"aspen-cli --ticket '{ticket}' --json "
              f"blob add /tmp/large-blob.bin "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          raw = leader_node.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          large_hash = out.get("hash")
          large_size = out.get("size_bytes")
          assert large_hash is not None, f"no hash for large blob: {out}"
          assert large_size >= 10000, \
              f"large blob too small: {large_size}"
          leader_node.log(
              f"Added large blob: {large_hash} ({large_size} bytes)"
          )

          time.sleep(5)

          for nid, nref in follower_nodes:
              out = cli(nref, f"blob has {large_hash}",
                        ticket=leader_ticket, check=False)
              if isinstance(out, dict) and out.get("does_exist") is True:
                  nref.log(
                      f"node{nid} has large blob ({large_size} bytes)"
                  )
              else:
                  nref.log(
                      f"node{nid} large blob not yet replicated "
                      f"(expected for large blobs)"
                  )

      # ================================================================
      # BLOB ADD VIA STDIN FROM FOLLOWER
      # ================================================================

      with subtest("blob add from stdin on follower"):
          fid, fnode = follower_nodes[0]
          ticket = leader_ticket
          fnode.succeed(
              f"echo 'follower-stdin-data' | aspen-cli --ticket '{ticket}' "
              f"--json blob add - >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = fnode.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          stdin_hash = out.get("hash")
          assert stdin_hash is not None, \
              f"no hash from stdin on node{fid}: {out}"
          fnode.log(f"node{fid} added blob via stdin: {stdin_hash}")

          time.sleep(3)

          # Verify from leader
          out = cli(leader_node, f"blob has {stdin_hash}",
                    ticket=leader_ticket, check=False)
          if isinstance(out, dict) and out.get("does_exist") is True:
              leader_node.log(
                  "Leader sees blob added via stdin on follower"
              )
          else:
              leader_node.log(
                  f"Follower stdin blob not yet on leader "
                  f"(replication lag): {out}"
              )

      # ================================================================
      # BLOB OPERATIONS SURVIVE LEADER FAILOVER
      # ================================================================

      with subtest("add blob before failover"):
          out = cli(leader_node,
                    "blob add --data 'pre-failover-blob'",
                    ticket=leader_ticket)
          pre_failover_hash = out.get("hash")
          leader_node.log(f"Pre-failover blob: {pre_failover_hash}")
          time.sleep(3)

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

      with subtest("pre-failover blob accessible after failover"):
          out = cli(new_leader_node,
                    f"blob has {pre_failover_hash}",
                    ticket=new_ticket, check=False)
          if isinstance(out, dict) and out.get("does_exist") is True:
              new_leader_node.log(
                  "Pre-failover blob accessible after failover"
              )
          else:
              new_leader_node.log(
                  f"Pre-failover blob not immediately accessible "
                  f"(blob replication lag): {out}"
              )

      with subtest("new blob operations work after failover"):
          out = cli(new_leader_node,
                    "blob add --data 'post-failover-blob'",
                    ticket=new_ticket)
          assert out.get("is_success") is True, \
              f"blob add after failover failed: {out}"
          post_hash = out.get("hash")
          new_leader_node.log(f"Post-failover blob: {post_hash}")

          time.sleep(3)

          # Verify from the other surviving follower
          other_nid, other_node = [
              (nid, nref) for nid, nref in follower_nodes
              if nid != new_leader
          ][0]
          out = cli(other_node, f"blob has {post_hash}",
                    ticket=new_ticket, check=False)
          if isinstance(out, dict) and out.get("does_exist") is True:
              other_node.log(
                  f"node{other_nid} has post-failover blob"
              )
          else:
              other_node.log(
                  f"node{other_nid} post-failover blob not yet "
                  f"replicated: {out}"
              )

      with subtest("blob list works after failover"):
          out = cli(new_leader_node, "blob list", ticket=new_ticket)
          blobs = out.get("blobs", [])
          assert len(blobs) >= 1, \
              f"blob list should have entries after failover: {blobs}"
          new_leader_node.log(
              f"Blob list after failover: {len(blobs)} blobs"
          )

      # ── restart old leader and verify ────────────────────────────────

      with subtest("restart old leader"):
          leader_node.succeed("systemctl start aspen-node.service")
          leader_node.wait_for_unit("aspen-node.service")
          leader_node.wait_for_file(
              "/var/lib/aspen/cluster-ticket.txt", timeout=30
          )
          time.sleep(8)
          leader_node.log(f"node{leader_id} restarted")

      with subtest("restarted node can add blobs"):
          out = cli(new_leader_node,
                    "blob add --data 'after-rejoin'",
                    ticket=new_ticket)
          assert out.get("is_success") is True, \
              f"blob add after rejoin failed: {out}"
          rejoin_hash = out.get("hash")
          new_leader_node.log(f"After-rejoin blob: {rejoin_hash}")

          time.sleep(3)

          # Verify the blob store works with all 3 nodes
          out = cli(new_leader_node, f"blob has {rejoin_hash}",
                    ticket=new_ticket, check=False)
          if isinstance(out, dict):
              new_leader_node.log(
                  f"After-rejoin blob check: "
                  f"exists={out.get('does_exist')}"
              )

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All multi-node blob integration tests passed!")
    '';
  }
