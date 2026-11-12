{ config, lib, pkgs, ... }:

let
  cfg = config.services.molten;
  state = cfg.stateDir;
  evidence = "${state}/evidence";
  startScript = pkgs.writeShellScript "molten-start" ''
    set -euo pipefail
    install -d -m 0700 ${lib.escapeShellArg state}
    install -d -m 0700 ${lib.escapeShellArg evidence}
    if [ ! -f ${lib.escapeShellArg "${state}/config.preserves"} ]; then
      ${lib.escapeShellArg cfg.package}/bin/molten node init \
        --state-root ${lib.escapeShellArg state} \
        --node-id ${lib.escapeShellArg cfg.nodeId} \
        --config-out ${lib.escapeShellArg "${evidence}/node-config.preserves"} \
        --identity-receipt-out ${lib.escapeShellArg "${evidence}/identity.preserves"}
    fi
    install -m 0600 ${lib.escapeShellArg cfg.sourceTrustFile} ${lib.escapeShellArg "${state}/source-gate-trust.json"}
    ${lib.escapeShellArg cfg.package}/bin/molten node run \
      --state-root ${lib.escapeShellArg state} \
      --startup-out ${lib.escapeShellArg "${evidence}/startup.preserves"}
  '';
  loopScript = pkgs.writeShellScript "molten-bounded-loop" ''
    set -euo pipefail
    rm -f ${lib.escapeShellArg "${state}/stop"}
    trap 'touch ${lib.escapeShellArg "${state}/stop"}' TERM INT
    while [ ! -e ${lib.escapeShellArg "${state}/stop"} ]; do
      ${lib.escapeShellArg cfg.package}/bin/molten node serve \
        --state-root ${lib.escapeShellArg state} \
        --max-ticks ${toString cfg.maxTicks} \
        --max-requests-per-tick ${toString cfg.maxRequestsPerTick} \
        --receipt-out ${lib.escapeShellArg "${evidence}/service.preserves"}
    done
  '';
  stopScript = pkgs.writeShellScript "molten-stop" ''
    set -euo pipefail
    touch ${lib.escapeShellArg "${state}/stop"}
    ${lib.escapeShellArg cfg.package}/bin/molten node stop \
      --state-root ${lib.escapeShellArg state} \
      --shutdown-out ${lib.escapeShellArg "${evidence}/shutdown.preserves"} \
      --receipt-out ${lib.escapeShellArg "${evidence}/shutdown-control.preserves"} || true
  '';
in
{
  options.services.molten = {
    enable = lib.mkEnableOption "Molten node service";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      example = lib.literalExpression "inputs.molten.packages.${pkgs.system}.default";
      description = "Molten package to run. The flake does not assume a nixpkgs package name.";
    };

    nodeId = lib.mkOption {
      type = lib.types.str;
      example = "node:example";
      description = "Node identity used for the first init only.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/molten";
      description = "Absolute state directory owned by the molten service user.";
    };

    sourceTrustFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "/run/secrets/molten-source-gate-trust.json";
      description = ''
        Operator-selected `source-gate-trust.json`. The file is copied into the
        state root at service start. Do not place secret material here; the
        startup source binding is authenticated evidence, not authority.
      '';
    };

    maxTicks = lib.mkOption {
      type = lib.types.ints.positive;
      default = 3600;
      description = "Bounded ticks per control-service run before the supervised loop restarts it.";
    };

    maxRequestsPerTick = lib.mkOption {
      type = lib.types.ints.positive;
      default = 16;
      description = "Bounded control requests processed in one tick.";
    };

    restartSeconds = lib.mkOption {
      type = lib.types.ints.positive;
      default = 5;
      description = "Delay before restarting a failed bounded-loop run.";
    };

    timeoutStartSeconds = lib.mkOption {
      type = lib.types.ints.positive;
      default = 3600;
      description = "Startup budget for source-closure measurement and node startup.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.package != null;
        message = "services.molten.package must be set to the exact Molten package for the selected profile";
      }
      {
        assertion = cfg.sourceTrustFile != null;
        message = "services.molten.sourceTrustFile must select the operator-approved source-gate trust document";
      }
      {
        assertion = lib.hasPrefix "/" cfg.stateDir;
        message = "services.molten.stateDir must be absolute";
      }
    ];

    users.groups.molten = { };
    users.users.molten = {
      isSystemUser = true;
      group = "molten";
      home = cfg.stateDir;
    };

    systemd.services.molten = {
      description = "Molten node (bounded service loop)";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.coreutils ];
      serviceConfig = {
        Type = "simple";
        User = "molten";
        Group = "molten";
        WorkingDirectory = cfg.stateDir;
        Environment = [ "HOME=${cfg.stateDir}" ];
        ExecStartPre = startScript;
        ExecStart = loopScript;
        ExecStop = stopScript;
        Restart = "on-failure";
        RestartSec = cfg.restartSeconds;
        TimeoutStartSec = cfg.timeoutStartSeconds;
        TimeoutStopSec = 30;
        KillMode = "mixed";
        ReadWritePaths = [ cfg.stateDir ];
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
      } // lib.optionalAttrs (cfg.stateDir == "/var/lib/molten") {
        StateDirectory = "molten";
        StateDirectoryMode = "0700";
      };
    };
  };
}
