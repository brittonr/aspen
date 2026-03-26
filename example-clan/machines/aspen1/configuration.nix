# aspen1: Primary node — 128GB RAM, 3.6TB NVMe
{
  config,
  pkgs,
  lib,
  ...
}: {
  networking.hostName = "aspen1";

  # Boot
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVars = true;

  # Networking
  networking.useDHCP = true;
  networking.firewall.enable = true;

  # Tailscale (already present on the machine)
  services.tailscale.enable = true;

  # Docker (already present)
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

  # iroh-ssh is handled externally (already configured)
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";

  system.stateVersion = "24.11";
}
