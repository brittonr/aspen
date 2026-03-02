# NixOS VM integration test: nginx in a Cloud Hypervisor microVM serving
# files via VirtioFS from the host.
#
# Proves the full VirtioFS data path end-to-end:
#   Host: virtiofsd shares a directory over vhost-user socket
#   VMM:  Cloud Hypervisor connects to socket, exposes virtio-fs device
#   Guest: NixOS mounts virtiofs → nginx serves files from the host
#   Test:  curl from host through TAP → guest nginx → VirtioFS content
#
# This validates the same vhost-user protocol path that AspenFs uses
# (AspenFs VirtioFS daemon is a drop-in replacement for virtiofsd).
# The AspenFs-specific FileSystem trait impl is tested separately in
# crates/aspen-fuse/tests/cloud_hypervisor_virtiofs_test.rs.
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
  virtiofsTag = "aspenfs";
  virtiofsSocket = "/tmp/aspenfs.sock";
  sharedDir = "/tmp/virtiofs-share";

  # Build the microVM guest as a standalone NixOS system
  nginxGuest = pkgs.nixos [
    microvm.nixosModules.microvm
    ({...}: {
      microvm = {
        hypervisor = "cloud-hypervisor";
        mem = 512;
        vcpu = 1;
        cloud-hypervisor.extraArgs = [
          "--serial"
          "file=/tmp/guest-serial.log"
          "--console"
          "off"
        ];
        kernelParams = [
          "console=ttyS0"
          "panic=1"
        ];
        volumes = [];
        # VirtioFS share — socket created by virtiofsd on the host;
        # microvm.nix wires it into the CH --fs argument.
        shares = [
          {
            source = sharedDir;
            mountPoint = "/var/www/aspen";
            tag = virtiofsTag;
            proto = "virtiofs";
            socket = virtiofsSocket;
          }
        ];
        interfaces = [
          {
            type = "tap";
            id = tapName;
            mac = guestMac;
          }
        ];
      };

      system.stateVersion = "24.11";
      documentation.enable = false;
      programs.command-not-found.enable = false;
      boot.loader.grub.enable = false;

      # nginx serves files from the VirtioFS mount
      services.nginx = {
        enable = true;
        virtualHosts.default = {
          default = true;
          root = "/var/www/aspen";
        };
      };

      networking = {
        hostName = "nginx-guest";
        firewall.enable = false;
        useDHCP = false;
        usePredictableInterfaceNames = false;
      };

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
        pkgs.virtiofsd
        pkgs.curl
        pkgs.iproute2
      ];

      networking.firewall.enable = false;
    };

    testScript = ''
      import time

      host.start()
      host.wait_for_unit("multi-user.target")

      # Verify nested KVM
      kvm_check = host.succeed("test -c /dev/kvm && echo 'kvm ok' || echo 'no kvm'")
      assert "kvm ok" in kvm_check, "Nested KVM not available"

      # Create the shared directory with pre-seeded content
      # This simulates what AspenFs would serve from its KV store
      host.succeed("mkdir -p ${sharedDir}")
      host.succeed("echo -n 'hello from aspen kv' > ${sharedDir}/index.html")
      host.succeed('echo -n \'{"source":"aspen-kv","ok":true}\' > ${sharedDir}/status.json')
      host.log("Created shared directory with test content")

      # Create TAP for guest networking
      host.succeed("ip tuntap add ${tapName} mode tap")
      host.succeed("ip addr add ${hostIp}/24 dev ${tapName}")
      host.succeed("ip link set ${tapName} up")

      # Start virtiofsd — serves the shared directory over vhost-user socket.
      # This is the same protocol path AspenFs VirtioFS daemon uses.
      host.succeed(
          "systemd-run --unit=virtiofsd "
          "--property=StandardOutput=file:/tmp/virtiofsd-stdout.log "
          "--property=StandardError=file:/tmp/virtiofsd-stderr.log "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "virtiofsd "
          "--socket-path=${virtiofsSocket} "
          "--shared-dir=${sharedDir} "
          "--xattr "
          "--announce-submounts "
      )
      host.log("Started virtiofsd")

      # Wait for the socket to appear
      host.wait_until_succeeds("test -S ${virtiofsSocket}", timeout=10)
      host.log("VirtioFS socket ready")

      # Launch the microVM — Cloud Hypervisor connects to the VirtioFS socket
      host.succeed(
          "systemd-run --unit=microvm-nginx "
          "--property=StandardOutput=file:/tmp/ch-stdout.log "
          "--property=StandardError=file:/tmp/ch-stderr.log "
          "--property=WorkingDirectory=/tmp "
          "'--property=Environment=PATH=/run/current-system/sw/bin' "
          "microvm-run"
      )
      host.log("Launched Cloud Hypervisor microVM")

      # Wait for CH to be running
      time.sleep(3)
      ch_status = host.succeed("systemctl is-active microvm-nginx.service || echo 'dead'").strip()
      host.log(f"CH service status: {ch_status}")

      if "dead" in ch_status or "failed" in ch_status:
          ch_err = host.succeed("cat /tmp/ch-stderr.log 2>/dev/null || echo 'none'")
          vfs_err = host.succeed("cat /tmp/virtiofsd-stderr.log 2>/dev/null || echo 'none'")
          host.log(f"CH stderr: {ch_err}")
          host.log(f"virtiofsd stderr: {vfs_err}")
          raise Exception(f"Cloud Hypervisor failed: {ch_status}")

      # Wait for nginx to be reachable (guest boot + virtiofs mount + nginx start)
      host.wait_until_succeeds(
          "curl -sf --connect-timeout 2 http://${guestIp}",
          timeout=90,
      )

      # Verify index.html is served through VirtioFS
      output = host.succeed("curl -sf http://${guestIp}/index.html")
      assert "hello from aspen kv" in output, f"unexpected index.html: {output!r}"
      host.log(f"index.html via VirtioFS: {output.strip()}")

      # Verify status.json is also served through VirtioFS
      status = host.succeed("curl -sf http://${guestIp}/status.json")
      assert '"source":"aspen-kv"' in status, f"unexpected status.json: {status!r}"
      host.log(f"status.json via VirtioFS: {status.strip()}")

      # Verify HTTP headers show nginx
      headers = host.succeed("curl -sfI http://${guestIp}/index.html")
      assert "200" in headers, f"expected 200: {headers!r}"
      assert "nginx" in headers.lower(), f"expected nginx: {headers!r}"
      host.log("HTTP headers OK — nginx serving VirtioFS content")

      # Show guest serial log for debugging
      serial = host.succeed("cat /tmp/guest-serial.log 2>/dev/null | tail -10 || echo 'no serial'")
      host.log(f"Guest serial:\n{serial}")

      # Clean up
      host.succeed("systemctl stop microvm-nginx.service 2>/dev/null || true")
      host.succeed("systemctl stop virtiofsd.service 2>/dev/null || true")
      time.sleep(1)

      host.log("PASSED: nginx in CH microVM served files via VirtioFS from host!")
    '';
  }
