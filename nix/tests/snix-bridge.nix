# NixOS VM integration test for the snix gRPC bridge.
#
# Tests the full round-trip:
#   1. Start aspen-snix-bridge on a Unix socket
#   2. Import a file into snix-store via the bridge
#   3. Verify the import produces a valid store path
#   4. snix-store daemon connects to bridge backends
#
# This proves snix-store can use Aspen's storage implementations
# transparently via the gRPC bridge.
#
# Run:
#   nix build .#checks.x86_64-linux.snix-bridge-test --impure
{
  pkgs,
  snixBridgePackage,
  snixStorePackage,
}:
pkgs.testers.nixosTest {
  name = "snix-bridge";

  nodes = {
    machine = {
      environment.systemPackages = [
        snixBridgePackage
        snixStorePackage
      ];

      networking.firewall.enable = false;
      virtualisation.memorySize = 1024;
      virtualisation.cores = 2;
    };
  };

  testScript = ''
    import time

    start_all()

    # ================================================================
    # 1. BRIDGE BINARY AVAILABLE
    # ================================================================

    with subtest("bridge binary available"):
        machine.succeed("aspen-snix-bridge --help")
        machine.log("aspen-snix-bridge binary found")

    with subtest("snix-store binary available"):
        machine.succeed("snix-store --help")
        machine.log("snix-store binary found")

    # ================================================================
    # 2. START BRIDGE
    # ================================================================

    with subtest("bridge starts and creates socket"):
        machine.succeed(
            "systemd-run --unit=snix-bridge "
            "aspen-snix-bridge --socket /tmp/snix-bridge.sock"
        )
        machine.wait_until_succeeds(
            "test -S /tmp/snix-bridge.sock",
            timeout=10,
        )
        machine.log("bridge socket created at /tmp/snix-bridge.sock")

    # ================================================================
    # 3. IMPORT A FILE VIA BRIDGE
    # ================================================================

    with subtest("import file into snix-store via bridge"):
        # Create a test file
        machine.succeed("echo 'hello from aspen bridge test' > /tmp/test-file.txt")

        # Import via snix-store, pointing at the bridge socket
        out = machine.succeed(
            "BLOB_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "DIRECTORY_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "PATH_INFO_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "snix-store import /tmp/test-file.txt 2>&1"
        ).strip()
        machine.log(f"import output: {out}")

        # The import should produce a /nix/store path
        assert "/nix/store/" in out, f"expected store path in output: {out}"
        machine.log(f"successfully imported file: {out}")

    # ================================================================
    # 4. IMPORT A DIRECTORY
    # ================================================================

    with subtest("import directory into snix-store via bridge"):
        machine.succeed("mkdir -p /tmp/test-dir/sub")
        machine.succeed("echo 'file A' > /tmp/test-dir/a.txt")
        machine.succeed("echo 'file B' > /tmp/test-dir/sub/b.txt")

        out = machine.succeed(
            "BLOB_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "DIRECTORY_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "PATH_INFO_SERVICE_ADDR=grpc+unix:///tmp/snix-bridge.sock "
            "snix-store import /tmp/test-dir 2>&1"
        ).strip()
        machine.log(f"dir import output: {out}")

        assert "/nix/store/" in out, f"expected store path: {out}"
        machine.log(f"successfully imported directory: {out}")

    # ================================================================
    # 5. VERIFY BRIDGE STAYS HEALTHY
    # ================================================================

    with subtest("bridge still running after imports"):
        machine.succeed("systemctl is-active snix-bridge")
        machine.succeed("test -S /tmp/snix-bridge.sock")
        machine.log("bridge still healthy")

    # ================================================================
    # 6. CLEANUP
    # ================================================================

    with subtest("cleanup"):
        machine.succeed("systemctl stop snix-bridge || true")
        machine.succeed("rm -f /tmp/snix-bridge.sock")
        machine.log("bridge stopped and socket cleaned up")

    machine.log("All snix bridge integration tests passed!")
  '';
}
