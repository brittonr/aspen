# NixOS VM integration test for the aspen-net service mesh.
#
# Tests the service mesh end-to-end with a single-node cluster:
#   - Service publish/lookup via CLI
#   - SOCKS5 proxy name resolution and TCP tunneling via iroh QUIC
#   - Cross-service routing
#
# Run:
#   nix build .#checks.x86_64-linux.net-service-mesh-test --impure
#
# Interactive:
#   nix build .#checks.x86_64-linux.net-service-mesh-test.driverInteractive --impure
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  aspenNetPackage,
  kvPluginWasm,
}: let
  # Deterministic Iroh secret key
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";
  cookie = "net-mesh-vm-test";

  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "kv";
        wasm = kvPluginWasm;
      }
    ];
  };
in
  pkgs.testers.runNixOSTest {
    name = "net-service-mesh-test";

    nodes = {
      node1 = {
        imports = [
          ../../nix/modules/aspen-node.nix
          pluginHelpers.nixosConfig
        ];

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
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [
          aspenCliPackage
          aspenNetPackage
          pkgs.python3
          pkgs.curl
        ];

        networking.firewall.enable = false;
        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      CLI = "${aspenCliPackage}/bin/aspen-cli"

      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket."""
          ticket = get_ticket()
          run = (
              f"{CLI} --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node1.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"{CLI} --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      # ================================================================
      # Phase 1: Cluster bootstrap
      # ================================================================
      start_all()

      with subtest("Wait for cluster"):
          node1.wait_for_unit("aspen-node.service")
          node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          node1.wait_until_succeeds(
              f"{CLI} --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
              timeout=60,
          )
          cli_text("cluster init")
          time.sleep(2)

      with subtest("Install KV plugin"):
          ${pluginHelpers.installPluginsScript}

      # ================================================================
      # Phase 2: Start HTTP service
      # ================================================================
      with subtest("Start HTTP server"):
          node1.succeed("mkdir -p /tmp/www")
          node1.succeed("echo 'hello from aspen net' > /tmp/www/test.txt")
          node1.succeed(
              "cd /tmp/www && python3 -m http.server 8080 &"
          )
          node1.wait_until_succeeds("curl -sf http://127.0.0.1:8080/test.txt", timeout=10)
          node1.log("HTTP server running on port 8080")

      # ================================================================
      # Phase 3: Publish service
      # ================================================================
      with subtest("Get endpoint ID"):
          # Read the endpoint ID from the cluster ticket (the node's iroh identity)
          ticket = get_ticket()
          # The endpoint ID is embedded in the ticket; extract via cluster status
          status = cli("cluster status")
          node1.log(f"Cluster status: {status}")
          # Use the node's endpoint_id from the ticket file
          endpoint_id = node1.succeed(
              f"{CLI} --ticket '{ticket}' --json cluster status 2>/dev/null "
              "| python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get(\"endpoint_id\", d.get(\"nodes\", [{}])[0].get(\"endpoint_id\", \"unknown\")))'"
          ).strip()
          node1.log(f"Endpoint ID: {endpoint_id}")

      with subtest("Publish HTTP service"):
          result = cli(f"net publish my-http --endpoint-id {endpoint_id} --port 8080 --proto tcp --tag web")
          node1.log(f"Publish result: {result}")

      with subtest("Verify service listing"):
          result = cli("net services")
          node1.log(f"Services: {result}")
          services = result if isinstance(result, dict) else {}
          svc_list = services.get("services", [])
          names = [s.get("name", "") for s in svc_list]
          assert "my-http" in names, f"my-http not in services: {svc_list}"

      with subtest("Verify service lookup"):
          result = cli("net lookup my-http")
          node1.log(f"Lookup result: {result}")

      # ================================================================
      # Phase 4: Start SOCKS5 proxy daemon
      # ================================================================
      with subtest("Start aspen-net daemon"):
          ticket = get_ticket()
          # Start daemon in background with SOCKS5 on port 1080
          node1.succeed(
              f"${aspenNetPackage}/bin/aspen-net up "
              f"--ticket '{ticket}' "
              f"--socks5-addr 127.0.0.1:1080 --no-dns "
              f">/tmp/aspen-net.log 2>&1 &"
          )
          # Wait for SOCKS5 to be listening
          node1.wait_until_succeeds("ss -tlnp | grep ':1080'", timeout=30)
          node1.log("SOCKS5 proxy listening on port 1080")

      # ================================================================
      # Phase 5: Route traffic through SOCKS5 proxy
      # ================================================================
      with subtest("SOCKS5 tunnel to HTTP service"):
          # curl through SOCKS5 proxy, addressing service by name
          result = node1.succeed(
              "curl -sf --socks5-hostname 127.0.0.1:1080 "
              "http://my-http.aspen:8080/test.txt",
              timeout=30,
          )
          assert "hello from aspen net" in result, \
              f"Expected 'hello from aspen net' in response, got: {result}"
          node1.log("SOCKS5 tunnel routing works!")

      # ================================================================
      # Phase 6: Unpublish and verify failure
      # ================================================================
      with subtest("Unpublish service"):
          result = cli("net unpublish my-http")
          node1.log(f"Unpublish result: {result}")

      with subtest("SOCKS5 fails after unpublish"):
          # Give time for cache to expire
          time.sleep(12)
          exit_code = node1.execute(
              "curl -sf --socks5-hostname 127.0.0.1:1080 "
              "http://my-http.aspen:8080/test.txt "
              ">/dev/null 2>&1"
          )[0]
          assert exit_code != 0, "curl should fail after service unpublished"
          node1.log("SOCKS5 correctly rejects unpublished service")

      # ================================================================
      # Phase 7: Multi-service test
      # ================================================================
      with subtest("Publish second service"):
          node1.succeed("echo 'second service' > /tmp/www/second.txt")
          cli(f"net publish my-http --endpoint-id {endpoint_id} --port 8080 --proto tcp")
          cli(f"net publish my-api --endpoint-id {endpoint_id} --port 8080 --proto tcp --tag api")

      with subtest("Both services resolve"):
          result = cli("net services")
          svc_list = result.get("services", []) if isinstance(result, dict) else []
          names = [s.get("name", "") for s in svc_list]
          assert "my-http" in names, f"my-http not in services: {names}"
          assert "my-api" in names, f"my-api not in services: {names}"

      # ================================================================
      # Cleanup
      # ================================================================
      with subtest("Cleanup"):
          node1.execute("kill %1 2>/dev/null || true")  # python HTTP server
          node1.execute("pkill -f 'aspen-net up' 2>/dev/null || true")
    '';
  }
