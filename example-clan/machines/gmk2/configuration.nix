# gmk2: Node 2 — Intel N150, 16GB RAM, 512GB NVMe
# IP: 192.168.1.114
{
  config,
  pkgs,
  lib,
  ...
}: {
  networking.hostName = "gmk2";

  # Boot (GRUB EFI — matches existing install)
  boot.loader.grub.enable = true;
  boot.loader.grub.efiSupport = true;
  boot.loader.grub.efiInstallAsRemovable = true;
  boot.loader.grub.device = "nodev";

  # Filesystems — by UUID (from blkid)
  fileSystems."/" = {
    device = "/dev/disk/by-uuid/1b153404-fce7-4cfa-8a3c-a42158e865fa";
    fsType = "ext4";
  };
  fileSystems."/boot" = {
    device = "/dev/disk/by-uuid/D2BB-E768";
    fsType = "vfat";
    options = ["fmask=0022" "dmask=0022"];
  };

  # Networking
  networking.useDHCP = true;
  networking.firewall.enable = true;

  # Tailscale
  services.tailscale.enable = true;

  # Nix settings
  nix.settings = {
    experimental-features = ["nix-command" "flakes"];
    trusted-users = ["root"];
    max-jobs = "auto";
  };

  # SSH
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILYzh3yIsSTOYXkJMFHBKzkakoDfonm3/RED5rqMqhIO britton@framework"
  ];

  # Basic packages
  environment.systemPackages = with pkgs; [
    git
    htop
    tmux
    jq
    ripgrep
  ];

  system.stateVersion = "24.11";
}
