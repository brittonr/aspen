# NixOS module for Aspen distributed node service
#
# This module provides a systemd service for running aspen-node with
# configurable options for cluster membership, CI, and worker settings.
#
# Usage in a NixOS configuration:
#   services.aspen.node = {
#     enable = true;
#     nodeId = 1;
#     cookie = "my-cluster-secret";
#     package = pkgs.aspen-node;
#   };
{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.aspen.node;

  # Generate deterministic 32-byte secret key from node ID for reproducible testing
  # Creates a 64-character hex string by repeating the node ID pattern
  # In production, this should be provided externally via secretKey option
  nodeIdHex = lib.trivial.toHexString cfg.nodeId;
  paddedNodeId = lib.strings.fixedWidthString 8 "0" nodeIdHex;
  # Repeat the 8-char pattern 8 times to get 64 hex chars (32 bytes)
  defaultSecretKey = lib.strings.concatStrings (lib.lists.replicate 8 paddedNodeId);
in {
  options.services.aspen.node = {
    enable = lib.mkEnableOption "Aspen distributed node";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The aspen-node package to use";
    };

    nodeId = lib.mkOption {
      type = lib.types.int;
      description = "Unique node identifier for Raft consensus (1-10)";
      example = 1;
    };

    cookie = lib.mkOption {
      type = lib.types.str;
      description = "Shared secret for cluster authentication";
      example = "my-cluster-secret";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/aspen";
      description = "Directory for persistent data (redb storage)";
    };

    storageBackend = lib.mkOption {
      type = lib.types.enum ["redb" "sqlite" "inmemory"];
      default = "redb";
      description = "Storage backend for Raft log and state machine";
    };

    secretKey = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Iroh secret key (hex-encoded). If null, generates deterministic key from nodeId";
    };

    relayMode = lib.mkOption {
      type = lib.types.enum ["disabled" "default" "custom"];
      default = "disabled";
      description = "Iroh relay mode for NAT traversal";
    };

    bindPort = lib.mkOption {
      type = lib.types.int;
      default = 7777;
      description = "Port to bind for QUIC connections (0 = random)";
    };

    enableWorkers = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable job workers for CI/CD execution";
    };

    workerCount = lib.mkOption {
      type = lib.types.int;
      default = 2;
      description = "Number of concurrent workers per node";
    };

    enableCi = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable CI/CD pipeline orchestration";
    };

    ciAutoTrigger = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Automatically trigger CI on repository updates";
    };

    watchedRepos = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "List of repository IDs to watch for CI triggers";
    };

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = "Rust log level (trace, debug, info, warn, error)";
    };

    extraArgs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Additional command-line arguments for aspen-node";
    };

    # Features to enable (maps to Cargo features)
    features = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = ["ci" "forge" "git-bridge" "nix-cache-gateway" "shell-worker" "blob"];
      description = "Aspen features to enable";
    };

    # CI VM isolation settings (Cloud Hypervisor nested VMs for build isolation)
    ciVmKernelPath = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to CI VM kernel (enables Cloud Hypervisor worker)";
    };

    ciVmInitrdPath = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to CI VM initrd";
    };

    ciVmToplevelPath = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to CI VM NixOS toplevel (contains init script)";
    };

    cloudHypervisorPath = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to cloud-hypervisor binary";
    };

    virtiofsdPath = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to virtiofsd binary";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.aspen-node = {
      description = "Aspen distributed node";
      wantedBy = ["multi-user.target"];
      after = ["network-online.target"];
      wants = ["network-online.target"];

      environment =
        {
          RUST_LOG = cfg.logLevel;
          ASPEN_CI_WATCHED_REPOS = lib.concatStringsSep "," cfg.watchedRepos;
        }
        // lib.optionalAttrs (cfg.ciVmKernelPath != null) {
          ASPEN_CI_KERNEL_PATH = cfg.ciVmKernelPath;
        }
        // lib.optionalAttrs (cfg.ciVmInitrdPath != null) {
          ASPEN_CI_INITRD_PATH = cfg.ciVmInitrdPath;
        }
        // lib.optionalAttrs (cfg.ciVmToplevelPath != null) {
          ASPEN_CI_TOPLEVEL_PATH = cfg.ciVmToplevelPath;
        }
        # For cloud-hypervisor and virtiofsd, add them when CI VM isolation is enabled
        # (indicated by ciVmKernelPath being set). Use nixpkgs packages directly
        # to ensure the paths exist inside the VM's Nix store closure
        // lib.optionalAttrs (cfg.ciVmKernelPath != null) {
          CLOUD_HYPERVISOR_PATH = "${pkgs.cloud-hypervisor}/bin/cloud-hypervisor";
          VIRTIOFSD_PATH = "${pkgs.virtiofsd}/bin/virtiofsd";
        };

      # Use path attribute to add tools to PATH without conflicting with systemd
      path =
        [
          pkgs.nix
          pkgs.git
          pkgs.coreutils
          pkgs.bash
          (pkgs.rustup or pkgs.cargo)
        ]
        ++ lib.optionals (cfg.ciVmKernelPath != null) [pkgs.cloud-hypervisor pkgs.virtiofsd];

      serviceConfig = {
        Type = "simple";
        ExecStart = let
          secretKeyArg =
            if cfg.secretKey != null
            then cfg.secretKey
            else defaultSecretKey;
          args =
            [
              "${cfg.package}/bin/aspen-node"
              "--node-id"
              (toString cfg.nodeId)
              "--cookie"
              cfg.cookie
              "--data-dir"
              cfg.dataDir
              "--storage-backend"
              cfg.storageBackend
              "--iroh-secret-key"
              secretKeyArg
              "--relay-mode"
              cfg.relayMode
              "--bind-port"
              (toString cfg.bindPort)
            ]
            ++ lib.optionals cfg.enableWorkers [
              "--enable-workers"
              "--worker-count"
              (toString cfg.workerCount)
            ]
            ++ lib.optionals cfg.enableCi [
              "--enable-ci"
            ]
            ++ lib.optionals cfg.ciAutoTrigger [
              "--ci-auto-trigger"
            ]
            ++ cfg.extraArgs;
        in
          lib.concatStringsSep " " args;

        Restart = "on-failure";
        RestartSec = "5s";

        # Send output to journal AND console (console goes to serial in VMs)
        # This allows dogfood-vm.sh to extract the cluster ticket from serial logs
        StandardOutput = "journal+console";
        StandardError = "journal+console";

        # State directory management (systemd creates before namespace setup)
        StateDirectory = "aspen";
        StateDirectoryMode = "0750";

        # Security hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        # ProtectHome is disabled to allow Nix CI builds to use /root/.cache/nix
        # The aspen-node process runs in a VM with ephemeral storage, so this is acceptable
        ProtectHome = false;
        PrivateTmp = true;
        ReadWritePaths = ["/tmp" "/root"];

        # Resource limits (Tiger Style)
        # Each CI VM uses 8GB + virtiofsd shmem (~1.2GB per VM)
        # With 2-VM pool: 2*(8+1.2)=~18.4GB + overhead
        # Set MemoryMax to accommodate worst case + 20% headroom
        MemoryMax = "24G";
        # Use MemoryHigh as a soft limit to trigger reclaim before OOM
        MemoryHigh = "20G";
        TasksMax = 4096;
      };
    };
  };
}
