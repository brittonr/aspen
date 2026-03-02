# NixOS VM integration test: nginx running inside a microvm.nix guest.
#
# Boots a QEMU VM (the "host") which launches a Cloud Hypervisor microVM
# (the "guest") running nginx. The host then curls the guest to verify
# the full microVM boot path works end-to-end.
#
# Requires nested KVM (enabled by default on most systems).
#
# Build & run:
#   nix build .#checks.x86_64-linux.microvm-nginx-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.microvm-nginx-test.driverInteractive
#   ./result/bin/nixos-test-driver --interactive
{
  pkgs,
  microvm,
}: let
  guestIp = "10.10.0.2";
  hostIp = "10.10.0.1";
  tapName = "vm-nginx";
  guestMac = "02:00:00:00:00:01";

  # Build the microVM guest as a standalone NixOS system
  nginxGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
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
        interfaces = [
          {
            type = "tap";
            id = tapName;
            mac = guestMac;
          }
        ];
        cloud-hypervisor.extraArgs = [
          "--serial"
          "file=/tmp/guest-serial.log"
          "--console"
          "off"
        ];
      };

      system.stateVersion = "24.11";
      documentation.enable = false;
      programs.command-not-found.enable = false;
      boot.loader.grub.enable = false;

      services.nginx = {
        enable = true;
        virtualHosts.default = {
          default = true;
          locations."/".return = ''200 "hello from microvm\n"'';
        };
      };

      networking = {
        hostName = "nginx-guest";
        firewall.enable = false;
        useDHCP = false;
        # Disable predictable names so virtio-net becomes eth0
        usePredictableInterfaceNames = false;
      };

      # Use systemd-networkd to assign the static IP.
      # Match any ether device so it works regardless of interface naming.
      systemd.network = {
        enable = true;
        networks."10-guest" = {
          matchConfig.Type = "ether";
          address = ["${guestIp}/24"];
          networkConfig.DHCP = "no";
        };
      };
    })
  ];

  # The cloud-hypervisor runner for this guest
  guestRunner = nginxGuest.config.microvm.runner.cloud-hypervisor;
in
  pkgs.testers.nixosTest {
    name = "microvm-nginx";

    nodes.host = {
      config,
      pkgs,
      lib,
      ...
    }: {
      # Nested KVM: pass host CPU features through QEMU
      virtualisation.qemu.options = [
        "-enable-kvm"
        "-cpu"
        "host"
      ];
      virtualisation.memorySize = 2048;
      virtualisation.cores = 2;

      environment.systemPackages = [
        guestRunner
        pkgs.cloud-hypervisor
        pkgs.curl
        pkgs.iproute2
      ];

      networking.firewall.enable = false;
    };

    testScript = ''
      import time

      host.start()
      host.wait_for_unit("multi-user.target")

      # Verify nested KVM is available inside the QEMU host
      kvm_check = host.succeed("test -c /dev/kvm && echo 'kvm ok' || echo 'no kvm'")
      host.log(f"KVM check: {kvm_check.strip()}")
      assert "kvm ok" in kvm_check, "Nested KVM not available inside QEMU host"

      # Create and configure TAP device for the guest
      host.succeed("ip tuntap add ${tapName} mode tap")
      host.succeed("ip addr add ${hostIp}/24 dev ${tapName}")
      host.succeed("ip link set ${tapName} up")
      tap_info = host.succeed("ip addr show ${tapName}")
      host.log(f"TAP device:\n{tap_info}")

      # Launch the microVM via systemd-run so it's properly daemonized.
      # microvm-run uses exec to replace with cloud-hypervisor (full store paths),
      # but the script also calls `rm` which needs PATH set.
      host.succeed(
          "systemd-run --unit=microvm-nginx "
          "--property=StandardOutput=file:/tmp/ch-stdout.log "
          "--property=StandardError=file:/tmp/ch-stderr.log "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "microvm-run"
      )
      host.log("Launched microvm-run via systemd-run")

      # Give Cloud Hypervisor time to start and guest to boot
      time.sleep(3)

      # Always dump diagnostic info
      ch_status = host.succeed("systemctl is-active microvm-nginx.service || echo 'dead'").strip()
      host.log(f"Service status: {ch_status}")

      ch_log = host.succeed("cat /tmp/ch-stdout.log 2>/dev/null || echo 'no stdout'")
      ch_err = host.succeed("cat /tmp/ch-stderr.log 2>/dev/null || echo 'no stderr'")
      jlog = host.succeed("journalctl -u microvm-nginx.service --no-pager -n 20 2>/dev/null || echo 'no journal'")
      host.log(f"CH stdout:\n{ch_log}")
      host.log(f"CH stderr:\n{ch_err}")
      host.log(f"Journal:\n{jlog}")

      # Check KVM access
      kvm_test = host.succeed("test -c /dev/kvm && echo 'kvm ok' || echo 'no kvm'").strip()
      host.log(f"KVM: {kvm_test}")

      # Check the microvm-run script can find cloud-hypervisor
      host.succeed("head -20 $(which microvm-run)")

      if "dead" in ch_status or "failed" in ch_status:
          raise Exception(f"Cloud Hypervisor service failed: {ch_status}")

      # Poll until curl succeeds (guest boots + nginx starts)
      host.wait_until_succeeds(
          "curl -sf --connect-timeout 2 http://${guestIp}",
          timeout=90,
      )

      # Verify the response content
      output = host.succeed("curl -sf http://${guestIp}")
      assert "hello from microvm" in output, f"unexpected response: {output!r}"
      host.log(f"Got response from microVM nginx: {output.strip()}")

      # Verify it's actually serving HTTP properly (check headers)
      headers = host.succeed("curl -sfI http://${guestIp}")
      assert "200" in headers, f"expected 200 in headers: {headers!r}"
      assert "nginx" in headers.lower(), f"expected nginx in headers: {headers!r}"
      host.log("HTTP headers OK")

      # Show guest serial log
      serial = host.succeed("cat /tmp/guest-serial.log 2>/dev/null | tail -20 || echo 'no serial log'")
      host.log(f"Guest serial (last 20 lines):\n{serial}")

      # Clean up
      host.succeed("systemctl stop microvm-nginx.service 2>/dev/null || pkill -f cloud-hypervisor || true")
      time.sleep(2)

      host.log("microvm-nginx test passed!")
    '';
  }
