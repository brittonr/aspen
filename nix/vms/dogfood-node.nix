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

    # Kernel parameters for debugging visibility
    kernelParams = [
      "loglevel=7" # KERN_DEBUG - verbose kernel messages
      "systemd.log_level=info" # systemd boot verbosity
      "systemd.log_target=console" # Send systemd logs to console
    ];

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
        # Use absolute path to prevent socket creation in project root
        # (Nix flakes can't copy Unix socket files)
        socket = "/tmp/aspen-node-${toString nodeId}-virtiofs-nix-store.sock";
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
        # Deterministic MAC address based on node ID (padded to 2 hex digits)
        mac = "02:00:00:01:01:${lib.fixedWidthString 2 "0" (lib.trivial.toHexString nodeId)}";
      }
    ];

    # Socket for Cloud Hypervisor API (pause/resume/snapshot)
    socket = "cloud-hypervisor.sock";

    # Security hardening note:
    # Landlock sandboxing is DISABLED because it cannot restrict virtual
    # filesystems like sysfs. Cloud Hypervisor needs to read TAP device flags
    # from /sys/class/net/{tap}/tun_flags for multi-queue verification.
    # Landlock only works on real filesystems with persistent inodes.
    #
    # Security is still provided by:
    #   - seccomp syscall filtering (enabled by default in cloud-hypervisor)
    #   - VM-level firewall rules (see networking.firewall below)
    #   - systemd service hardening (see aspen-node.nix)
    #   - KVM hardware isolation
    cloud-hypervisor.extraArgs = [
      # Serial console output to file for debugging boot issues
      # File is on host (Cloud Hypervisor runs on host), accessible via tail -f
      "--serial"
      "file=/tmp/aspen-node-${toString nodeId}-serial.log"
    ];
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
    defaultGateway = {
      address = "10.100.0.1";
      interface = "eth0";
    };

    # Use host DNS
    nameservers = ["10.100.0.1"];

    # Firewall enabled with explicit allowlist for isolation
    firewall = {
      enable = true;

      # Aspen node ports
      allowedTCPPorts = [
        7777 # Aspen QUIC (ALPN routing)
        9000 # Metrics/health
      ];

      allowedUDPPorts = [
        7777 # Aspen QUIC
        7778 # Gossip discovery
      ];

      # Allow ICMP for health checks
      allowPing = true;

      # Log dropped packets for debugging (rate limited)
      logRefusedConnections = true;

      # Block inter-VM communication except through allowed ports
      # This prevents lateral movement if a CI job is compromised
      extraCommands = ''
        # Allow established connections
        iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

        # Allow localhost
        iptables -A INPUT -i lo -j ACCEPT

        # Allow from host bridge only (10.100.0.1)
        iptables -A INPUT -s 10.100.0.1 -j ACCEPT

        # Rate limit new connections to prevent DoS
        iptables -A INPUT -p tcp --syn -m limit --limit 100/s --limit-burst 200 -j ACCEPT
        iptables -A INPUT -p udp -m limit --limit 100/s --limit-burst 200 -j ACCEPT
      '';
    };
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
