# MicroVM configuration for CI worker nodes
#
# This module creates an ephemeral Cloud Hypervisor microVM for running
# isolated CI jobs. Unlike dogfood-node.nix, this is a minimal configuration
# that runs aspen-node in worker-only mode to execute jobs.
#
# The VM:
# - Uses virtiofs to share /nix/store from host (read-only)
# - Has a workspace virtiofs mount for job data (read-write, per-job)
# - Uses tmpfs for writable store overlay (virtiofs lacks overlayfs support)
# - Runs aspen-node in worker-only mode (ephemeral CI worker)
# - Connects to the cluster via Iroh over TAP network
# - Uploads build artifacts directly to SNIX binary cache
# - Fresh ephemeral state each boot (no persistent storage)
#
# Flake inputs are prefetched on the host via `nix flake archive` and passed
# to the VM using --override-input flags, enabling offline evaluation.
#
# This configuration is used by CloudHypervisorWorker to execute CI jobs
# in isolated microVMs. The VM joins the cluster and receives jobs via
# the normal job queue, then uploads results to SNIX.
#
# Parameters:
#   vmId             - Unique VM identifier (e.g., "aspen-ci-n1-vm0")
#   aspenNodePackage - The aspen-node binary package
{
  lib,
  pkgs,
  vmId,
  aspenNodePackage,
  # Optional: closureInfo for CI build deps. When provided, these store paths
  # are registered in the VM's nix database at boot so the VM doesn't need
  # to rebuild FODs (rust toolchain, crate tarballs, etc.) that already exist
  # on the host's /nix/store (shared via virtiofs).
  ciBuildClosureInfo ? null,
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

    # DNS via local dnsmasq cache (see services.dnsmasq below).
    # Upstream servers (8.8.8.8, 8.8.4.4, 1.1.1.1) configured in dnsmasq.
    nameservers = ["127.0.0.1"];
  };

  # Configure eth0 via systemd-networkd without hardcoding the per-VM address.
  # CloudHypervisorWorker passes the actual address in the kernel `ip=`
  # parameter (10.200.0.10 + vm_index). If this NixOS module also hardcodes an
  # Address, every restored worker converges back to 10.200.0.10 after
  # switch-root, causing duplicate guest addresses and direct-Iroh timeouts.
  # Preserve the kernel/static configuration and only add DNS/route policy here;
  # the post-boot direct path below also re-applies the parsed `ip=` value before
  # starting the worker.
  systemd.network = {
    enable = true;
    networks."10-eth0" = {
      matchConfig.Name = "eth0";
      networkConfig = {
        DHCP = "no";
        KeepConfiguration = "static";
        # DNS handled by local dnsmasq cache (127.0.0.1)
        DNS = ["127.0.0.1"];
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

  # Local DNS cache via dnsmasq.
  # Under heavy parallel load (dozens of FOD builds curling crate sources),
  # outbound DNS queries through NAT can overwhelm conntrack and responses
  # get dropped. A local cache ensures:
  # 1. Repeated lookups hit cache instantly (static.crates.io, cache.nixos.org)
  # 2. Fewer outbound DNS queries through NAT
  # 3. Retries are handled locally with configurable timeouts
  services.dnsmasq = {
    enable = true;
    settings = {
      # Don't read /etc/resolv.conf (we manage upstream servers explicitly)
      no-resolv = true;
      # Upstream DNS servers (Google + Cloudflare for redundancy)
      server = ["8.8.8.8" "8.8.4.4" "1.1.1.1"];
      # Listen only on localhost
      listen-address = "127.0.0.1";
      bind-interfaces = true;
      # Cache size — 1000 entries covers all crate/nix package names
      cache-size = 1000;
      # Don't forward plain names (no domain suffix)
      domain-needed = true;
      # Don't forward reverse lookups for private ranges
      bogus-priv = true;
      # Reduce logging noise
      log-facility = "/dev/null";
    };
  };

  # Ensure dnsmasq starts before the CI worker
  systemd.services.dnsmasq = {
    before = ["aspen-ci-worker.service"];
    wantedBy = ["multi-user.target"];
  };

  # Point resolv.conf at the local dnsmasq cache.
  # Also configure aggressive retry/timeout for resilience under load.
  environment.etc."resolv.conf".text = ''
    nameserver 127.0.0.1
    options timeout:2 attempts:5 rotate
  '';

  # Minimal NixOS configuration for fast boot
  system.stateVersion = "24.11";

  # The NixOS test builder can run under transient high-numbered UIDs that are
  # absent from passwd. logrotate's build-time config check shells out to `id`
  # and fails before the guest image is built; skip that host-side check for
  # this ephemeral CI microVM image.
  services.logrotate.checkConfig = false;

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

      # Disable auto-GC in the VM. The tmpfs-backed overlay (12GB) fills up during
      # large builds, triggering Nix's auto-GC which scans /nix/store/.links causing
      # "Too many open files in system" errors. Instead of auto-GC:
      # - Host pre-builds cargoArtifacts so VM doesn't need to fetch as much
      # - Workspace is cleaned between jobs
      # - VMs are ephemeral (tmpfs is discarded on shutdown)
      # Note: Setting min-free to 0 disables the auto-GC feature entirely.
      min-free = 0;
      max-free = 0;
    };
  };

  # Explicitly disable nix store optimization (redundant but explicit)
  # This prevents the scheduled optimization service from running.
  nix.optimise.automatic = false;

  # Increase file descriptor limit for nix-daemon to handle large builds and GC.
  # During auto-GC, nix-daemon scans /nix/store/.links which may have 600k+ entries.
  # Set to 2M to match fs.nr_open (the kernel per-process maximum).
  # Use mkForce to override the default value from the nix-daemon module.
  systemd.services.nix-daemon.serviceConfig.LimitNOFILE = lib.mkForce 2097152;

  # Mask the host's /nix/store/.links in the overlay upper layer.
  # The host store has millions of hard links that are visible through the
  # overlayfs lower layer. If nix-daemon ever scans .links (GC, optimize),
  # it exhausts the system-wide FD limit traversing them through virtiofs.
  # Creating an empty .links dir in the upper layer (tmpfs) makes overlayfs
  # return an empty listing, preventing the FD exhaustion.
  systemd.services.nix-daemon.serviceConfig.ExecStartPre = lib.mkBefore [
    "+${pkgs.coreutils}/bin/mkdir -p /nix/.rw-store/store/.links"
  ];

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

  # Deterministic pre-worker readiness point for host-side golden snapshots.
  # Cloud Hypervisor reports `Running` before NixOS/systemd has finished booting;
  # the host waits for this serial marker before freezing the golden image.
  systemd.services.aspen-ci-snapshot-ready = {
    description = "Emit Aspen CI VM snapshot readiness marker";
    after = ["local-fs.target" "nix-daemon.service" "network-online.target"];
    requires = ["local-fs.target"];
    wants = ["nix-daemon.service" "network-online.target"];
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      StandardOutput = "tty";
      StandardError = "tty";
      TTYPath = "/dev/ttyS0";
    };
    script = ''
      guest_hostname=$(cat /proc/sys/kernel/hostname 2>/dev/null || true)
      echo "ASPEN_CI_SNAPSHOT_READY host=$guest_hostname ticket=$([ -f /workspace/.aspen-cluster-ticket ] && echo present || echo missing)"
    '';
  };

  # Aspen CI worker service - runs as ephemeral worker node
  # Connects to cluster via Iroh and processes CI jobs
  systemd.services.aspen-ci-worker = {
    description = "Aspen CI Worker (Ephemeral Node)";
    wantedBy = ["multi-user.target"];
    # Depend on local filesystems, network, nix-daemon, and the host-visible
    # snapshot readiness marker before starting the worker.
    after = ["local-fs.target" "nix-daemon.service" "network-online.target" "aspen-ci-snapshot-ready.service"];
    requires = ["local-fs.target" "aspen-ci-snapshot-ready.service"];
    wants = ["nix-daemon.service" "network-online.target"];

    serviceConfig = {
      Type = "simple";
      # Run aspen-node in worker-only mode
      # The cluster ticket is read from /workspace/.aspen-cluster-ticket
      # which is written by CloudHypervisorWorker before VM boot
      ExecStart = "${aspenNodePackage}/bin/aspen-node --worker-only --disable-mdns --relay-mode disabled";
      Restart = "always";
      RestartSec = "1s";

      # Working directory for job execution
      WorkingDirectory = "/workspace";

      # High file descriptor limit for nix builds (matches fs.nr_open)
      LimitNOFILE = 2097152;

      # Security hardening (relaxed to allow nix-daemon communication)
      NoNewPrivileges = true;
      ProtectHome = true;
      PrivateTmp = true;
      # Note: ProtectSystem removed - nix commands need access to daemon socket
      # and various /nix paths for builds
    };

    # Environment for worker-only mode
    environment = {
      HOME = "/root";
      NIX_PATH = "";
      # Enable flakes and nix-command
      NIX_CONFIG = "experimental-features = nix-command flakes";
      # Worker-only mode configuration
      ASPEN_MODE = "ci_worker";
      # Cluster ticket file written by CloudHypervisorWorker
      ASPEN_CLUSTER_TICKET_FILE = "/workspace/.aspen-cluster-ticket";
      # Job types this worker handles
      ASPEN_WORKER_JOB_TYPES = "ci_vm";
      # Workspace directory for job execution - use tmpfs instead of virtiofs
      # because nix's file import mechanism fails on virtiofs mounts (files
      # disappear or return I/O errors). Tmpfs is backed by RAM which is fine
      # for CI workspaces. The /workspace virtiofs mount is still used for the
      # cluster ticket file (ASPEN_CLUSTER_TICKET_FILE).
      ASPEN_CI_WORKSPACE_DIR = "/tmp/workspaces";
      # Debug logging for troubleshooting VM worker connections
      RUST_LOG = "info,aspen=debug,iroh=debug,iroh_net=debug";
      # mDNS cannot bind reliably in the nested VM TAP namespace; the cluster
      # ticket carries the bootstrap peer address for this E2E path. Relays are
      # also disabled so VM-CI dogfood proves only scoped TAP/direct connectivity.
      ASPEN_IROH_ENABLE_MDNS = "false";
      ASPEN_IROH_RELAY_MODE = "disabled";
      ASPEN_RELAY_DISABLED = "1";
    };

    # Send output to both journal and console for visibility in serial log
    serviceConfig.StandardOutput = "journal+console";
    serviceConfig.StandardError = "journal+console";
  };

  # Essential packages for CI jobs
  environment.systemPackages = [
    # The aspen-node binary (runs as worker)
    aspenNodePackage
    # Git is required for nix to fetch git-based flake inputs that weren't prefetched
    pkgs.git
    # Note: nix is already available via nix.enable = true
    # Debugging tools for troubleshooting
    pkgs.iproute2
    pkgs.netcat
    pkgs.curl
    pkgs.tcpdump
  ];

  # Network diagnostic service - runs independently (no blocking dependencies)
  # This just logs diagnostic info but doesn't block aspen-ci-worker startup
  systemd.services.network-diagnostic = {
    description = "Network Diagnostic for CI Worker";
    wantedBy = ["multi-user.target"];
    # Don't use 'before' - let aspen-ci-worker start independently
    after = ["network-online.target"];
    wants = ["network-online.target"];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      StandardOutput = "journal+console";
      StandardError = "journal+console";
      # Timeout after 10 seconds to avoid blocking boot
      TimeoutStartSec = "10s";
    };
    script = ''
      echo "=== Network Diagnostic ==="
      echo "IP addresses:"
      ${pkgs.iproute2}/bin/ip addr show eth0 2>/dev/null || echo "eth0 not found"
      echo ""
      echo "Default route:"
      ${pkgs.iproute2}/bin/ip route show default 2>/dev/null || echo "No default route"
      echo ""
      echo "Cluster ticket:"
      if [ -f /workspace/.aspen-cluster-ticket ]; then
        echo "Found (first 80 chars): $(head -c 80 /workspace/.aspen-cluster-ticket)"
      else
        echo "NOT FOUND"
      fi
      echo "=== End Network Diagnostic ==="
    '';
  };

  # Increase file descriptor limits for nix builds.
  # The nix build process opens many files when traversing /nix/store,
  # and the default limits (1024) are too low for large builds.
  # Set to 2M to match fs.nr_open (the kernel per-process maximum).
  security.pam.loginLimits = [
    {
      domain = "*";
      type = "soft";
      item = "nofile";
      value = "2097152";
    }
    {
      domain = "*";
      type = "hard";
      item = "nofile";
      value = "2097152";
    }
  ];

  # System-wide file descriptor limits via sysctl.
  # CRITICAL: fs.file-max is the TOTAL FDs across ALL processes, not per-process.
  # During auto-GC, nix-daemon scans /nix/store/.links which can have 600k+ entries,
  # easily consuming 1M+ FDs in a single process. With virtiofsd, systemd, and other
  # processes also running, the system-wide total can exceed 2M during GC storms.
  # We set file-max to 8M to provide headroom for:
  #   - nix-daemon GC scanning: up to 2M FDs
  #   - nix-daemon build operations: up to 1M FDs
  #   - virtiofsd (nix-store share): ~100k FDs
  #   - virtiofsd (workspace share): ~10k FDs
  #   - systemd and other services: ~50k FDs
  #   - kernel overhead: ~100k FDs
  boot.kernel.sysctl = {
    "fs.file-max" = 8388608; # 8M total FDs system-wide (was 2M - insufficient for GC)
    "fs.nr_open" = 2097152; # 2M per-process (doubled to allow GC + builds concurrently)
  };

  # Enable SSH for debugging (can connect from host via bridge network)
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PermitEmptyPasswords = "yes";
    };
  };

  # Allow root login without password for debugging
  users.users.root.password = "";

  # Disable unnecessary services for faster boot
  documentation.enable = false;
  programs.command-not-found.enable = false;

  # No auto-login needed (no console access in CI)
  services.getty.autologinUser = lib.mkForce null;

  # Ensure minimal boot
  boot.loader.grub.enable = false;
  # Use the standard NixOS stage-1 script here. The microvm.nix runner
  # only passes `init=${toplevel}/init`; forcing the systemd initrd in
  # this direct Cloud Hypervisor path reaches switch-root but then leaves
  # stage-2 unable to isolate its default target from the tmpfs root.
  boot.initrd.systemd.enable = lib.mkForce false;

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

  # Deterministic guest readiness point for host-side golden snapshots.
  # This direct Cloud Hypervisor path reaches NixOS stage-2 before systemd can
  # reliably isolate the generated unit graph from the tmpfs root. Start the
  # worker from the stage-2 post-boot hook, then emit the host-visible marker so
  # the snapshot captures a guest-backed worker process rather than a kernel-only
  # boot. Systemd units are still declared above for normal/microvm-runner boots.
  # Also register CI build closure in the VM's nix database at boot.
  # The host's /nix/store is shared via virtiofs (read-only), but the VM's
  # nix database is empty. Without registration, nix-daemon doesn't know
  # these paths exist and tries to rebuild FODs (rust toolchain, crate
  # tarballs), which fails when VM DNS is unreliable.
  # closureInfo produces a /registration file with nix-store --dump-db format.
  boot.postBootCommands =
    ''
      mkdir -p /tmp/workspaces

      # The direct Cloud Hypervisor path currently starts the worker from this
      # post-boot hook because systemd target isolation is unreliable under the
      # tmpfs-root snapshot boot. Configure the guest TAP network here as well;
      # otherwise the worker can start before systemd-networkd runs and Iroh dials
      # fail with ENETUNREACH even though the host bridge/TAP exists.
      vm_ip=10.200.0.10
      for arg in $(cat /proc/cmdline); do
        case "$arg" in
          ip=*)
            ip_cfg=''${arg#ip=}
            vm_ip=''${ip_cfg%%:*}
            ;;
        esac
      done

      # The direct boot path can reach postBootCommands before udev has named the
      # virtio-net device. Load the driver and wait for the first non-loopback
      # interface instead of assuming eth0 exists immediately.
      ${pkgs.kmod}/bin/modprobe virtio_net >/dev/null 2>&1 || true
      net_dev=
      for _ in $(seq 1 100); do
        for dev_path in /sys/class/net/*; do
          dev=''${dev_path##*/}
          if [ "$dev" != "lo" ] && [ -e "$dev_path" ]; then
            net_dev=$dev
            break
          fi
        done
        if [ -n "$net_dev" ]; then
          break
        fi
        sleep 0.1
      done

      if [ -n "$net_dev" ]; then
        ${pkgs.iproute2}/bin/ip link set dev "$net_dev" up || true
        ${pkgs.iproute2}/bin/ip addr replace "$vm_ip/24" dev "$net_dev" || true
        ${pkgs.iproute2}/bin/ip route replace default via 10.200.0.1 dev "$net_dev" || true
      fi
      echo "ASPEN_CI_NET_CONFIG ip=$vm_ip dev=''${net_dev:-missing}" >/dev/ttyS0
      ${pkgs.iproute2}/bin/ip addr show >/dev/ttyS0 2>&1 || true
      ${pkgs.iproute2}/bin/ip route show >/dev/ttyS0 2>&1 || true
    ''
    + lib.optionalString (ciBuildClosureInfo != null) ''
      if [ -f ${ciBuildClosureInfo}/registration ]; then
        ${pkgs.nix}/bin/nix-store --load-db < ${ciBuildClosureInfo}/registration
      fi
    ''
    + ''
      (
        export HOME=/root
        export NIX_PATH=
        export NIX_CONFIG='experimental-features = nix-command flakes'
        export ASPEN_MODE=ci_worker
        export ASPEN_CLUSTER_TICKET_FILE=/workspace/.aspen-cluster-ticket
        export ASPEN_WORKER_JOB_TYPES=ci_vm
        export ASPEN_CI_WORKSPACE_DIR=/tmp/workspaces
        export RUST_LOG=info,aspen=debug,iroh=debug,iroh_net=debug
        export ASPEN_IROH_ENABLE_MDNS=false
        export ASPEN_IROH_RELAY_MODE=disabled
        export ASPEN_RELAY_DISABLED=1
        cd /workspace
        exec ${aspenNodePackage}/bin/aspen-node --worker-only --disable-mdns --relay-mode disabled
      ) >/dev/ttyS0 2>&1 &
      guest_hostname=$(cat /proc/sys/kernel/hostname 2>/dev/null || true)
      echo "ASPEN_CI_SNAPSHOT_READY host=$guest_hostname ticket=$([ -f /workspace/.aspen-cluster-ticket ] && echo present || echo missing)" >/dev/ttyS0
    '';
}
