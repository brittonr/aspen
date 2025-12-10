# NixOS module for Aspen distributed consensus node.
#
# This module provides a systemd service definition for running aspen-node
# instances, supporting both single-node and multi-node cluster deployments.
#
# Example usage:
#   services.aspen-node = {
#     enable = true;
#     nodeId = 1;
#     httpAddr = "0.0.0.0:8301";
#     dataDir = "/var/lib/aspen/node-1";
#   };
{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.aspen-node;

  # Build the command-line arguments from configuration
  buildArgs = let
    optionalArg = name: value:
      if value != null
      then ["--${name}" (toString value)]
      else [];

    boolArg = name: value:
      if value
      then ["--${name}"]
      else [];

    listArg = name: values:
      concatMap (v: ["--${name}" v]) values;
  in
    flatten [
      ["--node-id" (toString cfg.nodeId)]
      (optionalArg "data-dir" cfg.dataDir)
      (optionalArg "storage-backend" cfg.storageBackend)
      (optionalArg "http-addr" cfg.httpAddr)
      (optionalArg "host" cfg.host)
      (optionalArg "port" cfg.ractorPort)
      (optionalArg "cookie" cfg.cookie)
      (optionalArg "heartbeat-interval-ms" cfg.raft.heartbeatIntervalMs)
      (optionalArg "election-timeout-min-ms" cfg.raft.electionTimeoutMinMs)
      (optionalArg "election-timeout-max-ms" cfg.raft.electionTimeoutMaxMs)
      (optionalArg "iroh-secret-key" cfg.iroh.secretKey)
      (optionalArg "iroh-relay-url" cfg.iroh.relayUrl)
      (optionalArg "ticket" cfg.iroh.ticket)
      (optionalArg "dns-discovery-url" cfg.iroh.dnsDiscoveryUrl)
      (optionalArg "pkarr-relay-url" cfg.iroh.pkarrRelayUrl)
      (boolArg "disable-gossip" cfg.iroh.disableGossip)
      (boolArg "disable-mdns" cfg.iroh.disableMdns)
      (boolArg "enable-dns-discovery" cfg.iroh.enableDnsDiscovery)
      (boolArg "enable-pkarr" cfg.iroh.enablePkarr)
      (listArg "peers" cfg.peers)
      cfg.extraArgs
    ];
in {
  options.services.aspen-node = {
    enable = mkEnableOption "Aspen distributed consensus node";

    package = mkOption {
      type = types.package;
      description = "The aspen-node package to use.";
    };

    nodeId = mkOption {
      type = types.int;
      description = "Unique Raft node identifier (must be non-zero).";
      example = 1;
    };

    dataDir = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Directory for persistent data storage (metadata, Raft logs, state machine).
        Defaults to /var/lib/aspen/node-{nodeId} if not specified.
      '';
    };

    storageBackend = mkOption {
      type = types.nullOr (types.enum ["inmemory" "sqlite" "redb"]);
      default = "sqlite";
      description = ''
        Storage backend for Raft log and state machine.
        - sqlite: ACID-compliant SQLite storage (recommended for production)
        - inmemory: Fast, non-durable (data lost on restart), good for testing
        - redb: ACID-compliant redb storage (deprecated)
      '';
    };

    httpAddr = mkOption {
      type = types.nullOr types.str;
      default = "127.0.0.1:8080";
      description = "Address for the HTTP control API.";
      example = "0.0.0.0:8301";
    };

    host = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Hostname recorded in the NodeServer's identity (informational).";
    };

    ractorPort = mkOption {
      type = types.nullOr types.port;
      default = null;
      description = ''
        Port for the Ractor node listener.
        Use 0 to request an OS-assigned port.
      '';
    };

    cookie = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Shared cookie for authenticating Ractor nodes in the cluster.";
    };

    peers = mkOption {
      type = types.listOf types.str;
      default = [];
      description = ''
        Peer node addresses in format: node_id@endpoint_id:relay_url:direct_addrs.
        Can be specified multiple times for multiple peers.
      '';
      example = ["1@abc123:https://relay.example.com:192.168.1.10:4001"];
    };

    raft = {
      heartbeatIntervalMs = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Raft heartbeat interval in milliseconds.";
      };

      electionTimeoutMinMs = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Minimum Raft election timeout in milliseconds.";
      };

      electionTimeoutMaxMs = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Maximum Raft election timeout in milliseconds.";
      };
    };

    iroh = {
      secretKey = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Hex-encoded Iroh secret key (64 hex characters = 32 bytes).
          If not provided, a new key is generated on startup.
        '';
      };

      relayUrl = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Iroh relay server URL.";
      };

      disableGossip = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Disable iroh-gossip for automatic peer discovery.
          When disabled, only manual peers are used.
        '';
      };

      ticket = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Aspen cluster ticket for gossip-based bootstrap.
          Contains the gossip topic ID and bootstrap peer endpoints.
        '';
      };

      disableMdns = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Disable mDNS discovery for local network peer discovery.
        '';
      };

      enableDnsDiscovery = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Enable DNS discovery for production peer discovery.
          Uses n0's public DNS service by default.
        '';
      };

      dnsDiscoveryUrl = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Custom DNS discovery service URL.";
      };

      enablePkarr = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Enable Pkarr publisher for distributed peer discovery.
          Publishes node addresses to a Pkarr relay (DHT-based).
        '';
      };

      pkarrRelayUrl = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Custom Pkarr relay URL.";
      };
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Extra command-line arguments to pass to aspen-node.";
    };

    environment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Environment variables to set for the aspen-node service.";
      example = {
        RUST_LOG = "info";
        RUST_BACKTRACE = "1";
      };
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open firewall ports for Aspen node communication.";
    };

    user = mkOption {
      type = types.str;
      default = "aspen";
      description = "User account under which aspen-node runs.";
    };

    group = mkOption {
      type = types.str;
      default = "aspen";
      description = "Group under which aspen-node runs.";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.nodeId > 0;
        message = "services.aspen-node.nodeId must be greater than 0";
      }
    ];

    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home =
        if cfg.dataDir != null
        then cfg.dataDir
        else "/var/lib/aspen/node-${toString cfg.nodeId}";
      createHome = true;
    };

    users.groups.${cfg.group} = {};

    systemd.services.aspen-node = {
      description = "Aspen Distributed Consensus Node";
      after = ["network.target"];
      wantedBy = ["multi-user.target"];

      environment =
        {
          RUST_LOG = mkDefault "info";
        }
        // cfg.environment;

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${cfg.package}/bin/aspen-node ${escapeShellArgs buildArgs}";
        Restart = "on-failure";
        RestartSec = "5s";

        # Security hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = ["AF_INET" "AF_INET6" "AF_UNIX"];
        RestrictNamespaces = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;

        # Allow write access to data directory
        ReadWritePaths =
          if cfg.dataDir != null
          then [cfg.dataDir]
          else ["/var/lib/aspen"];

        # Working directory
        WorkingDirectory =
          if cfg.dataDir != null
          then cfg.dataDir
          else "/var/lib/aspen/node-${toString cfg.nodeId}";

        # State directory management
        StateDirectory = "aspen/node-${toString cfg.nodeId}";
        StateDirectoryMode = "0750";
      };
    };

    # Open firewall ports if requested
    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = let
        # Parse HTTP port from address
        httpPort =
          if cfg.httpAddr != null
          then let
            parts = splitString ":" cfg.httpAddr;
          in
            if length parts == 2
            then toInt (elemAt parts 1)
            else 8080
          else 8080;
      in
        [httpPort]
        ++ (
          if cfg.ractorPort != null
          then [cfg.ractorPort]
          else [26000]
        );

      # Iroh uses UDP for QUIC
      allowedUDPPortRanges = [
        {
          from = 4000;
          to = 4100;
        } # Default Iroh port range
      ];
    };
  };
}
