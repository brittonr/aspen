# NixOS integration test for Aspen cluster.
#
# This test uses NixOS's testing framework to boot a 3-node Aspen cluster
# in QEMU VMs and verify basic functionality:
#
# 1. All nodes boot successfully
# 2. HTTP APIs are accessible
# 3. Raft cluster initializes
# 4. Key-value operations work
# 5. Nodes can discover each other
#
# Run with:
#   nix build .#checks.x86_64-linux.cluster-test
#
# Or interactively:
#   nix run .#checks.x86_64-linux.cluster-test.driverInteractive
{
  self,
  nixpkgs,
  system ? "x86_64-linux",
}: let
  pkgs = import nixpkgs {inherit system;};

  # Helper function to create a test node configuration
  makeTestNode = {
    nodeId,
    httpPort,
  }: {
    pkgs,
    lib,
    ...
  }: {
    imports = [self.nixosModules.aspen-node];

    # Basic system configuration
    system.stateVersion = lib.trivial.release;

    # Enable aspen-node service
    services.aspen-node = {
      enable = true;
      package = self.packages.${system}.aspen-node;
      nodeId = nodeId;
      httpAddr = "0.0.0.0:${toString httpPort}";
      dataDir = "/var/lib/aspen/node-${toString nodeId}";
      storageBackend = "sqlite";
      cookie = "test-cookie";

      # Enable local network discovery
      iroh = {
        disableMdns = false;
        disableGossip = false;
      };

      environment = {
        RUST_LOG = "info,aspen=debug";
        RUST_BACKTRACE = "1";
      };
    };

    # Open firewall ports
    networking.firewall = {
      enable = true;
      allowedTCPPorts = [httpPort];
      allowedUDPPortRanges = [
        {
          from = 4000;
          to = 4100;
        }
      ];
    };

    # Basic networking
    networking.useDHCP = false;
    networking.interfaces.eth1.useDHCP = true;
  };
in
  pkgs.testers.runNixOSTest {
    name = "aspen-cluster";

    nodes = {
      node1 = makeTestNode {
        nodeId = 1;
        httpPort = 8301;
      };
      node2 = makeTestNode {
        nodeId = 2;
        httpPort = 8302;
      };
      node3 = makeTestNode {
        nodeId = 3;
        httpPort = 8303;
      };
    };

    testScript = ''
      import json
      import time

      # Start all nodes
      start_all()

      # Wait for all nodes to boot
      node1.wait_for_unit("multi-user.target")
      node2.wait_for_unit("multi-user.target")
      node3.wait_for_unit("multi-user.target")

      # Wait for aspen-node service to start
      node1.wait_for_unit("aspen-node.service")
      node2.wait_for_unit("aspen-node.service")
      node3.wait_for_unit("aspen-node.service")

      # Give services time to fully initialize
      time.sleep(5)

      # Test 1: Verify HTTP APIs are accessible
      with subtest("HTTP APIs are accessible"):
          node1.succeed("curl -f http://localhost:8301/health")
          node2.succeed("curl -f http://localhost:8302/health")
          node3.succeed("curl -f http://localhost:8303/health")

      # Test 2: Get node IPs for cluster initialization
      node1_ip = node1.succeed("hostname -I | cut -d' ' -f1").strip()
      node2_ip = node2.succeed("hostname -I | cut -d' ' -f1").strip()
      node3_ip = node3.succeed("hostname -I | cut -d' ' -f1").strip()

      print(f"Node IPs: node1={node1_ip}, node2={node2_ip}, node3={node3_ip}")

      # Test 3: Initialize the Raft cluster from node 1
      with subtest("Initialize Raft cluster"):
          init_payload = json.dumps({
              "initial_members": [
                  {"id": 1, "addr": f"{node1_ip}:8301"},
                  {"id": 2, "addr": f"{node2_ip}:8302"},
                  {"id": 3, "addr": f"{node3_ip}:8303"}
              ]
          })
          node1.succeed(f"curl -f -X POST -H 'Content-Type: application/json' -d '{init_payload}' http://localhost:8301/init")

      # Wait for leader election
      time.sleep(10)

      # Test 4: Verify cluster status
      with subtest("Cluster status shows leader"):
          status = node1.succeed("curl -f http://localhost:8301/cluster/status")
          print(f"Cluster status: {status}")
          # The response should contain a leader_id
          assert "leader_id" in status or "leader" in status.lower(), "No leader found in cluster status"

      # Test 5: Write a key-value pair
      with subtest("Write key-value pair"):
          node1.succeed("curl -f -X PUT -d 'test-value-123' http://localhost:8301/kv/test-key")

      # Test 6: Read back from the same node
      with subtest("Read from writer node"):
          result = node1.succeed("curl -f http://localhost:8301/kv/test-key")
          assert result.strip() == "test-value-123", f"Unexpected value: {result}"

      # Wait for replication
      time.sleep(3)

      # Test 7: Read from other nodes (verify replication)
      with subtest("Read from follower nodes"):
          result2 = node2.succeed("curl -f http://localhost:8302/kv/test-key")
          result3 = node3.succeed("curl -f http://localhost:8303/kv/test-key")
          assert result2.strip() == "test-value-123", f"Node 2 has wrong value: {result2}"
          assert result3.strip() == "test-value-123", f"Node 3 has wrong value: {result3}"

      # Test 8: Write multiple keys
      with subtest("Write multiple keys"):
          for i in range(10):
              node1.succeed(f"curl -f -X PUT -d 'value-{i}' http://localhost:8301/kv/key-{i}")

      # Wait for replication
      time.sleep(5)

      # Test 9: Verify all keys replicated
      with subtest("Verify all keys replicated"):
          for i in range(10):
              result = node3.succeed(f"curl -f http://localhost:8303/kv/key-{i}")
              assert result.strip() == f"value-{i}", f"Key {i} has wrong value: {result}"

      # Test 10: Update an existing key
      with subtest("Update existing key"):
          node2.succeed("curl -f -X PUT -d 'updated-value' http://localhost:8302/kv/test-key")

      time.sleep(3)

      # Test 11: Verify update propagated
      with subtest("Verify update propagated"):
          result = node3.succeed("curl -f http://localhost:8303/kv/test-key")
          assert result.strip() == "updated-value", f"Update not propagated: {result}"

      print("All tests passed!")
    '';
  }
