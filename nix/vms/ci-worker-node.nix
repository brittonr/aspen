# MicroVM configuration for CI worker nodes
#
# This module creates an ephemeral Cloud Hypervisor microVM for running
# isolated CI jobs. Unlike dogfood-node.nix, this is a minimal configuration
# that only runs the guest agent (aspen-ci-agent) for executing jobs.
#
# The VM:
# - Uses virtiofs to share /nix/store from host (read-only)
# - Has a workspace virtiofs mount for job data (read-write, per-job)
# - Runs aspen-ci-agent on vsock for host communication
# - Has no network interface (all I/O via virtiofs and vsock)
# - Fresh ephemeral state each boot (no persistent storage)
#
# Flake inputs are resolved by rewriting flake.lock to use path: URLs pointing
# to store paths that were prefetched by `nix flake archive` on the host.
#
# This configuration is used by CloudHypervisorWorker to execute CI jobs
# in isolated microVMs. Jobs are sent via vsock and output is streamed back.
#
# Parameters:
#   vmId               - Unique VM identifier (e.g., "aspen-ci-n1-vm0")
#   aspenCiAgentPackage - The aspen-ci-agent binary package
{
  lib,
  vmId,
  aspenCiAgentPackage,
  ...
}: {
  # MicroVM hypervisor configuration
  microvm = {
    # Use Cloud Hypervisor for fast boot times (~125ms)
    hypervisor = "cloud-hypervisor";

    # Resource allocation for Rust builds
    # 8GB RAM sufficient now that writable store overlay is disk-backed
    mem = 8192; # 8GB RAM for CI jobs
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

    # VirtioFS shares - these MUST be defined here so microvm.nix includes
    # proper mount units in the initrd. The actual sockets are created by
    # CloudHypervisorWorker at runtime, but the tags and mount points must match.
    # Without this, the initrd doesn't know how to mount virtiofs shares.
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
      {
        # Writable store overlay - for nix build artifacts inside VM
        # Uses host disk storage to avoid tmpfs memory limits
        # Rust builds can generate 10GB+ of artifacts
        source = "/tmp/rw-store";
        mountPoint = "/nix/.rw-store";
        tag = "rw-store";
        proto = "virtiofs";
      }
    ];

    # Writable overlay for any store paths built inside VM
    # Required for CI jobs that run nix builds
    writableStoreOverlay = "/nix/.rw-store";

    # No network interfaces - all I/O via virtiofs and vsock
    # This provides stronger isolation and faster boot
    interfaces = [];

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

  # Minimal networking (localhost only, no external network)
  networking = {
    hostName = vmId;

    # No network interfaces
    useDHCP = false;

    # Firewall disabled (no network)
    firewall.enable = false;
  };

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

    # Writable store overlay (disk-backed for build artifacts)
    # This provides persistent storage for nix build outputs, avoiding tmpfs memory limits.
    # Without this, large Rust builds fail with "running auto-GC to free X bytes".
    "/nix/.rw-store" = {
      fsType = "virtiofs";
      device = "rw-store";
      options = ["rw"];
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
  environment.systemPackages = with lib; [
    # The guest agent itself
    aspenCiAgentPackage
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
