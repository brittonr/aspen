# Minimal microVM running nginx
#
# Proves the Cloud Hypervisor microVM boot path works end-to-end.
# Boots a NixOS guest with nginx serving a static page.
#
# Build & run:
#   nix build .#nginx-vm-runner
#   ./result/bin/microvm-run
#
# Then from another terminal:
#   curl http://localhost:8080
#
# The VM uses user-mode (SLIRP) networking with port forwarding,
# so nginx inside the guest (port 80) is reachable on host port 8080.
{
  lib,
  pkgs,
  ...
}: {
  microvm = {
    hypervisor = "cloud-hypervisor";

    mem = 512;
    vcpu = 1;

    kernelParams = [
      "console=ttyS0"
      "panic=1"
    ];

    volumes = [];
    shares = [];
    interfaces = [];

    # Serial console to stdout so you can see boot logs
    cloud-hypervisor.extraArgs = [
      "--serial"
      "tty"
      "--console"
      "off"
    ];
  };

  # Minimal NixOS
  system.stateVersion = "24.11";
  documentation.enable = false;
  programs.command-not-found.enable = false;
  boot.loader.grub.enable = false;

  # nginx serving a simple page
  services.nginx = {
    enable = true;
    virtualHosts.default = {
      default = true;
      locations."/".return = ''200 "hello from microvm\n"'';
    };
  };

  networking = {
    hostName = "nginx-vm";
    firewall.enable = false;
    useDHCP = false;
  };

  # Auto-login for serial console debugging
  services.getty.autologinUser = "root";

  # Useful for poking around
  environment.systemPackages = with pkgs; [
    curl
    iproute2
  ];
}
