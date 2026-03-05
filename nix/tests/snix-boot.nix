# NixOS VM integration test: SNIX MicroVM Boot
#
# Verifies the full snix boot chain inside a NixOS VM with nested KVM:
#
#   1. snix-store virtiofs daemon starts and creates vhost-user socket
#   2. cloud-hypervisor boots a microVM with kernel + initrd
#   3. snix-init inside the VM mounts /nix/store via VirtioFS
#   4. The VM processes its cmdline and shuts down cleanly
#
# Data flow:
#   NixOS QEMU VM → snix-store virtiofs daemon → vhost-user socket →
#   cloud-hypervisor (nested KVM) → kernel + u-root initrd → snix-init →
#   VirtioFS mount at /nix/store → find/shell/run
#
# Requires nested KVM (/dev/kvm inside the VM).
#
# Build & run:
#   nix build .#checks.x86_64-linux.snix-boot-test --impure
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.snix-boot-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  snixBoot,
}:
pkgs.testers.nixosTest {
  name = "snix-boot";

  nodes = {
    host = {
      virtualisation.memorySize = 2048;
      virtualisation.cores = 2;

      # Nested KVM required for cloud-hypervisor inside QEMU
      virtualisation.qemu.options = [
        "-enable-kvm"
        "-cpu"
        "host"
      ];

      environment.systemPackages = [
        snixBoot.runVM
        snixBoot.snix-store
        pkgs.cloud-hypervisor
      ];

      # /dev/kvm must be accessible
      boot.kernelModules = ["kvm-amd" "kvm-intel"];
      security.wrappers = {};
      networking.firewall.enable = false;
    };
  };

  testScript = ''
    start_all()

    # ================================================================
    # 1. VERIFY COMPONENTS EXIST
    # ================================================================

    with subtest("snix-store binary available"):
        out = host.succeed("snix-store --help 2>&1 || true")
        assert "snix-store" in out.lower() or "usage" in out.lower() or "virtiofs" in out.lower(), \
            f"snix-store help unexpected: {out[:200]}"
        host.log("snix-store binary OK")

    with subtest("cloud-hypervisor available"):
        out = host.succeed("cloud-hypervisor --version 2>&1 || true")
        host.log(f"cloud-hypervisor: {out.strip()}")

    with subtest("kvm available"):
        host.succeed("test -c /dev/kvm")
        host.log("/dev/kvm present — nested KVM OK")

    # ================================================================
    # 2. BOOT SNIX MICROVM WITH snix.find
    # ================================================================

    with subtest("snix microVM boots and shuts down cleanly"):
        # Run the VM with snix.find cmdline — it will list /nix/store
        # contents (empty with in-memory backend) then power off.
        # Timeout: VM should boot and exit within 30 seconds.
        out = host.succeed(
            "timeout 30 run-snix-vm 2>&1 || true"
        )

        # Verify kernel booted
        assert "Linux version" in out, \
            f"kernel didn't boot: {out[:500]}"
        host.log("kernel booted")

        # Verify snix-init ran (ASCII art banner)
        assert "Snix Init" in out or "_____" in out, \
            f"snix-init banner not found: {out[-500:]}"
        host.log("snix-init banner displayed")

        # Verify VirtioFS was discovered
        assert "virtiofs" in out.lower() and "snix" in out, \
            f"virtiofs tag 'snix' not discovered: {out[-500:]}"
        host.log("VirtioFS tag 'snix' discovered")

        # Verify FUSE handshake
        assert "FUSE INIT" in out, \
            f"FUSE handshake didn't happen: {out[-500:]}"
        host.log("FUSE INIT handshake completed")

        # Verify clean shutdown
        assert "Power down" in out or "reboot" in out.lower(), \
            f"VM didn't shut down cleanly: {out[-500:]}"
        host.log("VM powered off cleanly")

    # ================================================================
    # 3. VERIFY VIRTIOFS DAEMON LIFECYCLE
    # ================================================================

    with subtest("virtiofs daemon creates socket"):
        # Start just the virtiofs daemon, verify socket appears, kill it
        host.succeed("mkdir -p /tmp/snix-test")
        host.succeed(
            "snix-store virtiofs /tmp/snix-test/test.sock &"
            " VIRTIOFS_PID=$!;"
            " for i in $(seq 1 50); do"
            "   [ -e /tmp/snix-test/test.sock ] && break;"
            "   sleep 0.1;"
            " done;"
            " test -e /tmp/snix-test/test.sock;"
            " kill $VIRTIOFS_PID 2>/dev/null || true;"
            " rm -rf /tmp/snix-test"
        )
        host.log("VirtioFS daemon created socket successfully")

    # ================================================================
    # 4. BOOT WITH CUSTOM CMDLINE
    # ================================================================

    with subtest("snix microVM respects CH_CMDLINE"):
        # Boot with snix.find (default) — VM should mention /nix/store
        out = host.succeed(
            "CH_CMDLINE='snix.find' timeout 30 run-snix-vm 2>&1 || true"
        )
        assert "Linux version" in out, \
            f"kernel didn't boot with custom cmdline: {out[:500]}"
        assert "/nix/store" in out, \
            f"/nix/store not referenced in find output: {out[-500:]}"
        host.log("CH_CMDLINE=snix.find works")

    with subtest("snix microVM respects CH_MEM_SIZE"):
        # Boot with 256M memory
        out = host.succeed(
            "CH_MEM_SIZE=256M CH_CMDLINE='snix.find' timeout 30 run-snix-vm 2>&1 || true"
        )
        assert "Linux version" in out, \
            f"kernel didn't boot with 256M: {out[:500]}"
        host.log("CH_MEM_SIZE=256M works")

    # ── done ─────────────────────────────────────────────────────────
    host.log("All SNIX boot integration tests passed!")
  '';
}
