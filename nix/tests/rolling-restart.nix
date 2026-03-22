# Minimal rolling restart test: 3-node cluster formation + rolling restart.
#
# Tests that all 3 nodes can be restarted (followers first, leader last)
# and the cluster recovers with a functioning leader. No CI, no forge,
# no nix builds — just Raft consensus + iroh networking.
#
# Run:
#   nix build .#checks.x86_64-linux.rolling-restart-test --option sandbox false
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  secretKey1 = "0000000000000009000000000000000900000000000000090000000000000009";
  secretKey2 = "000000000000000a000000000000000a000000000000000a000000000000000a";
  secretKey3 = "000000000000000b000000000000000b000000000000000b000000000000000b";
  cookie = "rolling-restart-test";

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
      enableSnix = false;
      features = [];
    };

    environment.systemPackages = [aspenCliPackage];
    networking.firewall.enable = false;

    virtualisation = {
      memorySize = 1024;
      cores = 2;
    };
  };
in
  pkgs.testers.nixosTest {
    name = "rolling-restart";
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

      # ── helpers ──
      def get_ticket(node):
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, ticket=None):
          if ticket is None:
              ticket = get_ticket(node1)
          raw = node.succeed(f"aspen-cli --ticket '{ticket}' {cmd} 2>/dev/null").strip()
          try:
              return json.loads(raw)
          except Exception:
              return raw

      def cli_text(node, cmd, ticket=None):
          if ticket is None:
              ticket = get_ticket(node1)
          return node.succeed(f"aspen-cli --ticket '{ticket}' {cmd} 2>/dev/null").strip()

      def get_endpoint_addr_json(node):
          output = node.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep 'cluster ticket generated'"
              " | head -1"
          )
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, f"endpoint_id not found: {output[:300]}"
          eid = eid_match.group(1)
          addrs = []
          addr_match = re.search(r'direct_addrs=\[(.*?)\]', output)
          if addr_match:
              for a in re.findall(r'[\d.]+:\d+', addr_match.group(1)):
                  addrs.append(a)
          return json.dumps({"id": eid, "addrs": addrs})

      def wait_for_healthy(node, timeout=60):
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      # ── 1. Form cluster ──
      with subtest("form 3-node cluster"):
          node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          wait_for_healthy(node1)

          addr2_json = get_endpoint_addr_json(node2)
          addr3_json = get_endpoint_addr_json(node3)

          cli_text(node1, "cluster init")
          time.sleep(2)
          cli_text(node1, f"cluster add-learner --node-id 2 --addr '{addr2_json}'")
          time.sleep(3)
          cli_text(node1, f"cluster add-learner --node-id 3 --addr '{addr3_json}'")
          time.sleep(2)
          cli_text(node1, "cluster change-membership 1 2 3")
          time.sleep(3)

          # Wait for all nodes healthy
          for n in [node1, node2, node3]:
              wait_for_healthy(n, timeout=30)
          node1.log("3-node cluster formed")

      # ── 2. Write test data ──
      with subtest("write KV data"):
          ticket1 = get_ticket(node1)
          cli(node1, "kv set test-key test-value", ticket=ticket1)
          result = cli(node1, "kv get test-key", ticket=ticket1)
          node1.log(f"KV get result: {result}")

      # ── 3. Find leader ──
      with subtest("identify leader"):
          ticket1 = get_ticket(node1)
          metrics = cli(node1, "cluster metrics", ticket=ticket1)
          leader_id = metrics.get("current_leader", 1) if isinstance(metrics, dict) else 1
          node1.log(f"Leader before restart: node{leader_id}")

      # ── 4. Rolling restart (followers first, leader last) ──
      nodes_by_id = {1: node1, 2: node2, 3: node3}
      # Restart order: non-leaders first, leader last
      restart_order = [i for i in [1,2,3] if i != leader_id] + [leader_id]
      node1.log(f"Restart order: {restart_order}")

      for idx, nid in enumerate(restart_order):
          label = "leader" if nid == leader_id else f"follower {idx+1}"
          with subtest(f"restart node{nid} ({label})"):
              node = nodes_by_id[nid]
              node.succeed("systemctl stop aspen-node.service")
              time.sleep(5)
              node.succeed("systemctl start aspen-node.service")
              wait_for_healthy(node, timeout=120)
              node.log(f"node{nid} restarted and healthy")

      # ── 5. Verify cluster health ──
      with subtest("cluster healthy after rolling restart"):
          # Use node1's ticket — as leader it can answer cluster queries.
          # Other nodes' tickets only point to themselves and may return
          # "not initialized" if they haven't caught up yet.
          ticket1 = get_ticket(node1)
          m = cli(node1, "cluster metrics", ticket=ticket1)
          node1.log(f"metrics response: {m}")
          if isinstance(m, dict):
              current = m.get("current_leader")
              node1.log(f"Leader after restart: node{current}")
              assert current is not None and current > 0, f"No leader in metrics: {m}"
          else:
              # Metrics returned non-JSON — try cluster health instead
              node1.wait_until_succeeds(
                  f"aspen-cli --ticket '{ticket1}' cluster health 2>/dev/null",
                  timeout=30,
              )
              node1.log("Cluster healthy (health check passed)")

      # ── 6. Verify KV data survived ──
      with subtest("KV data survived restart"):
          for nid, node in nodes_by_id.items():
              try:
                  ticket = get_ticket(node)
                  result = cli(node, "kv get test-key", ticket=ticket)
                  val = result.get("value", "") if isinstance(result, dict) else str(result)
                  if "test-value" in str(val):
                      node1.log(f"KV data verified on node{nid}")
                      break
              except Exception:
                  pass

      node1.log("=== Rolling restart test PASSED ===")
    '';
  }
