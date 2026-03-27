# Aspen example clan — 3-node physical cluster
#
# Machines:
#   gmk1: Intel N150, 16GB RAM, 512GB NVMe (192.168.1.146) — Raft node 1, leader
#   gmk2: Intel N150, 16GB RAM, 512GB NVMe (192.168.1.114) — Raft node 2, follower
#   gmk3: Intel N150, 16GB RAM, 512GB NVMe (192.168.1.40)  — Raft node 3, follower
#
# Install: `clan machines install gmk1 -i ~/.ssh/framework`
# Update:  `clan machines update gmk1`
{self, ...}: let
  inputs = self.inputs;

  # Shared machine config applied to all nodes
  commonModule = {
    config,
    pkgs,
    lib,
    ...
  }: {
    # NOTE: disko.nix removed — each machine defines its own fileSystems
    # in configuration.nix to match actual partition layout (by UUID).

    # Boot (GRUB EFI)
    boot.loader.grub.enable = true;
    boot.loader.grub.efiSupport = true;
    boot.loader.grub.efiInstallAsRemovable = true;
    boot.loader.grub.device = "nodev";

    # Networking
    networking.useDHCP = true;
    networking.firewall.enable = true;
    networking.firewall.allowedUDPPorts = [7777]; # iroh QUIC

    # Tailscale
    services.tailscale.enable = true;

    # Nix
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

    environment.systemPackages = with pkgs; [
      git
      htop
      tmux
      jq
      ripgrep
    ];

    system.stateVersion = "24.11";
  };
in {
  meta.name = "aspen-cluster";

  inventory.machines = {
    gmk1 = {};
    gmk2 = {};
    gmk3 = {};
  };

  machines = {
    gmk1 = {
      config,
      pkgs,
      ...
    }: {
      imports = [
        commonModule
        inputs.aspen.nixosModules.aspen-node
        ./machines/gmk1/hardware-configuration.nix
      ];

      networking.hostName = "gmk1";
      clan.core.networking.targetHost = "root@192.168.1.146";

      services.aspen.node = {
        enable = true;
        package = inputs.aspen.packages.${pkgs.system}.aspen-node-clan;
        nodeId = 1;
        cookie = "aspen-example-cluster";
        bindPort = 7777;
        relayMode = "default";
        enableCi = true;
        enableWorkers = true;
        workerCount = 2;
        ciLocalExecutor = true;
        logLevel = "info,aspen=debug";
      };
    };

    gmk2 = {
      config,
      pkgs,
      ...
    }: {
      imports = [
        commonModule
        inputs.aspen.nixosModules.aspen-node
        ./machines/gmk2/hardware-configuration.nix
      ];

      networking.hostName = "gmk2";
      clan.core.networking.targetHost = "root@192.168.1.114";

      services.aspen.node = {
        enable = true;
        package = inputs.aspen.packages.${pkgs.system}.aspen-node-clan;
        nodeId = 2;
        cookie = "aspen-example-cluster";
        bindPort = 7777;
        relayMode = "default";
        enableCi = true;
        enableWorkers = true;
        workerCount = 2;
        ciLocalExecutor = true;
        logLevel = "info,aspen=debug";
      };
    };

    gmk3 = {
      config,
      pkgs,
      ...
    }: {
      imports = [
        commonModule
        inputs.aspen.nixosModules.aspen-node
        ./machines/gmk3/hardware-configuration.nix
      ];

      networking.hostName = "gmk3";
      clan.core.networking.targetHost = "root@192.168.1.40";

      services.aspen.node = {
        enable = true;
        package = inputs.aspen.packages.${pkgs.system}.aspen-node-clan;
        nodeId = 3;
        cookie = "aspen-example-cluster";
        bindPort = 7777;
        relayMode = "default";
        enableCi = true;
        enableWorkers = true;
        workerCount = 2;
        ciLocalExecutor = true;
        logLevel = "info,aspen=debug";
      };
    };
  };
}
