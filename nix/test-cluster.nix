# Aspen VM Test Cluster Configuration
#
# This module provides a NixOS-based test cluster launcher that creates
# three Aspen nodes in Cloud Hypervisor microVMs with proper TAP networking
# for network partition testing.
#
# Usage:
#   # From flake directory
#   nix run .#vm-cluster
#
#   # Or build runners directly
#   nix build .#nixosConfigurations.x86_64-linux-aspen-node-1.config.microvm.declaredRunner
{
  self,
  nixpkgs,
  microvm,
  system ? "x86_64-linux",
}: let
  lib = nixpkgs.lib;
  pkgs = import nixpkgs {inherit system;};

  # Network bridge configuration for VM interconnection
  # All VMs connect to this bridge, enabling network isolation testing
  bridgeName = "aspen-br0";
  bridgeSubnet = "10.100.0";

  # Generate TAP device names for each node
  tapName = nodeId: "aspen-${toString nodeId}";

  # Generate static IP for each node on the bridge network
  nodeIp = nodeId: "${bridgeSubnet}.${toString (10 + nodeId)}";

  # Script to set up network bridge and TAP devices
  setupNetworkScript = pkgs.writeShellScript "setup-aspen-network" ''
    set -e

    BRIDGE="${bridgeName}"

    # Check for root/sudo
    if [ "$(id -u)" -ne 0 ]; then
      echo "This script must be run as root"
      exit 1
    fi

    # Create bridge if it doesn't exist
    if ! ip link show "$BRIDGE" &>/dev/null; then
      echo "Creating bridge $BRIDGE..."
      ip link add name "$BRIDGE" type bridge
      ip addr add ${bridgeSubnet}.1/24 dev "$BRIDGE"
      ip link set "$BRIDGE" up

      # Enable IP forwarding for the bridge
      sysctl -w net.ipv4.ip_forward=1
      # Enable bridge netfilter for iptables rules
      modprobe br_netfilter || true
    else
      echo "Bridge $BRIDGE already exists"
    fi

    # Create TAP devices for each node
    for i in 1 2 3; do
      TAP="${tapName "\${i}"}"
      if ! ip link show "$TAP" &>/dev/null; then
        echo "Creating TAP device $TAP..."
        ip tuntap add name "$TAP" mode tap
        ip link set "$TAP" master "$BRIDGE"
        ip link set "$TAP" up
      else
        echo "TAP device $TAP already exists"
      fi
    done

    # Set up dnsmasq for DHCP on the bridge (optional, for easier testing)
    # Nodes can also use static IPs
    echo "Network setup complete!"
    echo "Bridge: $BRIDGE (${bridgeSubnet}.1/24)"
    echo "TAP devices: ${tapName 1}, ${tapName 2}, ${tapName 3}"
  '';

  # Script to tear down network resources
  teardownNetworkScript = pkgs.writeShellScript "teardown-aspen-network" ''
    set -e

    BRIDGE="${bridgeName}"

    if [ "$(id -u)" -ne 0 ]; then
      echo "This script must be run as root"
      exit 1
    fi

    # Remove TAP devices
    for i in 1 2 3; do
      TAP="${tapName "\${i}"}"
      if ip link show "$TAP" &>/dev/null; then
        echo "Removing TAP device $TAP..."
        ip link set "$TAP" down
        ip link delete "$TAP"
      fi
    done

    # Remove bridge
    if ip link show "$BRIDGE" &>/dev/null; then
      echo "Removing bridge $BRIDGE..."
      ip link set "$BRIDGE" down
      ip link delete "$BRIDGE"
    fi

    echo "Network teardown complete!"
  '';

  # Script to launch all VMs
  launchClusterScript = pkgs.writeShellScript "launch-aspen-cluster" ''
    set -e

    echo "Launching Aspen test cluster..."
    echo "================================"

    # Get the directory where VM images will be stored
    CLUSTER_DIR="''${ASPEN_CLUSTER_DIR:-./aspen-cluster-vms}"
    mkdir -p "$CLUSTER_DIR"
    cd "$CLUSTER_DIR"

    # Check if running as root (needed for TAP devices)
    if [ "$(id -u)" -ne 0 ]; then
      echo "Warning: Not running as root. TAP networking may not work."
      echo "Consider running with sudo for network isolation testing."
    fi

    # Set up network first
    echo ""
    echo "Setting up network..."
    sudo ${setupNetworkScript}

    # Launch each node in background
    echo ""
    echo "Launching nodes..."

    NODE1_DIR="$CLUSTER_DIR/node1"
    NODE2_DIR="$CLUSTER_DIR/node2"
    NODE3_DIR="$CLUSTER_DIR/node3"

    mkdir -p "$NODE1_DIR" "$NODE2_DIR" "$NODE3_DIR"

    # Launch node 1
    echo "Starting node 1..."
    (cd "$NODE1_DIR" && ${self.nixosConfigurations."${system}-aspen-node-1".config.microvm.declaredRunner}/bin/microvm-run) &
    NODE1_PID=$!

    # Launch node 2
    echo "Starting node 2..."
    (cd "$NODE2_DIR" && ${self.nixosConfigurations."${system}-aspen-node-2".config.microvm.declaredRunner}/bin/microvm-run) &
    NODE2_PID=$!

    # Launch node 3
    echo "Starting node 3..."
    (cd "$NODE3_DIR" && ${self.nixosConfigurations."${system}-aspen-node-3".config.microvm.declaredRunner}/bin/microvm-run) &
    NODE3_PID=$!

    echo ""
    echo "Cluster launched!"
    echo "  Node 1 PID: $NODE1_PID"
    echo "  Node 2 PID: $NODE2_PID"
    echo "  Node 3 PID: $NODE3_PID"
    echo ""
    echo "Node HTTP endpoints:"
    echo "  Node 1: http://${nodeIp 1}:8301"
    echo "  Node 2: http://${nodeIp 2}:8302"
    echo "  Node 3: http://${nodeIp 3}:8303"
    echo ""
    echo "To initialize the cluster, run from one node:"
    echo '  curl -X POST http://${nodeIp 1}:8301/init -d @- <<EOF'
    echo '  {"initial_members": ['
    echo '    {"id": 1, "addr": "${nodeIp 1}:8301"},'
    echo '    {"id": 2, "addr": "${nodeIp 2}:8302"},'
    echo '    {"id": 3, "addr": "${nodeIp 3}:8303"}'
    echo '  ]}'
    echo '  EOF'
    echo ""
    echo "Press Ctrl+C to stop the cluster..."

    # Wait for any of the nodes to exit
    wait $NODE1_PID $NODE2_PID $NODE3_PID

    echo "Cluster stopped."
  '';

  # Script to inject network partition
  injectPartitionScript = pkgs.writeShellScript "inject-partition" ''
    set -e

    if [ $# -lt 2 ]; then
      echo "Usage: $0 <node_id> <target_node_id>"
      echo "  Isolates node_id from target_node_id using iptables"
      echo ""
      echo "Example: $0 1 2  # Isolate node 1 from node 2"
      exit 1
    fi

    SOURCE_NODE=$1
    TARGET_NODE=$2
    SOURCE_IP="${bridgeSubnet}.$((10 + SOURCE_NODE))"
    TARGET_IP="${bridgeSubnet}.$((10 + TARGET_NODE))"

    echo "Injecting network partition: node $SOURCE_NODE <-> node $TARGET_NODE"
    echo "  Blocking traffic from $SOURCE_IP to $TARGET_IP"

    # Block traffic in both directions on the bridge
    sudo iptables -I FORWARD -s "$SOURCE_IP" -d "$TARGET_IP" -j DROP
    sudo iptables -I FORWARD -s "$TARGET_IP" -d "$SOURCE_IP" -j DROP

    echo "Partition injected successfully!"
    echo "To heal: sudo iptables -D FORWARD -s $SOURCE_IP -d $TARGET_IP -j DROP && sudo iptables -D FORWARD -s $TARGET_IP -d $SOURCE_IP -j DROP"
  '';

  # Script to heal network partition
  healPartitionScript = pkgs.writeShellScript "heal-partition" ''
    set -e

    if [ $# -lt 2 ]; then
      echo "Usage: $0 <node_id> <target_node_id>"
      echo "  Removes partition between node_id and target_node_id"
      exit 1
    fi

    SOURCE_NODE=$1
    TARGET_NODE=$2
    SOURCE_IP="${bridgeSubnet}.$((10 + SOURCE_NODE))"
    TARGET_IP="${bridgeSubnet}.$((10 + TARGET_NODE))"

    echo "Healing network partition: node $SOURCE_NODE <-> node $TARGET_NODE"

    # Remove the DROP rules
    sudo iptables -D FORWARD -s "$SOURCE_IP" -d "$TARGET_IP" -j DROP 2>/dev/null || true
    sudo iptables -D FORWARD -s "$TARGET_IP" -d "$SOURCE_IP" -j DROP 2>/dev/null || true

    echo "Partition healed!"
  '';
in {
  # Export the scripts as packages
  packages = {
    vm-cluster-setup-network = setupNetworkScript;
    vm-cluster-teardown-network = teardownNetworkScript;
    vm-cluster-launch = launchClusterScript;
    vm-cluster-inject-partition = injectPartitionScript;
    vm-cluster-heal-partition = healPartitionScript;

    # Combined package with all scripts
    vm-cluster = pkgs.buildEnv {
      name = "aspen-vm-cluster";
      paths = [
        (pkgs.runCommand "aspen-cluster-scripts" {} ''
          mkdir -p $out/bin
          cp ${setupNetworkScript} $out/bin/aspen-setup-network
          cp ${teardownNetworkScript} $out/bin/aspen-teardown-network
          cp ${launchClusterScript} $out/bin/aspen-launch-cluster
          cp ${injectPartitionScript} $out/bin/aspen-inject-partition
          cp ${healPartitionScript} $out/bin/aspen-heal-partition
        '')
      ];
    };
  };

  # Network configuration constants for use in tests
  config = {
    bridge = bridgeName;
    subnet = bridgeSubnet;
    nodes = {
      node1 = {
        id = 1;
        ip = nodeIp 1;
        httpPort = 8301;
        ractorPort = 26001;
        tap = tapName 1;
      };
      node2 = {
        id = 2;
        ip = nodeIp 2;
        httpPort = 8302;
        ractorPort = 26002;
        tap = tapName 2;
      };
      node3 = {
        id = 3;
        ip = nodeIp 3;
        httpPort = 8303;
        ractorPort = 26003;
        tap = tapName 3;
      };
    };
  };

  # App definitions for running from flake
  apps = {
    launch-cluster = {
      type = "app";
      program = "${launchClusterScript}";
    };
    setup-network = {
      type = "app";
      program = "${setupNetworkScript}";
    };
    teardown-network = {
      type = "app";
      program = "${teardownNetworkScript}";
    };
    inject-partition = {
      type = "app";
      program = "${injectPartitionScript}";
    };
    heal-partition = {
      type = "app";
      program = "${healPartitionScript}";
    };
  };
}
