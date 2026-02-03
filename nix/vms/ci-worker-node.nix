# MicroVM configuration for CI worker nodes
#
# This module creates an ephemeral Cloud Hypervisor microVM for running
# isolated CI jobs. Unlike dogfood-node.nix, this is a minimal configuration
# that only runs the guest agent (aspen-ci-agent) for executing jobs.
#
# The VM:
# - Uses virtiofs to share /nix/store from host (read-only)
# - Has a workspace virtiofs mount for job data (read-write, per-job)
# - Uses tmpfs for writable store overlay (virtiofs lacks overlayfs support)
# - Runs aspen-ci-agent on vsock for host communication
# - Has no network interface (all I/O via virtiofs and vsock)
# - Fresh ephemeral state each boot (no persistent storage)
#
# Flake inputs are prefetched on the host via `nix flake archive` and passed
# to the VM using --override-input flags, enabling offline evaluation.
#
# This configuration is used by CloudHypervisorWorker to execute CI jobs
# in isolated microVMs. Jobs are sent via vsock and output is streamed back.
#
# Parameters:
#   vmId               - Unique VM identifier (e.g., "aspen-ci-n1-vm0")
#   aspenCiAgentPackage - The aspen-ci-agent binary package
{
  lib,
  pkgs,
  vmId,
  aspenCiAgentPackage,
  ...
}: {
  # MicroVM hypervisor configuration
  microvm = {
    # Use Cloud Hypervisor for fast boot times (~125ms)
    hypervisor = "cloud-hypervisor";

    # Resource allocation for Rust builds
    # 24GB RAM - tmpfs grows as store paths are fetched/built (~6-10GB),
    # plus nix evaluation needs ~4GB, and build processes need ~4GB for linking
    mem = 24576; # 24GB RAM for CI jobs
    vcpu = 4; # 4 vCPUs

    # Kernel parameters for minimal boot
    kernelParams = [
      "console=ttyS0" # Direct kernel output to serial port
      "loglevel=4" # KERN_WARNING - less verbose than dogfood
      "systemd.log_level=info"
      "net.ifnames=0"
      "panic=1" # Reboot immediately on kernel panic
    ];

    # No disk volumes - all storage via virtiofs shares
    # (CloudHypervisorWorker doesn't support --disk, only --fs)
    volumes = [];

    # VirtioFS shares - only nix-store and workspace.
    # Note: We use tmpfs for /nix/.rw-store instead of virtiofs because
    # virtiofs lacks the filesystem features required by overlayfs for its
    # upper layer (see microvm.nix issue #43). The actual sockets are created
    # by CloudHypervisorWorker at runtime, but tags and mount points must match.
    shares = [
      {
        # Host Nix store shared read-only
        # Socket created by CloudHypervisorWorker before VM boot
        source = "/nix/store";
        mountPoint = "/nix/.ro-store";
        tag = "nix-store";
        proto = "virtiofs";
      }
      {
        # Workspace for job data, read-write
        # Socket created by CloudHypervisorWorker before VM boot
        source = "/tmp/workspace";
        mountPoint = "/workspace";
        tag = "workspace";
        proto = "virtiofs";
      }
    ];

    # Writable overlay for any store paths built inside VM
    # Required for CI jobs that run nix builds
    writableStoreOverlay = "/nix/.rw-store";

    # TAP network interface for downloading from substituters (cache.nixos.org).
    # While isolation is important, we need network access to fetch prebuilt derivations
    # that aren't flake inputs (e.g., bash, stdenv). The VM uses SLIRP/user-mode networking
    # via Cloud Hypervisor's built-in network which doesn't require host bridge setup.
    interfaces = [
      {
        type = "tap";
        id = vmId; # TAP device name
        mac = "02:00:00:c1:00:01"; # Fixed MAC for CI VMs
      }
    ];

    # Vsock configuration for host-guest communication
    # The guest agent listens on port 5000, host connects via Unix socket
    # CID 3 is the guest address (0=hypervisor, 1=loopback, 2=host)
    vsock.cid = 3;

    # Socket for Cloud Hypervisor API
    socket = "api.sock";

    # Serial console output for debugging
    cloud-hypervisor.extraArgs = [
      "--serial"
      "file=/tmp/${vmId}-serial.log"
    ];
  };

  # No imports - this is a minimal configuration

  # Networking for cache.nixos.org access
  # IP is configured via kernel ip= parameter (set by CloudHypervisorWorker)
  # This provides a static IP in the 10.200.0.0/24 range with the host bridge
  # (aspen-ci-br0 at 10.200.0.1) as the gateway providing NAT.
  networking = {
    hostName = vmId;

    # Use systemd-networkd for deterministic network configuration
    useNetworkd = true;

    # Don't use DHCP - IP is set via kernel cmdline
    useDHCP = false;

    # Firewall disabled for full unrestricted internet access.
    # The VM is ephemeral and isolated via virtualization, so the security
    # boundary is at the hypervisor level rather than the guest firewall.
    firewall.enable = false;

    # DNS for package downloads - write directly to /etc/resolv.conf
    nameservers = ["8.8.8.8" "8.8.4.4"];
  };

  # Configure eth0 via systemd-networkd.
  # The kernel ip= parameter configures IP at early boot, but systemd-networkd
  # takes over after switch-root. We configure the same IP/gateway here to
  # maintain the configuration, and add DNS which kernel ip= doesn't support.
  #
  # IP address (10.200.0.10 + vm_index) and gateway (10.200.0.1) are set by
  # CloudHypervisorWorker in the kernel cmdline. We duplicate here for clarity
  # and to ensure systemd-networkd doesn't reset the interface.
  systemd.network = {
    enable = true;
    networks."10-eth0" = {
      matchConfig.Name = "eth0";
      # Static IP configuration for VM network access.
      # The kernel ip= parameter sets IP at early boot, but systemd-networkd
      # needs explicit address config to maintain it after switch-root.
      address = ["10.200.0.10/24"];
      networkConfig = {
        DHCP = "no";
        DNS = ["8.8.8.8" "8.8.4.4"];
      };
      # Default route via host bridge
      routes = [
        {
          Gateway = "10.200.0.1";
          # GatewayOnLink ensures we can reach the gateway even if it's
          # not on the same subnet (though in our case it is)
          GatewayOnLink = true;
        }
      ];
      linkConfig.RequiredForOnline = "routable";
    };
  };

  # Disable systemd-resolved (too heavyweight for CI VM).
  services.resolved.enable = false;

  # Explicitly write /etc/resolv.conf since networking.nameservers + useNetworkd
  # may have race conditions. This ensures DNS is available immediately at boot.
  environment.etc."resolv.conf".text = ''
    nameserver 8.8.8.8
    nameserver 8.8.4.4
  '';

  # Minimal NixOS configuration for fast boot
  system.stateVersion = "24.11";

  # Enable nix for builds inside VM
  nix = {
    enable = true;
    settings = {
      experimental-features = ["nix-command" "flakes"];
      # Trust root for builds
      trusted-users = ["root"];
      # Sandbox builds for isolation within VM
      sandbox = true;
      # Disable global flake registry to prevent network fetch attempts.
      # Without this, Nix tries to fetch https://channels.nixos.org/flake-registry.json
      # even in offline mode (GitHub Issue #8953).
      flake-registry = "";
      # CRITICAL: Disable store optimization when using writable overlay store.
      # Store optimization tries to create hardlinks between store paths with identical
      # contents, but this fails across overlayfs layers. See microvm.nix issue #50.
      # Without this, nix-daemon may fail with "Read-only file system" errors.
      auto-optimise-store = false;
      # CRITICAL: Disable build users group to prevent chown on overlay store.
      # When nix-daemon starts, it tries to chown /nix/store to root:nixbld with mode 01775.
      # This fails on overlayfs because you cannot chown the overlay mount point itself
      # (only files in the upper layer). The host's /nix/store already has correct ownership,
      # but the overlay appears to have different ownership from nix-daemon's perspective.
      # Setting build-users-group to empty disables this check and runs builds as root.
      # This is safe in our isolated CI VM context where only trusted code runs.
      build-users-group = "";

      # Explicit substituters for binary cache access.
      # The VM has full internet access via TAP network with NAT, so it can
      # download pre-built packages instead of compiling from source.
      substituters = [
        "https://cache.nixos.org"
      ];
      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      ];

      # Always try to substitute before building
      builders-use-substitutes = true;
    };
  };

  # Explicitly disable nix store optimization (redundant but explicit)
  # This prevents the scheduled optimization service from running.
  nix.optimise.automatic = false;

  # Mount points for virtiofs shares
  # These extend the mounts created by microvm.nix from the shares config above
  # neededForBoot = true ensures they're mounted in the initrd before switch-root
  fileSystems = {
    # Nix store (read-only from host via virtiofs)
    # Must be mounted before switch-root so the init can run
    "/nix/.ro-store" = {
      fsType = "virtiofs";
      device = "nix-store";
      options = ["ro"];
      neededForBoot = true;
    };

    # Workspace (read-write for job data)
    # Must be mounted before aspen-ci-agent starts
    "/workspace" = {
      fsType = "virtiofs";
      device = "workspace";
      options = ["rw"];
      neededForBoot = true;
    };

    # Writable store overlay using tmpfs.
    # We use tmpfs instead of virtiofs because virtiofs lacks the filesystem
    # features required by overlayfs for its upper layer (xattr, etc.).
    # Size set to 12GB to accommodate store paths fetched from substituters.
    # With 24GB RAM total, this leaves ~12GB for system and build processes.
    "/nix/.rw-store" = {
      fsType = "tmpfs";
      device = "tmpfs";
      options = ["rw" "size=12G" "mode=755"];
      neededForBoot = true;
    };
  };

  # Guest agent service - receives jobs from host via vsock
  systemd.services.aspen-ci-agent = {
    description = "Aspen CI Guest Agent";
    wantedBy = ["multi-user.target"];
    # Depend on local filesystems and nix-daemon being ready
    # nix-daemon is required for nix build commands to work
    after = ["local-fs.target" "nix-daemon.service"];
    requires = ["local-fs.target"];
    wants = ["nix-daemon.service"];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${aspenCiAgentPackage}/bin/aspen-ci-agent --vsock-port 5000";
      Restart = "always";
      RestartSec = "1s";

      # Working directory for job execution
      WorkingDirectory = "/workspace";

      # Security hardening (relaxed to allow nix-daemon communication)
      NoNewPrivileges = true;
      ProtectHome = true;
      PrivateTmp = true;
      # Note: ProtectSystem removed - nix commands need access to daemon socket
      # and various /nix paths for builds
    };

    # Environment for nix builds
    environment = {
      HOME = "/root";
      NIX_PATH = "";
      # Enable flakes and nix-command
      NIX_CONFIG = "experimental-features = nix-command flakes";
    };
  };

  # Essential packages for CI jobs
  environment.systemPackages = [
    # The guest agent itself
    aspenCiAgentPackage
    # Git is required for nix to fetch git-based flake inputs that weren't prefetched
    pkgs.git
    # Note: nix is already available via nix.enable = true
  ];

  # Disable unnecessary services for faster boot
  services.openssh.enable = false;
  documentation.enable = false;
  programs.command-not-found.enable = false;

  # No auto-login needed (no console access in CI)
  services.getty.autologinUser = lib.mkForce null;

  # Ensure minimal boot
  boot.loader.grub.enable = false;
  boot.initrd.systemd.enable = true;

  # Required kernel modules for vsock (host-guest communication)
  # and virtiofs (Nix store sharing)
  boot.initrd.availableKernelModules = [
    "vsock"
    "vmw_vsock_virtio_transport"
    "vhost_vsock"
    "virtiofs"
  ];
  boot.kernelModules = [
    "vsock"
    "vmw_vsock_virtio_transport"
  ];

  # Users for job execution
  users.users.ci = {
    isNormalUser = true;
    home = "/home/ci";
    description = "CI Job Runner";
    # No password - jobs run as root via systemd
  };
}
