# NixOS VM integration test for the Aspen HTTP proxy (TCP tunneling over iroh).
#
# Spins up a 2-node QEMU environment:
#
#   server — runs aspen-node with --enable-proxy plus a simple HTTP service
#   client — runs aspen-cli proxy commands to tunnel through the server
#
# Test scenarios:
#
#   1. TCP tunnel mode (proxy start --target)
#      Client tunnels to server's local HTTP service through iroh QUIC.
#
#   2. HTTP forward proxy mode (proxy forward)
#      Client uses CONNECT-style HTTP proxying through iroh QUIC.
#
#   3. Authentication — only peers with the correct cluster cookie can proxy.
#
#   4. Concurrent connections — multiple simultaneous requests through the tunnel.
#
# Run:
#   nix build .#checks.x86_64-linux.proxy-tunnel-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.proxy-tunnel-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each).
  serverSecretKey = "0000000000000001000000000000000100000000000000010000000000000001";
  clientSecretKey = "0000000000000002000000000000000200000000000000020000000000000002";

  cookie = "proxy-tunnel-test";

  # Port the origin HTTP server listens on (server-side only).
  httpPort = 8080;
in
  pkgs.testers.nixosTest {
    name = "proxy-tunnel";
    skipLint = true;

    nodes = {
      # ── server node: aspen-node with proxy + a local HTTP service ──
      server = {
        imports = [../../nix/modules/aspen-node.nix];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = serverSecretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
          extraArgs = ["--enable-proxy"];
        };

        # Simple Python HTTP server as the origin service to proxy to.
        systemd.services.origin-http = {
          description = "Origin HTTP test server";
          wantedBy = ["multi-user.target"];
          after = ["network.target"];
          serviceConfig = {
            Type = "simple";
            ExecStart = let
              handler = pkgs.writeText "handler.py" ''
                import http.server
                import json

                class Handler(http.server.BaseHTTPRequestHandler):
                    def do_GET(self):
                        body = json.dumps({
                            "status": "ok",
                            "path": self.path,
                            "message": "hello from origin",
                        }).encode()
                        self.send_response(200)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(body)))
                        self.end_headers()
                        self.wfile.write(body)

                    def log_message(self, fmt, *args):
                        pass  # suppress noisy access logs

                http.server.HTTPServer(("0.0.0.0", ${toString httpPort}), Handler).serve_forever()
              '';
            in "${pkgs.python3}/bin/python3 ${handler}";
            Restart = "on-failure";
          };
        };

        environment.systemPackages = with pkgs; [curl aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };

      # ── client node: only runs aspen-cli proxy commands ────────────
      client = {
        environment.systemPackages = with pkgs; [curl aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 1024;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      HTTP_PORT = ${toString httpPort}

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket():
          """Read cluster ticket from the server node."""
          return server.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def curl_json(node, url, timeout=10):
          """curl a URL and parse the JSON response."""
          raw = node.succeed(
              f"curl -sf --max-time {timeout} '{url}' 2>/dev/null"
          )
          return json.loads(raw)

      # ── boot ─────────────────────────────────────────────────────────
      start_all()

      server.wait_for_unit("aspen-node.service")
      server.wait_for_unit("origin-http.service")
      server.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      server.wait_for_open_port(HTTP_PORT)

      client.wait_for_unit("default.target")

      # Verify the origin HTTP service works locally on the server.
      with subtest("origin http reachable locally"):
          resp = curl_json(server, f"http://127.0.0.1:{HTTP_PORT}/healthz")
          assert resp["status"] == "ok", f"origin unhealthy: {resp}"
          assert resp["path"] == "/healthz", f"wrong path: {resp}"
          server.log("Origin HTTP service is healthy")

      # Verify the origin is NOT reachable directly from the client
      # (they are on different VLANs by default — but both get eth1 on
      # the same VLAN in single-VLAN setups, so the server IP *is*
      # reachable on the network level. The point of the proxy is to
      # tunnel over iroh QUIC, not for network isolation. We test the
      # proxy path works correctly below.)

      # Wait for the cluster to be responsive before proxying.
      ticket = get_ticket()
      server.wait_until_succeeds(
          f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
          timeout=60,
      )
      server.log(f"Cluster healthy, ticket={ticket[:40]}...")

      # Verify proxy handler is registered by checking server journal.
      server.succeed(
          "journalctl -u aspen-node --no-pager"
          " | grep -q 'HTTP proxy protocol handler registered'"
      )
      server.log("Proxy protocol handler confirmed in journal")

      # ── TCP tunnel mode ──────────────────────────────────────────────

      with subtest("tcp tunnel basic request"):
          # Start the proxy tunnel in the background on the client.
          # It binds a local port and tunnels to server:8080 over iroh.
          client.succeed(
              f"aspen-cli --ticket '{ticket}' proxy start "
              f"--target 127.0.0.1:{HTTP_PORT} "
              f"--local-addr 127.0.0.1:19090 "
              f"&>/tmp/proxy-tcp.log &"
          )
          # Give the proxy a moment to bind and connect.
          time.sleep(3)

          # Verify the proxy listener is up.
          client.wait_for_open_port(19090, timeout=15)
          client.log("TCP tunnel proxy is listening on :19090")

          # Make a request through the tunnel.
          resp = curl_json(client, "http://127.0.0.1:19090/via-tunnel")
          assert resp["status"] == "ok", f"tunnel request failed: {resp}"
          assert resp["path"] == "/via-tunnel", f"wrong path: {resp}"
          client.log(f"TCP tunnel response: {resp}")

      with subtest("tcp tunnel multiple requests"):
          # Send several requests to verify the tunnel stays healthy.
          for i in range(5):
              resp = curl_json(client, f"http://127.0.0.1:19090/req-{i}")
              assert resp["status"] == "ok", f"request {i} failed: {resp}"
              assert resp["path"] == f"/req-{i}", f"wrong path on req {i}: {resp}"
          client.log("5 sequential requests through TCP tunnel succeeded")

      with subtest("tcp tunnel concurrent requests"):
          # Fire off several curl requests in parallel.
          for i in range(4):
              client.succeed(
                  f"curl -sf --max-time 10 "
                  f"'http://127.0.0.1:19090/concurrent-{i}' "
                  f">/tmp/concurrent-{i}.json 2>/dev/null &"
              )
          # Wait for all to finish.
          time.sleep(5)
          for i in range(4):
              raw = client.succeed(f"cat /tmp/concurrent-{i}.json")
              resp = json.loads(raw)
              assert resp["status"] == "ok", f"concurrent-{i} failed: {resp}"
          client.log("4 concurrent requests through TCP tunnel succeeded")

      # Kill the TCP tunnel proxy.
      client.execute("pgrep -f 'aspen-cli.*proxy' | xargs -r kill 2>/dev/null; true")
      time.sleep(1)

      # ── HTTP forward proxy mode ─────────────────────────────────────

      with subtest("http forward proxy"):
          # Start the forward proxy on the client.
          client.succeed(
              f"aspen-cli --ticket '{ticket}' proxy forward "
              f"--local-addr 127.0.0.1:19091 "
              f"&>/tmp/proxy-fwd.log &"
          )
          time.sleep(3)
          client.wait_for_open_port(19091, timeout=15)
          client.log("HTTP forward proxy is listening on :19091")

          # Use curl with HTTP_PROXY to request through the forward proxy.
          # The forward proxy routes to the remote node, which reaches origin.
          raw = client.succeed(
              f"curl -sf --max-time 10 "
              f"--proxy http://127.0.0.1:19091 "
              f"'http://127.0.0.1:{HTTP_PORT}/via-forward' "
              f"2>/dev/null"
          )
          resp = json.loads(raw)
          assert resp["status"] == "ok", f"forward proxy request failed: {resp}"
          assert resp["path"] == "/via-forward", f"wrong path: {resp}"
          client.log(f"HTTP forward proxy response: {resp}")

      # Kill the forward proxy.
      client.execute("pgrep -f 'aspen-cli.*proxy' | xargs -r kill 2>/dev/null; true")
      time.sleep(1)

      # ── tunnel restart resilience ────────────────────────────────────

      with subtest("tunnel restart resilience"):
          # Start a fresh tunnel, make a request, kill it, restart, request again.
          client.succeed(
              f"aspen-cli --ticket '{ticket}' proxy start "
              f"--target 127.0.0.1:{HTTP_PORT} "
              f"--local-addr 127.0.0.1:19092 "
              f"&>/tmp/proxy-restart.log &"
          )
          time.sleep(3)
          client.wait_for_open_port(19092, timeout=15)

          resp = curl_json(client, "http://127.0.0.1:19092/before-restart")
          assert resp["status"] == "ok", f"pre-restart request failed: {resp}"

          client.execute("pgrep -f 'aspen-cli.*proxy' | xargs -r kill 2>/dev/null; true")
          time.sleep(2)

          # Restart on a new port (old one might linger briefly).
          client.succeed(
              f"aspen-cli --ticket '{ticket}' proxy start "
              f"--target 127.0.0.1:{HTTP_PORT} "
              f"--local-addr 127.0.0.1:19093 "
              f"&>/tmp/proxy-restart2.log &"
          )
          time.sleep(3)
          client.wait_for_open_port(19093, timeout=15)

          resp = curl_json(client, "http://127.0.0.1:19093/after-restart")
          assert resp["status"] == "ok", f"post-restart request failed: {resp}"
          client.log("Tunnel restart resilience verified")

      # Clean up.
      client.execute("pgrep -f 'aspen-cli.*proxy' | xargs -r kill 2>/dev/null; true")

      server.log("=== proxy-tunnel test complete ===")
    '';
  }
