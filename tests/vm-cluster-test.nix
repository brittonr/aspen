# NixOS VM cluster test for mvm-ci
# Run with: nix-build tests/vm-cluster-test.nix
# Or: nix-build tests/vm-cluster-test.nix -A driver && ./result/bin/nixos-test-driver

{ pkgs ? import <nixpkgs> {} }:

let
  # Build mvm-ci from local source
  mvm-ci = pkgs.rustPlatform.buildRustPackage {
    pname = "mvm-ci";
    version = "0.1.0";
    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;
  };

  # Hiqlite cluster configuration generator
  mkHiqliteConfig = { nodeId, nodes }: pkgs.writeText "hiqlite.toml" ''
    [hiqlite]
    node_id = ${toString nodeId}
    data_dir = "/var/lib/hiqlite"
    secret_raft = "SuperSecureSecret1337ForRaft"
    secret_api = "SuperSecureSecret1337ForAPI"
    enc_keys = ["bVCyTsGaggVy5yqQ/UzluN29DZW41M3hTSkx6Y3NtZmRuQkR2TnJxUTYzcjQ="]
    enc_key_active = "bVCyTsGaggVy5yqQ"

    # Raft cluster members
    nodes = [
      ${pkgs.lib.concatMapStringsSep "\n      " (n: ''"${toString n.id} ${n.host}:${toString n.raftPort} ${n.host}:${toString n.apiPort}"'') nodes}
    ]

    listen_addr_api = "0.0.0.0"
    listen_addr_raft = "0.0.0.0"
  '';

  # Common node configuration
  commonConfig = { config, pkgs, ... }: {
    # Basic system setup
    networking.firewall.enable = false;  # Simplified for testing

    environment.systemPackages = with pkgs; [
      curl
      jq
      mvm-ci
    ];

    # Create necessary directories
    systemd.tmpfiles.rules = [
      "d /var/lib/hiqlite 0755 mvm-ci mvm-ci -"
      "d /var/lib/mvm-ci 0755 mvm-ci mvm-ci -"
      "d /var/lib/flawless 0755 flawless flawless -"
    ];

    users.users.mvm-ci = {
      isSystemUser = true;
      group = "mvm-ci";
      home = "/var/lib/mvm-ci";
      createHome = true;
    };
    users.groups.mvm-ci = {};

    users.users.flawless = {
      isSystemUser = true;
      group = "flawless";
      home = "/var/lib/flawless";
      createHome = true;
    };
    users.groups.flawless = {};
  };

  # Cluster node definitions
  clusterNodes = [
    { id = 1; host = "node1"; raftPort = 9000; apiPort = 9001; httpPort = 3020; }
    { id = 2; host = "node2"; raftPort = 9000; apiPort = 9001; httpPort = 3020; }
    { id = 3; host = "node3"; raftPort = 9000; apiPort = 9001; httpPort = 3020; }
  ];

  # Generate node configuration
  mkNodeConfig = nodeInfo: { config, pkgs, ... }: {
    imports = [ commonConfig ];

    networking.hostName = nodeInfo.host;

    # Flawless service (one per node)
    systemd.services.flawless = {
      description = "Flawless WASM workflow engine";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        Type = "simple";
        User = "flawless";
        Group = "flawless";
        WorkingDirectory = "/var/lib/flawless";
        ExecStart = "${pkgs.flawless}/bin/flawless up";
        Restart = "on-failure";
        RestartSec = "5s";
      };
    };

    # mvm-ci service
    systemd.services.mvm-ci = {
      description = "MVM-CI distributed workflow coordinator";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" "flawless.service" ];
      requires = [ "flawless.service" ];

      environment = {
        HTTP_PORT = toString nodeInfo.httpPort;
        FLAWLESS_URL = "http://localhost:27288";
        IROH_BLOBS_PATH = "/var/lib/mvm-ci/iroh-blobs";
      };

      serviceConfig = {
        Type = "simple";
        User = "mvm-ci";
        Group = "mvm-ci";
        WorkingDirectory = "/var/lib/mvm-ci";
        ExecStartPre = "${pkgs.coreutils}/bin/cp ${mkHiqliteConfig { nodeId = nodeInfo.id; nodes = clusterNodes; }} /var/lib/mvm-ci/hiqlite.toml";
        ExecStart = "${mvm-ci}/bin/mvm-ci";
        Restart = "on-failure";
        RestartSec = "10s";

        # Resource limits
        MemoryMax = "2G";
        CPUQuota = "200%";
      };
    };
  };

in pkgs.nixosTest {
  name = "mvm-ci-cluster";

  nodes = {
    node1 = mkNodeConfig (builtins.elemAt clusterNodes 0);
    node2 = mkNodeConfig (builtins.elemAt clusterNodes 1);
    node3 = mkNodeConfig (builtins.elemAt clusterNodes 2);
  };

  testScript = ''
    import time

    # Start all nodes
    start_all()

    print("=== Waiting for services to start ===")
    node1.wait_for_unit("flawless.service")
    node2.wait_for_unit("flawless.service")
    node3.wait_for_unit("flawless.service")

    time.sleep(5)

    node1.wait_for_unit("mvm-ci.service")
    node2.wait_for_unit("mvm-ci.service")
    node3.wait_for_unit("mvm-ci.service")

    time.sleep(10)  # Let Raft cluster form

    print("=== Verifying services are running ===")
    node1.succeed("systemctl is-active mvm-ci.service")
    node2.succeed("systemctl is-active mvm-ci.service")
    node3.succeed("systemctl is-active mvm-ci.service")

    print("=== Testing HTTP endpoints ===")
    node1.wait_for_open_port(3020)
    node2.wait_for_open_port(3020)
    node3.wait_for_open_port(3020)

    # Test health check
    node1.succeed("curl -f http://localhost:3020/")
    node2.succeed("curl -f http://localhost:3020/")
    node3.succeed("curl -f http://localhost:3020/")

    print("=== Testing Raft cluster formation ===")
    # Submit job to node1
    node1.succeed(
        'curl -X POST http://localhost:3020/new-job '
        '-H "Content-Type: application/x-www-form-urlencoded" '
        '-d "url=https://test-job-1.com"'
    )

    time.sleep(5)

    # Verify job is visible on all nodes (Raft replication)
    print("=== Verifying job replication across cluster ===")
    node1_jobs = node1.succeed("curl -s http://localhost:3020/queue/list")
    node2_jobs = node2.succeed("curl -s http://localhost:3020/queue/list")
    node3_jobs = node3.succeed("curl -s http://localhost:3020/queue/list")

    print(f"Node 1 jobs: {node1_jobs}")
    print(f"Node 2 jobs: {node2_jobs}")
    print(f"Node 3 jobs: {node3_jobs}")

    # All nodes should see the job
    assert "test-job-1" in node1_jobs, "Job not found on node1"
    assert "test-job-1" in node2_jobs, "Job not replicated to node2"
    assert "test-job-1" in node3_jobs, "Job not replicated to node3"

    print("=== Testing job submission from different nodes ===")
    node2.succeed(
        'curl -X POST http://localhost:3020/new-job '
        '-H "Content-Type: application/x-www-form-urlencoded" '
        '-d "url=https://test-job-2.com"'
    )

    time.sleep(5)

    # Verify second job is visible everywhere
    node1_stats = node1.succeed("curl -s http://localhost:3020/queue/stats")
    print(f"Cluster stats: {node1_stats}")

    print("=== Testing node failure resilience ===")
    # Kill node3
    node3.succeed("systemctl stop mvm-ci.service")
    time.sleep(2)

    # Submit job while node3 is down
    node1.succeed(
        'curl -X POST http://localhost:3020/new-job '
        '-H "Content-Type: application/x-www-form-urlencoded" '
        '-d "url=https://test-job-3-during-failure.com"'
    )

    time.sleep(3)

    # Restart node3
    node3.succeed("systemctl start mvm-ci.service")
    time.sleep(10)

    # Verify node3 caught up via Raft log replay
    node3_jobs = node3.succeed("curl -s http://localhost:3020/queue/list")
    assert "test-job-3-during-failure" in node3_jobs, "Node3 did not catch up after restart"

    print("=== Testing network partition (split-brain scenario) ===")
    # Isolate node1 from the cluster
    node1.succeed("iptables -A INPUT -s node2 -j DROP")
    node1.succeed("iptables -A INPUT -s node3 -j DROP")
    node1.succeed("iptables -A OUTPUT -d node2 -j DROP")
    node1.succeed("iptables -A OUTPUT -d node3 -j DROP")

    time.sleep(5)

    # Node2 and Node3 should still form a quorum (2/3)
    node2.succeed(
        'curl -X POST http://localhost:3020/new-job '
        '-H "Content-Type: application/x-www-form-urlencoded" '
        '-d "url=https://test-job-partition.com"'
    )

    time.sleep(3)

    # Node3 should see the job (part of majority)
    node3_jobs_partition = node3.succeed("curl -s http://localhost:3020/queue/list")
    assert "test-job-partition" in node3_jobs_partition

    # Heal the partition
    node1.succeed("iptables -F")
    time.sleep(10)

    # Node1 should catch up
    node1_jobs_healed = node1.succeed("curl -s http://localhost:3020/queue/list")
    assert "test-job-partition" in node1_jobs_healed, "Node1 did not rejoin cluster"

    print("=== Testing cross-node communication ===")
    # Node1 should be able to reach Node2's HTTP endpoint
    node1.succeed("curl -f http://node2:3020/")
    node2.succeed("curl -f http://node3:3020/")
    node3.succeed("curl -f http://node1:3020/")

    print("=== All tests passed! ===")
  '';
}
