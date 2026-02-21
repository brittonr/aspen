# NixOS VM integration test for the Aspen HTTP proxy across a 3-node cluster.
#
# Spins up a 4-node QEMU environment:
#
#   node1, node2, node3 — 3-node Raft cluster with proxy enabled, each
#                         running a local HTTP origin service on a unique port
#   client             — external machine with only aspen-cli (proxy features)
#
# Test scenarios:
#
#   1. TCP tunnel through each cluster node
#      Client tunnels to origin services running on different cluster nodes.
#
#   2. Cross-node tunneling
#      Client tunnels through node1's proxy to reach a service on node2.
#
#   3. Proxy during KV operations
#      Proxy tunnels work while the cluster is serving Raft consensus writes.
#
#   4. HTTP forward proxy through cluster nodes
#      Client uses CONNECT-style HTTP proxying through different cluster members.
#
#   5. Concurrent proxy streams
#      Multiple simultaneous tunnels through different cluster nodes.
#
#   6. Proxy survives leader failover
#      Tunnel through a surviving follower continues working after leader stop.
#
#   7. Proxy to restarted node
#      After old leader restarts, proxy through it works again.
#
#   8. Large payload transfer through proxy
#      Transfer large HTTP responses through the tunnel.
#
#   9. Multiple proxy targets on the same node
#      Tunnel to different services behind the same cluster node.
#
# Run:
#   nix build .#checks.x86_64-linux.multi-node-proxy-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.multi-node-proxy-test.driverInteractive
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

  cookie = "multi-node-proxy-test";

  # Each cluster node runs its own HTTP origin service.
  # Different ports so cross-node tunneling is testable.
  httpPort1 = 8081;
  httpPort2 = 8082;
  httpPort3 = 8083;
  # Second service on node1 for multi-target testing.
  httpPort1Alt = 8091;

  # Python handler template for origin HTTP services.
  # Returns JSON with the node identity and request path.
  mkHandler = nodeLabel: port:
    pkgs.writeText "handler-${nodeLabel}.py" ''
      import http.server
      import json
      import sys

      class Handler(http.server.BaseHTTPRequestHandler):
          def do_GET(self):
              # Support ?size=N query param for large payload tests
              size_param = 0
              if "?" in self.path:
                  parts = self.path.split("?", 1)
                  path = parts[0]
                  for param in parts[1].split("&"):
                      if param.startswith("size="):
                          try:
                              size_param = int(param[5:])
                          except ValueError:
                              pass
              else:
                  path = self.path

              if size_param > 0:
                  # Generate a large JSON payload of approximately the requested size
                  payload = "A" * size_param
                  body = json.dumps({
                      "status": "ok",
                      "node": "${nodeLabel}",
                      "path": path,
                      "payload_size": size_param,
                      "payload": payload,
                  }).encode()
              else:
                  body = json.dumps({
                      "status": "ok",
                      "node": "${nodeLabel}",
                      "path": path,
                      "port": ${toString port},
                      "message": "hello from ${nodeLabel}",
                  }).encode()
              self.send_response(200)
              self.send_header("Content-Type", "application/json")
              self.send_header("Content-Length", str(len(body)))
              self.end_headers()
              self.wfile.write(body)

          def log_message(self, fmt, *args):
              pass  # suppress noisy access logs

      http.server.HTTPServer(("0.0.0.0", ${toString port}), Handler).serve_forever()
    '';

  # Systemd service for an origin HTTP server.
  mkOriginService = nodeLabel: port: handler: {
    description = "Origin HTTP service ${nodeLabel}:${toString port}";
    wantedBy = ["multi-user.target"];
    after = ["network.target"];
    serviceConfig = {
      Type = "simple";
      ExecStart = "${pkgs.python3}/bin/python3 ${handler}";
      Restart = "on-failure";
    };
  };

  # Shared NixOS configuration for each cluster node.
  mkNodeConfig = {
    nodeId,
    secretKey,
    httpPort,
    nodeLabel,
    extraOriginServices ? {},
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
      extraArgs = ["--enable-proxy"];
    };

    systemd.services =
      {
        "origin-http" =
          mkOriginService nodeLabel httpPort (mkHandler nodeLabel httpPort);
      }
      // extraOriginServices;

    environment.systemPackages = with pkgs; [curl aspenCliPackage];

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "multi-node-proxy";
    skipLint = true;

    nodes = {
      node1 = mkNodeConfig {
        nodeId = 1;
        secretKey = secretKey1;
        httpPort = httpPort1;
        nodeLabel = "node1";
        # Second origin service for multi-target testing.
        extraOriginServices = {
          "origin-http-alt" =
            mkOriginService "node1-alt" httpPort1Alt
            (mkHandler "node1-alt" httpPort1Alt);
        };
      };

      node2 = mkNodeConfig {
        nodeId = 2;
        secretKey = secretKey2;
        httpPort = httpPort2;
        nodeLabel = "node2";
      };

      node3 = mkNodeConfig {
        nodeId = 3;
        secretKey = secretKey3;
        httpPort = httpPort3;
        nodeLabel = "node3";
      };

      # External client node — no aspen-node, only CLI.
      client = {
        environment.systemPackages = with pkgs; [curl aspenCliPackage];
        networking.firewall.enable = false;
        virtualisation.memorySize = 1024;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import re
      import time

      HTTP_PORT1 = ${toString httpPort1}
      HTTP_PORT2 = ${toString httpPort2}
      HTTP_PORT3 = ${toString httpPort3}
      HTTP_PORT1_ALT = ${toString httpPort1Alt}

      # Local proxy port allocator — avoids collisions across subtests.
      _next_proxy_port = 19100

      def alloc_port():
          global _next_proxy_port
          p = _next_proxy_port
          _next_proxy_port += 1
          return p

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket(node):
          """Read cluster ticket from a node."""
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
          """Extract full endpoint address (id + addrs) from journal."""
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

      def curl_json(node, url, timeout=10):
          """curl a URL and parse JSON response."""
          raw = node.succeed(
              f"curl -sf --max-time {timeout} '{url}' 2>/dev/null"
          )
          return json.loads(raw)

      def curl_proxy_json(node, url, proxy_addr, timeout=10):
          """curl through an HTTP proxy and parse JSON response."""
          raw = node.succeed(
              f"curl -sf --max-time {timeout} "
              f"--proxy http://{proxy_addr} "
              f"'{url}' 2>/dev/null"
          )
          return json.loads(raw)

      def start_tcp_tunnel(node, ticket, target, local_port, log_file):
          """Start a TCP tunnel proxy in the background. Returns the port."""
          node.succeed(
              f"aspen-cli --ticket '{ticket}' proxy start "
              f"--target {target} "
              f"--local-addr 127.0.0.1:{local_port} "
              f"&>{log_file} &"
          )
          time.sleep(3)
          node.wait_for_open_port(local_port, timeout=15)
          return local_port

      def start_forward_proxy(node, ticket, local_port, log_file):
          """Start an HTTP forward proxy in the background. Returns the port."""
          node.succeed(
              f"aspen-cli --ticket '{ticket}' proxy forward "
              f"--local-addr 127.0.0.1:{local_port} "
              f"&>{log_file} &"
          )
          time.sleep(3)
          node.wait_for_open_port(local_port, timeout=15)
          return local_port

      def kill_proxies(node):
          """Kill all proxy processes on a node."""
          # Use pgrep+kill instead of pkill to avoid matching the shell itself.
          # Exit 0 regardless (processes may already be gone).
          node.execute("pgrep -f 'aspen-cli.*proxy' | xargs -r kill 2>/dev/null; true")
          time.sleep(1)

      # ── cluster boot + formation ────────────────────────────────────
      start_all()

      # Wait for all cluster nodes to boot.
      for node in [node1, node2, node3]:
          node.wait_for_unit("aspen-node.service")
          node.wait_for_unit("origin-http.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      # Wait for node1's alt origin too.
      node1.wait_for_unit("origin-http-alt.service")

      # Wait for origin ports.
      node1.wait_for_open_port(HTTP_PORT1)
      node1.wait_for_open_port(HTTP_PORT1_ALT)
      node2.wait_for_open_port(HTTP_PORT2)
      node3.wait_for_open_port(HTTP_PORT3)

      client.wait_for_unit("default.target")

      # Verify origin services locally.
      for label, node, port in [("node1", node1, HTTP_PORT1),
                                 ("node2", node2, HTTP_PORT2),
                                 ("node3", node3, HTTP_PORT3)]:
          resp = curl_json(node, f"http://127.0.0.1:{port}/healthz")
          assert resp["status"] == "ok", f"{label} origin unhealthy: {resp}"
          assert resp["node"] == label, f"{label} wrong identity: {resp}"
      node1.log("All origin HTTP services healthy")

      wait_for_healthy(node1)

      # Form a 3-node cluster.
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
      node1.log("3-node Raft cluster formed")

      # Identify leader / followers.
      metrics = cli(node1, "cluster metrics")
      leader_id = metrics.get("current_leader")
      leader_node = {1: node1, 2: node2, 3: node3}[leader_id]
      leader_ticket = get_ticket(leader_node)
      follower_items = [
          (nid, nref)
          for nid, nref in [(1, node1), (2, node2), (3, node3)]
          if nid != leader_id
      ]
      node1.log(f"Leader is node{leader_id}")

      # Verify all proxy handlers are registered.
      for label, node in [("node1", node1), ("node2", node2), ("node3", node3)]:
          node.succeed(
              "journalctl -u aspen-node --no-pager"
              " | grep -q 'HTTP proxy protocol handler registered'"
          )
      node1.log("Proxy protocol handler registered on all 3 nodes")

      # Collect tickets for each node.
      ticket1 = get_ticket(node1)
      ticket2 = get_ticket(node2)
      ticket3 = get_ticket(node3)

      # ================================================================
      # 1. TCP TUNNEL THROUGH EACH CLUSTER NODE
      # ================================================================

      with subtest("tcp tunnel through node1 to its own origin"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port,
                           "/tmp/proxy-n1-local.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/from-node1")
          assert resp["status"] == "ok", f"tunnel to node1 failed: {resp}"
          assert resp["node"] == "node1", f"wrong node: {resp}"
          assert resp["path"] == "/from-node1", f"wrong path: {resp}"
          client.log("TCP tunnel through node1 → node1:8081 works")

      with subtest("tcp tunnel through node2 to its own origin"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket2,
                           f"127.0.0.1:{HTTP_PORT2}", port,
                           "/tmp/proxy-n2-local.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/from-node2")
          assert resp["status"] == "ok", f"tunnel to node2 failed: {resp}"
          assert resp["node"] == "node2", f"wrong node: {resp}"
          client.log("TCP tunnel through node2 → node2:8082 works")

      with subtest("tcp tunnel through node3 to its own origin"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket3,
                           f"127.0.0.1:{HTTP_PORT3}", port,
                           "/tmp/proxy-n3-local.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/from-node3")
          assert resp["status"] == "ok", f"tunnel to node3 failed: {resp}"
          assert resp["node"] == "node3", f"wrong node: {resp}"
          client.log("TCP tunnel through node3 → node3:8083 works")

      kill_proxies(client)

      # ================================================================
      # 2. CROSS-NODE TUNNELING
      # ================================================================
      # Tunnel through node1's proxy but target a service on node2/node3.
      # The upstream proxy on node1 forwards to the target address; since
      # all nodes are on the same VLAN, node1 can reach node2's HTTP port.

      with subtest("cross-node: tunnel via node1 to node2 origin"):
          # NixOS test framework makes node names DNS-resolvable.
          port = alloc_port()
          start_tcp_tunnel(client, ticket1,
                           f"node2:{HTTP_PORT2}", port,
                           "/tmp/proxy-cross-n1-n2.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/cross-n2")
          assert resp["status"] == "ok", f"cross-node tunnel failed: {resp}"
          assert resp["node"] == "node2", f"expected node2, got {resp}"
          client.log("Cross-node tunnel: client → node1 proxy → node2:8082")

      with subtest("cross-node: tunnel via node2 to node3 origin"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket2,
                           f"node3:{HTTP_PORT3}", port,
                           "/tmp/proxy-cross-n2-n3.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/cross-n3")
          assert resp["status"] == "ok", f"cross-node tunnel failed: {resp}"
          assert resp["node"] == "node3", f"expected node3, got {resp}"
          client.log("Cross-node tunnel: client → node2 proxy → node3:8083")

      with subtest("cross-node: tunnel via node3 to node1 origin"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket3,
                           f"node1:{HTTP_PORT1}", port,
                           "/tmp/proxy-cross-n3-n1.log")
          resp = curl_json(client, f"http://127.0.0.1:{port}/cross-n1")
          assert resp["status"] == "ok", f"cross-node tunnel failed: {resp}"
          assert resp["node"] == "node1", f"expected node1, got {resp}"
          client.log("Cross-node tunnel: client → node3 proxy → node1:8081")

      kill_proxies(client)

      # ================================================================
      # 3. PROXY DURING KV OPERATIONS
      # ================================================================
      # Verify the proxy tunnel works while the cluster is handling KV writes.

      with subtest("proxy works while kv writes are in progress"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port,
                           "/tmp/proxy-kv-concurrent.log")

          # Issue KV writes on the cluster in parallel with proxy requests.
          for i in range(5):
              cli(node1, f"kv set proxy-test:key{i} value{i}",
                  ticket=leader_ticket)
              resp = curl_json(client, f"http://127.0.0.1:{port}/kv-iter-{i}")
              assert resp["status"] == "ok", f"proxy during kv write {i}: {resp}"

          # Verify KV data was written correctly.
          for i in range(5):
              out = cli(node1, f"kv get proxy-test:key{i}",
                        ticket=leader_ticket)
              assert out.get("value") == f"value{i}", f"kv data lost: {out}"

          client.log("Proxy operates correctly during concurrent KV writes")

      kill_proxies(client)

      # ================================================================
      # 4. HTTP FORWARD PROXY THROUGH CLUSTER NODES
      # ================================================================

      with subtest("forward proxy through node1"):
          port = alloc_port()
          start_forward_proxy(client, ticket1, port,
                              "/tmp/proxy-fwd-n1.log")

          # Request through forward proxy to node1's origin.
          resp = curl_proxy_json(
              client,
              f"http://127.0.0.1:{HTTP_PORT1}/fwd-n1",
              f"127.0.0.1:{port}"
          )
          assert resp["status"] == "ok", f"forward proxy failed: {resp}"
          assert resp["node"] == "node1", f"wrong node: {resp}"
          client.log("HTTP forward proxy through node1 works")

      kill_proxies(client)

      with subtest("forward proxy through node2"):
          port = alloc_port()
          start_forward_proxy(client, ticket2, port,
                              "/tmp/proxy-fwd-n2.log")

          resp = curl_proxy_json(
              client,
              f"http://127.0.0.1:{HTTP_PORT2}/fwd-n2",
              f"127.0.0.1:{port}"
          )
          assert resp["status"] == "ok", f"forward proxy failed: {resp}"
          assert resp["node"] == "node2", f"wrong node: {resp}"
          client.log("HTTP forward proxy through node2 works")

      kill_proxies(client)

      # ================================================================
      # 5. CONCURRENT PROXY STREAMS THROUGH MULTIPLE NODES
      # ================================================================

      with subtest("concurrent tunnels through all 3 nodes"):
          port_a = alloc_port()
          port_b = alloc_port()
          port_c = alloc_port()

          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port_a,
                           "/tmp/proxy-multi-a.log")
          start_tcp_tunnel(client, ticket2,
                           f"127.0.0.1:{HTTP_PORT2}", port_b,
                           "/tmp/proxy-multi-b.log")
          start_tcp_tunnel(client, ticket3,
                           f"127.0.0.1:{HTTP_PORT3}", port_c,
                           "/tmp/proxy-multi-c.log")

          # Fire parallel requests through all three tunnels.
          for i in range(3):
              client.succeed(
                  f"curl -sf --max-time 10 "
                  f"'http://127.0.0.1:{port_a}/multi-a-{i}' "
                  f">/tmp/multi-a-{i}.json 2>/dev/null &"
              )
              client.succeed(
                  f"curl -sf --max-time 10 "
                  f"'http://127.0.0.1:{port_b}/multi-b-{i}' "
                  f">/tmp/multi-b-{i}.json 2>/dev/null &"
              )
              client.succeed(
                  f"curl -sf --max-time 10 "
                  f"'http://127.0.0.1:{port_c}/multi-c-{i}' "
                  f">/tmp/multi-c-{i}.json 2>/dev/null &"
              )

          # Wait and verify.
          time.sleep(6)
          for prefix, expected_node in [("a", "node1"), ("b", "node2"),
                                         ("c", "node3")]:
              for i in range(3):
                  raw = client.succeed(f"cat /tmp/multi-{prefix}-{i}.json")
                  resp = json.loads(raw)
                  assert resp["status"] == "ok", \
                      f"multi-{prefix}-{i} failed: {resp}"
                  assert resp["node"] == expected_node, \
                      f"multi-{prefix}-{i} wrong node: {resp}"

          client.log("9 concurrent requests through 3 tunnels all succeeded")

      kill_proxies(client)

      # ================================================================
      # 6. PROXY SURVIVES LEADER FAILOVER
      # ================================================================

      with subtest("setup: tunnel through a follower before failover"):
          # Pick a follower node to tunnel through.
          follower_id, follower_node = follower_items[0]
          follower_ticket = get_ticket(follower_node)
          follower_port = {1: HTTP_PORT1, 2: HTTP_PORT2, 3: HTTP_PORT3}[follower_id]
          failover_proxy_port = alloc_port()

          start_tcp_tunnel(
              client, follower_ticket,
              f"127.0.0.1:{follower_port}", failover_proxy_port,
              "/tmp/proxy-failover.log",
          )

          resp = curl_json(
              client,
              f"http://127.0.0.1:{failover_proxy_port}/pre-failover"
          )
          assert resp["status"] == "ok", f"pre-failover proxy failed: {resp}"
          client.log(f"Pre-failover tunnel through node{follower_id} works")

      with subtest("stop leader"):
          leader_node.succeed("systemctl stop aspen-node.service")
          leader_node.log(f"Stopped leader node{leader_id}")
          time.sleep(8)

      with subtest("proxy through follower still works after leader stop"):
          # The tunnel was through a follower, so it should survive.
          resp = curl_json(
              client,
              f"http://127.0.0.1:{failover_proxy_port}/post-failover"
          )
          assert resp["status"] == "ok", \
              f"post-failover proxy through follower failed: {resp}"
          client.log("Proxy through follower survived leader failover")

      with subtest("verify new leader elected"):
          new_leader = None
          for nid, nref in follower_items:
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
          client.log(f"New leader is node{new_leader}")

      with subtest("new tunnel through new leader works"):
          new_leader_port = {1: HTTP_PORT1, 2: HTTP_PORT2, 3: HTTP_PORT3}[new_leader]
          port = alloc_port()
          start_tcp_tunnel(
              client, new_ticket,
              f"127.0.0.1:{new_leader_port}", port,
              "/tmp/proxy-new-leader.log",
          )
          resp = curl_json(client, f"http://127.0.0.1:{port}/new-leader")
          assert resp["status"] == "ok", f"new leader tunnel failed: {resp}"
          assert resp["node"] == f"node{new_leader}", f"wrong node: {resp}"
          client.log(f"Tunnel through new leader node{new_leader} works")

      kill_proxies(client)

      # ================================================================
      # 7. PROXY TO RESTARTED NODE
      # ================================================================

      with subtest("restart old leader"):
          leader_node.succeed("systemctl start aspen-node.service")
          leader_node.wait_for_unit("aspen-node.service")
          leader_node.wait_for_file(
              "/var/lib/aspen/cluster-ticket.txt", timeout=30
          )
          time.sleep(8)
          leader_node.log(f"node{leader_id} restarted")

      with subtest("tunnel through restarted node works"):
          restarted_ticket = get_ticket(leader_node)
          restarted_port = {1: HTTP_PORT1, 2: HTTP_PORT2, 3: HTTP_PORT3}[leader_id]
          port = alloc_port()
          start_tcp_tunnel(
              client, restarted_ticket,
              f"127.0.0.1:{restarted_port}", port,
              "/tmp/proxy-restarted.log",
          )
          resp = curl_json(client, f"http://127.0.0.1:{port}/restarted")
          assert resp["status"] == "ok", f"restarted node tunnel failed: {resp}"
          assert resp["node"] == f"node{leader_id}", f"wrong node: {resp}"
          client.log(f"Tunnel through restarted node{leader_id} works")

      kill_proxies(client)

      # ================================================================
      # 8. LARGE PAYLOAD TRANSFER THROUGH PROXY
      # ================================================================

      # Refresh tickets — node1 was restarted during failover and may
      # have a new endpoint address / port in its ticket.
      ticket1 = get_ticket(node1)
      ticket2 = get_ticket(node2)
      ticket3 = get_ticket(node3)

      with subtest("large payload through tunnel"):
          port = alloc_port()
          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port,
                           "/tmp/proxy-large.log")

          # Start with a small request to warm the tunnel connection.
          resp = curl_json(client, f"http://127.0.0.1:{port}/warmup")
          assert resp["status"] == "ok", f"warmup failed: {resp}"

          # Request a ~10KB JSON response through the tunnel.
          # (Kept moderate — napkin warns about large data in VM tests.)
          raw = client.succeed(
              f"curl -sf --max-time 30 "
              f"'http://127.0.0.1:{port}/large?size=10000' 2>/dev/null"
          )
          resp = json.loads(raw)
          assert resp["status"] == "ok", f"large payload failed: {resp}"
          payload_len = len(resp.get("payload", ""))
          assert payload_len >= 9000, \
              f"payload truncated: got {payload_len} chars"
          client.log(f"Large payload ({payload_len} chars) through tunnel OK")

      with subtest("multiple large payloads sequentially"):
          for i in range(3):
              raw = client.succeed(
                  f"curl -sf --max-time 30 "
                  f"'http://127.0.0.1:{port}/large-seq-{i}?size=5000' "
                  f"2>/dev/null"
              )
              resp = json.loads(raw)
              assert resp["status"] == "ok", f"large seq {i} failed: {resp}"
              assert len(resp.get("payload", "")) >= 4000, \
                  f"large seq {i} truncated"
          client.log("3 sequential large payloads through tunnel OK")

      kill_proxies(client)

      # ================================================================
      # 9. MULTIPLE PROXY TARGETS ON THE SAME NODE
      # ================================================================

      with subtest("two tunnels to different ports on node1"):
          port_main = alloc_port()
          port_alt = alloc_port()

          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port_main,
                           "/tmp/proxy-multi-target-main.log")
          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1_ALT}", port_alt,
                           "/tmp/proxy-multi-target-alt.log")

          resp_main = curl_json(
              client, f"http://127.0.0.1:{port_main}/main-service"
          )
          resp_alt = curl_json(
              client, f"http://127.0.0.1:{port_alt}/alt-service"
          )

          assert resp_main["node"] == "node1", f"main wrong: {resp_main}"
          assert resp_alt["node"] == "node1-alt", f"alt wrong: {resp_alt}"
          assert resp_main["port"] == HTTP_PORT1, f"main port: {resp_main}"
          assert resp_alt["port"] == HTTP_PORT1_ALT, f"alt port: {resp_alt}"
          client.log("Two tunnels to different services on node1 work")

      with subtest("interleaved requests across two targets"):
          for i in range(4):
              resp_m = curl_json(
                  client, f"http://127.0.0.1:{port_main}/interleave-{i}"
              )
              resp_a = curl_json(
                  client, f"http://127.0.0.1:{port_alt}/interleave-{i}"
              )
              assert resp_m["node"] == "node1", \
                  f"interleave main {i}: {resp_m}"
              assert resp_a["node"] == "node1-alt", \
                  f"interleave alt {i}: {resp_a}"
          client.log("Interleaved requests across two targets succeed")

      kill_proxies(client)

      # ================================================================
      # 10. TUNNEL RESTART RESILIENCE (MULTI-NODE)
      # ================================================================

      with subtest("tunnel restart through different nodes"):
          # Start tunnel through node1, verify, kill, restart through node2.
          port_a = alloc_port()
          start_tcp_tunnel(client, ticket1,
                           f"127.0.0.1:{HTTP_PORT1}", port_a,
                           "/tmp/proxy-restart-a.log")
          resp = curl_json(client, f"http://127.0.0.1:{port_a}/restart-a")
          assert resp["node"] == "node1", f"restart-a wrong: {resp}"

          kill_proxies(client)
          time.sleep(2)

          # Restart through node2 on a new port.
          port_b = alloc_port()
          start_tcp_tunnel(client, ticket2,
                           f"127.0.0.1:{HTTP_PORT2}", port_b,
                           "/tmp/proxy-restart-b.log")
          resp = curl_json(client, f"http://127.0.0.1:{port_b}/restart-b")
          assert resp["node"] == "node2", f"restart-b wrong: {resp}"

          kill_proxies(client)
          time.sleep(2)

          # Restart through node3 on yet another port.
          port_c = alloc_port()
          start_tcp_tunnel(client, ticket3,
                           f"127.0.0.1:{HTTP_PORT3}", port_c,
                           "/tmp/proxy-restart-c.log")
          resp = curl_json(client, f"http://127.0.0.1:{port_c}/restart-c")
          assert resp["node"] == "node3", f"restart-c wrong: {resp}"

          client.log("Tunnel restart through node1 → node2 → node3 works")

      kill_proxies(client)

      # ── done ─────────────────────────────────────────────────────────
      node1.log("=== multi-node-proxy test complete ===")
    '';
  }
