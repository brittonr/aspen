# Aspen example clan — 2-node physical cluster
#
# Machines:
#   aspen1: AMD Ryzen AI MAX+ 395, 128GB RAM, 3.6TB NVMe (Raft node 1, leader)
#   aspen2: AMD Ryzen AI MAX+ 395, 64GB RAM, 1.8TB NVMe  (Raft node 2, follower)
#
# Connectivity: iroh-ssh proxy (P2P QUIC, NAT-traversing)
# Deploy: `clan machines update aspen1` / `clan machines update aspen2`
{inputs, ...}: {
  meta.name = "aspen-cluster";

  inventory.machines = {
    aspen1 = {};
    aspen2 = {};
  };

  inventory.instances = {
    # SSH access
    sshd = {
      roles.server.tags.all = {};
      roles.server.settings.authorizedKeys = {
        "britton-framework" = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILYzh3yIsSTOYXkJMFHBKzkakoDfonm3/RED5rqMqhIO britton@framework";
      };
    };

    # Root password
    user-root = {
      module.name = "users";
      roles.default.tags.all = {};
      roles.default.settings = {
        user = "root";
        prompt = true;
      };
    };
  };

  machines = {
    aspen1 = {
      config,
      pkgs,
      lib,
      ...
    }: {
      imports = [
        inputs.aspen.nixosModules.aspen-node
        ./machines/aspen1/hardware-configuration.nix
      ];

      services.aspen.node = {
        enable = true;
        package = inputs.aspen.packages.${pkgs.system}.aspen-node-clan;
        nodeId = 1;
        cookie = "aspen-example-cluster";
        bindPort = 7777;
        relayMode = "default"; # Use iroh relays for NAT traversal
        enableCi = true;
        enableWorkers = true;
        workerCount = 4; # 32 cores, plenty of headroom
        ciLocalExecutor = true;
        logLevel = "info,aspen=debug";
      };

      networking.firewall.allowedUDPPorts = [7777]; # iroh QUIC
    };

    aspen2 = {
      config,
      pkgs,
      lib,
      ...
    }: {
      imports = [
        inputs.aspen.nixosModules.aspen-node
        ./machines/aspen2/hardware-configuration.nix
      ];

      services.aspen.node = {
        enable = true;
        package = inputs.aspen.packages.${pkgs.system}.aspen-node-clan;
        nodeId = 2;
        cookie = "aspen-example-cluster";
        bindPort = 7777;
        relayMode = "default";
        enableCi = true;
        enableWorkers = true;
        workerCount = 4;
        ciLocalExecutor = true;
        logLevel = "info,aspen=debug";
      };

      networking.firewall.allowedUDPPorts = [7777];
    };
  };
}
