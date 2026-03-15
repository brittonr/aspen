# NixOS VM integration test: alerts continue firing after leadership transfer.
#
# Proves that the periodic alert evaluator picks up on the new Raft leader
# after a failover. The alert state machine (Ok → Firing → Ok) must survive
# the old leader going down and the new leader being elected.
#
#   1. Bootstrap a 3-node cluster with fast alert evaluation (10s interval)
#   2. Create an alert rule (cpu_usage > 80, fires immediately)
#   3. Ingest metrics above threshold, manually evaluate → verify Firing
#   4. Stop the leader
#   5. Wait for new leader election
#   6. Ingest fresh metrics above threshold on new leader
#   7. Manually evaluate on new leader → verify still Firing
#   8. Wait for one periodic evaluation cycle → confirm evaluator is running
#   9. Ingest metrics below threshold, evaluate → verify Ok
#
# Run:
#   nix build .#checks.x86_64-linux.alert-failover-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.alert-failover-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

  cookie = "alert-failover-test";

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
      # Fast alert evaluation for test speed (10s = minimum allowed)
      extraArgs = ["--alert-evaluation-interval" "10"];
    };

    environment.systemPackages = [aspenCliPackage];

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "alert-failover";
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
          """Run aspen-cli --json on a node and return parsed JSON."""
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
          """Run aspen-cli (human output) on a node."""
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

      def now_us(node=None):
          """Current time as Unix microseconds.

          Uses the VM's clock (not the test driver's) to avoid clock skew.
          The alert evaluate RPC uses SystemTime::now() inside the VM, so
          metric timestamps must also come from the VM clock.
          """
          if node is not None:
              epoch_secs = node.succeed("date +%s").strip()
              return int(epoch_secs) * 1_000_000
          return int(time.time() * 1_000_000)

      def ingest_metric(node, name, value, ticket=None, ts_us=None):
          """Ingest a single gauge metric data point."""
          if ts_us is None:
              ts_us = now_us(node)
          dp = json.dumps([{
              "name": name,
              "metric_type": "Gauge",
              "timestamp_us": ts_us,
              "value": value,
              "labels": [],
              "histogram_buckets": None,
              "histogram_sum": None,
              "histogram_count": None,
          }])
          if ticket is None:
              ticket = get_ticket(node1)
          node.succeed(
              f"echo '{dp}' | aspen-cli --ticket '{ticket}' "
              f"metric ingest >/tmp/_ingest.json 2>/dev/null"
          )

      def get_alert_status(node, name, ticket=None):
          """Get the current alert status string for a rule."""
          out = cli(node, f"alert get {name}", ticket=ticket)
          state = out.get("state")
          if state is None:
              return "UNKNOWN"
          return state.get("status", "UNKNOWN")

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
      node1.log("3-node cluster formed")

      # Find the leader
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
      # CREATE ALERT RULE AND TRIGGER FIRING
      # ================================================================

      with subtest("create alert rule"):
          cli_text(
              leader_node,
              "alert create cpu-high "
              "--metric cpu_usage "
              "--comparison gt "
              "--threshold 80 "
              "--aggregation avg "
              "--window-seconds 300 "
              "--for-seconds 0 "
              "--severity critical "
              '--description "CPU above 80 pct"',
              ticket=leader_ticket,
          )
          leader_node.log("Alert rule cpu-high created")

      with subtest("ingest metric above threshold"):
          ingest_metric(leader_node, "cpu_usage", 95.0, ticket=leader_ticket)
          leader_node.log("Ingested cpu_usage=95.0")

      with subtest("manual evaluate fires alert"):
          out = cli(leader_node, "alert evaluate cpu-high",
                    ticket=leader_ticket)
          assert out.get("status") == "FIRING", \
              f"alert should be FIRING after threshold breach: {out}"
          assert out.get("did_transition") is True, \
              f"should have transitioned Ok→Firing: {out}"
          leader_node.log("Alert cpu-high is FIRING (Ok→Firing transition)")

      with subtest("alert get confirms Firing state"):
          status = get_alert_status(leader_node, "cpu-high",
                                    ticket=leader_ticket)
          assert status == "Firing", f"expected Firing, got {status}"
          leader_node.log("alert get confirms Firing state persisted")

      # ================================================================
      # LEADERSHIP TRANSFER (STOP LEADER)
      # ================================================================

      with subtest("stop leader"):
          leader_node.succeed("systemctl stop aspen-node.service")
          leader_node.log(f"Stopped leader (node{leader_id})")
          time.sleep(8)

      with subtest("verify new leader elected"):
          new_leader = None
          new_leader_node = None
          new_ticket = None

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

      # ================================================================
      # ALERT STATE SURVIVES FAILOVER
      # ================================================================

      with subtest("alert state readable on new leader"):
          status = get_alert_status(new_leader_node, "cpu-high",
                                    ticket=new_ticket)
          assert status == "Firing", \
              f"alert state should survive failover: expected Firing, got {status}"
          new_leader_node.log("Alert state Firing survived leadership transfer")

      with subtest("ingest fresh metric on new leader"):
          # Old data point might age out of the 300s window eventually.
          # Ingest a fresh one so evaluation has data.
          ingest_metric(new_leader_node, "cpu_usage", 92.0,
                        ticket=new_ticket)
          new_leader_node.log("Ingested cpu_usage=92.0 on new leader")

      with subtest("manual evaluate on new leader still fires"):
          out = cli(new_leader_node, "alert evaluate cpu-high",
                    ticket=new_ticket)
          assert out.get("status") == "FIRING", \
              f"alert should remain FIRING on new leader: {out}"
          # No transition because it was already Firing
          assert out.get("did_transition") is False, \
              f"should NOT transition (Firing→Firing is no-op): {out}"
          new_leader_node.log("Manual evaluate on new leader: still FIRING")

      # ================================================================
      # PERIODIC EVALUATOR RUNS ON NEW LEADER
      # ================================================================
      # Prove the background evaluator picks up on the new leader by
      # creating a second alert rule + ingesting metrics, then waiting
      # for the periodic cycle to fire it WITHOUT any manual evaluate.

      with subtest("create second alert rule for periodic eval proof"):
          cli_text(
              new_leader_node,
              "alert create mem-high "
              "--metric mem_usage "
              "--comparison gt "
              "--threshold 70 "
              "--aggregation avg "
              "--window-seconds 300 "
              "--for-seconds 0 "
              "--severity warning "
              '--description "Memory above 70 pct"',
              ticket=new_ticket,
          )
          new_leader_node.log("Alert rule mem-high created on new leader")

      with subtest("ingest metric for second rule"):
          ingest_metric(new_leader_node, "mem_usage", 95.0,
                        ticket=new_ticket)
          new_leader_node.log("Ingested mem_usage=95.0 on new leader")

      with subtest("periodic evaluator fires second alert automatically"):
          # The evaluator runs every 10s. Wait up to 30s for it to
          # transition mem-high from Ok to Firing without manual trigger.
          fired = False
          for _ in range(6):
              time.sleep(5)
              status = get_alert_status(new_leader_node, "mem-high",
                                        ticket=new_ticket)
              new_leader_node.log(f"mem-high status: {status}")
              if status == "Firing":
                  fired = True
                  break

          assert fired, \
              "periodic evaluator did not fire mem-high on new leader within 30s"
          new_leader_node.log("Periodic evaluator confirmed: mem-high fired automatically")

      # ================================================================
      # ALERT RESOLVES AFTER METRICS DROP
      # ================================================================

      with subtest("ingest metric below threshold"):
          # Ingest many low values to shift the average below 80
          for i in range(5):
              ingest_metric(new_leader_node, "cpu_usage", 10.0,
                            ticket=new_ticket,
                            ts_us=now_us(new_leader_node) + i * 1000)
          new_leader_node.log("Ingested 5x cpu_usage=10.0 below threshold")

      with subtest("evaluate resolves alert to Ok"):
          out = cli(new_leader_node, "alert evaluate cpu-high",
                    ticket=new_ticket)
          assert out.get("status") == "OK", \
              f"alert should resolve to OK with low metrics: {out}"
          assert out.get("did_transition") is True, \
              f"should transition Firing→Ok: {out}"
          new_leader_node.log("Alert resolved: Firing→Ok after low metrics")

      with subtest("alert get confirms Ok state"):
          status = get_alert_status(new_leader_node, "cpu-high",
                                    ticket=new_ticket)
          assert status == "Ok", f"expected Ok after resolution, got {status}"
          new_leader_node.log("alert get confirms Ok state after resolution")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("Alert failover test passed!")
    '';
  }
