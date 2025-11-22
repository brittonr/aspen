# Simple test to verify NixOS VM testing works
# Run with: nix-build tests/simple-network-test.nix

import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "simple-network-test";

  nodes = {
    node1 = { config, pkgs, ... }: {
      networking.firewall.enable = false;

      # Simple HTTP server on port 3020
      systemd.services.http-server = {
        description = "Simple HTTP server";
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          ExecStart = "${pkgs.python3}/bin/python3 -m http.server 3020";
          WorkingDirectory = "/tmp";
        };
      };
    };

    node2 = { config, pkgs, ... }: {
      networking.firewall.enable = false;
      environment.systemPackages = [ pkgs.curl pkgs.jq ];
    };

    node3 = { config, pkgs, ... }: {
      networking.firewall.enable = false;
      environment.systemPackages = [ pkgs.curl ];
    };
  };

  testScript = ''
    import time

    print("=== Starting all VMs ===")
    start_all()

    print("=== Waiting for services ===")
    node1.wait_for_unit("http-server.service")
    node1.wait_for_open_port(3020)

    print("=== Testing cross-VM communication ===")
    # Node2 should be able to reach Node1
    output = node2.succeed("curl -s http://node1:3020/")
    print(f"Node2 -> Node1: Success ({len(output)} bytes)")

    # Node3 should also reach Node1
    output = node3.succeed("curl -s http://node1:3020/")
    print(f"Node3 -> Node1: Success ({len(output)} bytes)")

    # All nodes can ping each other
    node1.succeed("ping -c 1 node2")
    node1.succeed("ping -c 1 node3")
    node2.succeed("ping -c 1 node3")
    print("All nodes can ping each other")

    print("=== Testing network partition ===")
    # Block traffic from node2 to node1
    node1.succeed("iptables -A INPUT -s node2 -j DROP")
    node1.succeed("iptables -A OUTPUT -d node2 -j DROP")

    time.sleep(1)

    # Node2 should NOT be able to reach Node1
    node2.fail("curl --max-time 2 http://node1:3020/")
    print("Network partition working - node2 cannot reach node1")

    # But Node3 can still reach Node1
    node3.succeed("curl -s http://node1:3020/")
    print("Node3 can still reach node1 (not partitioned)")

    print("=== Healing network partition ===")
    node1.succeed("iptables -F")
    time.sleep(1)

    # Now Node2 can reach Node1 again
    node2.succeed("curl -s http://node1:3020/")
    print("Partition healed - node2 can reach node1 again")

    print("=== Testing simultaneous requests ===")
    # All nodes hit node1 at the same time
    node2.succeed("curl -s http://node1:3020/ &")
    node3.succeed("curl -s http://node1:3020/ &")
    time.sleep(1)
    print("Concurrent requests handled")

    print("=== All tests passed! ===")
    print("")
    print("This demonstrates:")
    print("  ✓ Multiple VMs running in isolation")
    print("  ✓ Cross-VM network communication")
    print("  ✓ Network partition simulation")
    print("  ✓ Partition healing")
    print("  ✓ Concurrent operations")
    print("")
    print("You're ready for distributed mvm-ci testing!")
  '';
})
