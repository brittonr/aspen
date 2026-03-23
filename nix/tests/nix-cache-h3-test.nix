# NixOS VM integration test for Nix binary cache over HTTP/3 (iroh QUIC).
#
# Tests the full H3 round-trip:
#   1. Start a single-node Aspen cluster
#   2. Start aspen-nix-cache-gateway with --h3 (HTTP/3 over iroh QUIC)
#   3. Parse endpoint ID and direct address from gateway logs
#   4. Start aspen-h3-proxy bridging HTTP/1.1 → H3
#   5. Verify /nix-cache-info through the proxy
#   6. Verify 404 for non-existent narinfo through the proxy
#
# Run:
#   nix build .#checks.x86_64-linux.nix-cache-h3-test
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  gatewayH3Package,
  h3ProxyPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "nix-cache-h3-test";
in
  pkgs.testers.nixosTest {
    name = "nix-cache-h3";

    nodes = {
      machine = {
        imports = [../../nix/modules/aspen-node.nix];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = secretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info";
          relayMode = "disabled";
          enableWorkers = true;
          enableCi = true;
          features = [];
        };

        environment.systemPackages = [
          aspenCliPackage
          gatewayH3Package
          h3ProxyPackage
          pkgs.curl
          pkgs.gnugrep
          pkgs.gnused
        ];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import re
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return machine.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          machine.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return machine.succeed("cat /tmp/_cli_out.txt")

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      machine.wait_for_unit("aspen-node.service")
      machine.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      machine.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) --timeout 5000 cluster health",
          timeout=60,
      )

      cli_text("cluster init")
      time.sleep(2)

      # ── start gateway in H3 mode ─────────────────────────────────────

      with subtest("start nix cache gateway with --h3"):
          ticket = get_ticket()
          machine.succeed(
              f"systemd-run --unit=nix-cache-gw-h3 "
              f"aspen-nix-cache-gateway --ticket '{ticket}' --h3 "
              f"--cache-name test-cache"
          )
          # Wait for gateway to log its endpoint ID
          machine.wait_until_succeeds(
              "journalctl -u nix-cache-gw-h3 --no-pager | grep 'nix cache gateway listening'",
              timeout=30,
          )
          machine.log("H3 gateway started")

      # ── parse endpoint ID and direct address from logs ──────────────

      with subtest("parse gateway endpoint info"):
          log_output = machine.succeed(
              "journalctl -u nix-cache-gw-h3 --no-pager"
          )
          machine.log(f"Gateway logs:\n{log_output}")

          # Extract endpoint_id from the tracing log line
          # Format: endpoint_id=<base32key>
          m = re.search(r'endpoint_id=(\S+)', log_output)
          assert m is not None, "could not find endpoint_id in logs"
          endpoint_id = m.group(1)
          machine.log(f"Parsed endpoint_id: {endpoint_id}")

          # Extract direct address from the tracing log line
          # Format: direct_addrs=["127.0.0.1:PORT", ...]
          # The tracing debug format uses square brackets
          m = re.search(r'direct_addrs=\[([^\]]*)\]', log_output)
          assert m is not None, "could not find direct_addrs in logs"
          addrs_raw = m.group(1)
          machine.log(f"Raw direct_addrs: {addrs_raw}")

          # Parse out the first IPv4 address (skip IPv6 for simplicity)
          # Addresses are quoted strings like "ip:10.0.2.15:38848"
          ipv4_addrs = re.findall(r'"ip:((?:\d+\.){3}\d+:\d+)"', addrs_raw)
          assert len(ipv4_addrs) > 0, f"no IPv4 direct addresses found in: {addrs_raw}"
          direct_addr = ipv4_addrs[0]
          machine.log(f"Using direct_addr: {direct_addr}")

      # ── start h3-proxy bridging to gateway ───────────────────────────

      with subtest("start h3 proxy"):
          machine.succeed(
              f"systemd-run --unit=h3-proxy "
              f"aspen-h3-proxy "
              f"--endpoint-id '{endpoint_id}' "
              f"--alpn 'iroh+h3' "
              f"--port 8381 "
              f"--direct-addr '{direct_addr}' "
              f"--timeout-secs 30"
          )
          # Give the proxy time to bind
          time.sleep(2)

          # Verify the proxy is listening
          machine.wait_until_succeeds(
              "curl -sf http://127.0.0.1:8381/nix-cache-info",
              timeout=60,
          )
          machine.log("H3 proxy is forwarding requests")

      # ── verify /nix-cache-info through the H3 path ──────────────────

      with subtest("nix-cache-info via H3 proxy"):
          info = machine.succeed("curl -sf http://127.0.0.1:8381/nix-cache-info")
          machine.log(f"nix-cache-info via H3: {info}")
          assert "StoreDir: /nix/store" in info, f"missing StoreDir: {info}"
          assert "WantMassQuery: 1" in info, f"missing WantMassQuery: {info}"
          assert "Priority: 30" in info, f"missing Priority: {info}"

      # ── verify 404 for non-existent narinfo through H3 ──────────────

      with subtest("narinfo 404 via H3 proxy"):
          code = machine.succeed(
              "curl -s -o /dev/null -w '%{http_code}' "
              "http://127.0.0.1:8381/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.narinfo"
          ).strip()
          machine.log(f"narinfo 404 via H3: HTTP {code}")
          # nar-bridge returns 404 for unknown paths
          assert code == "404", f"expected HTTP 404, got {code}"

      # ── verify unknown path returns 404 through H3 ──────────────────

      with subtest("unknown path 404 via H3 proxy"):
          code = machine.succeed(
              "curl -s -o /dev/null -w '%{http_code}' "
              "http://127.0.0.1:8381/unknown/path"
          ).strip()
          machine.log(f"unknown path via H3: HTTP {code}")
          assert code == "404", f"expected HTTP 404, got {code}"

      # ── done ─────────────────────────────────────────────────────────
      machine.log("All H3 nix cache integration tests passed!")
    '';
  }
