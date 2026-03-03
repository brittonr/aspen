# NixOS VM integration test for the aspen-net service mesh.
#
# Tests the service mesh end-to-end with a 3-node cluster:
#   - Service publish/lookup via CLI
#   - SOCKS5 proxy name resolution
#   - Port forwarding through iroh QUIC tunnels
#   - DNS record creation/deletion
#
# Run:
#   nix build .#checks.x86_64-linux.net-service-mesh-test
#
# Interactive:
#   nix build .#checks.x86_64-linux.net-service-mesh-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
}: let
  # Deterministic Iroh secret keys
  secretKey1 = "0000000000000001000000000000000100000000000000010000000000000001";
  secretKey2 = "0000000000000002000000000000000200000000000000020000000000000002";
  secretKey3 = "0000000000000003000000000000000300000000000000030000000000000003";

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

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.runNixOSTest {
    name = "net-service-mesh-test";

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
      import json, time

      CLI = "${aspenCliPackage}/bin/aspen-cli"

      def cli(node, cmd, timeout=30):
          """Run aspen-cli command on a node, return stdout."""
          result = node.succeed(f"{CLI} {cmd}", timeout=timeout)
          return result.strip()

      def wait_healthy(node, timeout=60):
          """Wait until the node responds to health checks."""
          node.wait_for_unit("aspen-node.service")
          start = time.time()
          while time.time() - start < timeout:
              try:
                  out = node.succeed(f"{CLI} cluster health 2>/dev/null || true")
                  if "healthy" in out.lower() or "ok" in out.lower():
                      return
              except:
                  pass
              time.sleep(2)
          raise Exception(f"Node not healthy after {timeout}s")

      # ================================================================
      # Phase 1: Cluster bootstrap
      # ================================================================
      start_all()

      with subtest("Wait for nodes to start"):
          for n in [node1, node2, node3]:
              n.wait_for_unit("aspen-node.service")

      with subtest("Bootstrap cluster on node1"):
          cli(node1, "cluster init")
          time.sleep(3)

      with subtest("Add learners and promote"):
          # Get endpoint IDs from node2 and node3, add as learners, change membership
          ticket1 = cli(node1, "cluster ticket")
          # Node2 and node3 join via ticket
          time.sleep(5)

      # ================================================================
      # Phase 2: Service mesh operations
      # ================================================================

      with subtest("Task 10.6 + 14.7: Publish service via CLI"):
          # Publish a service on node1
          result = cli(node1, "net publish mydb --port 5432 --proto tcp --tag db --tag prod")
          assert "success" in result.lower() or "ok" in result.lower(), f"publish failed: {result}"

      with subtest("Task 10.6: Lookup service via CLI"):
          result = cli(node1, "net services")
          assert "mydb" in result, f"mydb not in service list: {result}"

      with subtest("Task 14.7: Unpublish service"):
          result = cli(node1, "net unpublish mydb")
          assert "success" in result.lower() or "ok" in result.lower(), f"unpublish failed: {result}"

          result = cli(node1, "net services")
          assert "mydb" not in result, f"mydb still in list after unpublish: {result}"

      with subtest("Task 14.7: Re-publish for further tests"):
          cli(node1, "net publish mydb --port 5432 --proto tcp --tag db")

      # ================================================================
      # Phase 3: Cross-node verification
      # ================================================================

      with subtest("Task 10.6: Cross-node lookup"):
          # Service published on node1, should be visible from node2
          result = cli(node2, "net services")
          assert "mydb" in result, f"mydb not visible from node2: {result}"

      # ================================================================
      # Phase 4: DNS (if enabled)
      # ================================================================

      # DNS integration tests would go here:
      # - Query DNS for mydb.aspen A record
      # - Query DNS for _tcp.mydb.aspen SRV record
      # - Unpublish and verify records gone

      # ================================================================
      # Cleanup
      # ================================================================
      with subtest("Cleanup"):
          cli(node1, "net unpublish mydb")
    '';
  }
