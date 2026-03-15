# NixOS module overlay for using aspen-sops-install-secrets as the sops-nix backend.
#
# Import alongside sops-nix to use Aspen Transit (or age) for secret decryption.
#
# Usage:
#   imports = [
#     sops-nix.nixosModules.sops
#     ./nix/modules/aspen-sops.nix
#   ];
#
#   sops.aspenTransit.clusterTicket = "aspen1q...";
#   sops.secrets."db-password" = {
#     sopsFile = ./secrets.yaml;
#   };
{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.sops;
in {
  options.sops.aspenTransit = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Use aspen-sops-install-secrets as the sops-nix backend.
        When enabled, replaces the Go sops-install-secrets with the Rust
        implementation that supports Aspen Transit key management.
      '';
    };

    package = lib.mkOption {
      type = lib.types.package;
      description = ''
        The aspen-sops-install-secrets package.
      '';
    };

    clusterTicket = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = ''
        Aspen cluster ticket for Transit-based decryption.
        When set, the ASPEN_CLUSTER_TICKET environment variable is passed
        to sops-install-secrets. When null, only age-based decryption is used.
      '';
    };

    clusterTicketFile = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = ''
        Path to a file containing the Aspen cluster ticket.
        Read at activation time. Alternative to clusterTicket for
        keeping tickets out of the Nix store.
      '';
    };
  };

  config = lib.mkIf cfg.aspenTransit.enable {
    # Override sops-nix's package with our Rust implementation
    sops.package = lib.mkForce cfg.aspenTransit.package;

    # Pass cluster ticket to the binary via environment
    sops.environment = lib.mkMerge [
      (lib.mkIf (cfg.aspenTransit.clusterTicket != null) {
        ASPEN_CLUSTER_TICKET = cfg.aspenTransit.clusterTicket;
      })
      (lib.mkIf (cfg.aspenTransit.clusterTicketFile != null) {
        ASPEN_CLUSTER_TICKET = "$(cat ${cfg.aspenTransit.clusterTicketFile})";
      })
    ];
  };
}
