# aspen2: Secondary node — 64GB RAM, 1.8TB NVMe
{
  config,
  pkgs,
  lib,
  ...
}: {
  networking.hostName = "aspen2";

  # Boot
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVars = true;

  # Networking
  networking.useDHCP = true;
  networking.firewall.enable = true;

  # Tailscale
  services.tailscale.enable = true;

  # Docker
  virtualisation.docker.enable = true;

  # Nix settings
  nix.settings = {
    experimental-features = ["nix-command" "flakes"];
    trusted-users = ["root"];
    max-jobs = "auto";
  };

  # Basic packages
  environment.systemPackages = with pkgs; [
    git
    htop
    tmux
    jq
    ripgrep
  ];

  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";

  system.stateVersion = "24.11";
}
