# MicroVM configuration for Aspen dogfood nodes
#
# This module creates an ephemeral Cloud Hypervisor microVM for running
# isolated Aspen nodes during dogfood testing. The VM:
#
# - Uses virtiofs to share /nix/store from host (fast, no duplication)
# - Has ephemeral storage (fresh state each boot)
# - Runs aspen-node as a systemd service
# - Connects to host via TAP networking on 10.100.0.0/24
#
# Parameters:
#   nodeId       - Unique node identifier (1-10)
#   cookie       - Cluster authentication secret
#   aspenPackage - The aspen-node package to run
{
  lib,
  nodeId,
  cookie,
  aspenPackage,
  ...
}: {
  # MicroVM hypervisor configuration
  microvm = {
    # Use Cloud Hypervisor for fast boot times (~125ms)
    hypervisor = "cloud-hypervisor";

    # Resource allocation
    mem = 4096; # 4GB RAM
    vcpu = 4; # 4 vCPUs

    # Ephemeral root filesystem (no persistent volumes)
    # VM starts fresh each boot - all state is temporary
    volumes = [];

    # Share host's /nix/store via virtiofs for fast access
    # This avoids downloading/copying store paths into each VM
    shares = [
      {
        source = "/nix/store";
        mountPoint = "/nix/.ro-store";
        tag = "nix-store";
        proto = "virtiofs";
      }
    ];

    # Writable overlay for any store paths built inside VM
    # Required for CI jobs that run nix builds
    writableStoreOverlay = "/nix/.rw-store";

    # TAP networking - connects to host bridge aspen-br0
    interfaces = [
      {
        type = "tap";
        # TAP device name on host (aspen-1, aspen-2, etc.)
        id = "aspen-${toString nodeId}";
        # Deterministic MAC address based on node ID
        mac = "02:00:00:01:01:${lib.trivial.toHexString nodeId}";
      }
    ];

    # Socket for Cloud Hypervisor API (pause/resume/snapshot)
    socket = "cloud-hypervisor.sock";
  };

  # Import the aspen-node service module
  imports = [
    ../modules/aspen-node.nix
  ];

  # Configure the aspen-node service
  services.aspen.node = {
    enable = true;
    inherit nodeId cookie;
    package = aspenPackage;

    # Enable CI/CD features for dogfood testing
    enableCi = true;
    ciAutoTrigger = true;
    enableWorkers = true;
    workerCount = 2;

    # Use ephemeral storage inside VM
    dataDir = "/tmp/aspen";
    storageBackend = "redb";

    # Disable relay mode for local testing
    relayMode = "disabled";

    # Detailed logging for debugging
    logLevel = "info";

    # Features for dogfood workflow
    features = [
      "ci"
      "forge"
      "git-bridge"
      "nix-cache-gateway"
      "shell-worker"
      "blob"
    ];
  };

  # VM networking configuration
  networking = {
    hostName = "aspen-node-${toString nodeId}";

    # Static IP on the test network (10.100.0.11, 10.100.0.12, etc.)
    interfaces.eth0 = {
      useDHCP = false;
      ipv4.addresses = [
        {
          address = "10.100.0.${toString (10 + nodeId)}";
          prefixLength = 24;
        }
      ];
    };

    # Gateway is the host bridge
    defaultGateway = "10.100.0.1";

    # Use host DNS
    nameservers = ["10.100.0.1"];

    # Firewall disabled for test environment
    firewall.enable = false;
  };

  # Minimal NixOS configuration for fast boot
  system.stateVersion = "24.11";

  # Enable nix for builds inside VM
  nix = {
    enable = true;
    settings = {
      experimental-features = ["nix-command" "flakes"];
      # Trust the host's store
      trusted-users = ["root"];
    };
  };

  # Essential packages for CI jobs
  environment.systemPackages = with lib; [
    # These will be provided by the calling context
  ];

  # Disable unnecessary services for faster boot
  services.openssh.enable = false;
  documentation.enable = false;
  programs.command-not-found.enable = false;

  # Auto-login to console for debugging
  services.getty.autologinUser = "root";

  # Ensure aspen-node starts on boot
  systemd.services.aspen-node.wantedBy = ["multi-user.target"];
}
