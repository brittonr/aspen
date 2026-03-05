# NixOS VM integration test for FUSE mount operations.
#
# Spins up a single-node Aspen cluster, mounts aspen-fuse at /mnt/aspen,
# then exercises basic POSIX filesystem operations:
#
#   - Write and read files
#   - Create directories (virtual, from key prefixes)
#   - Delete files
#   - Write and read large files (chunked I/O)
#   - Verify data via both FUSE and aspen-cli KV scan
#
# Run:
#   nix build .#checks.x86_64-linux.fuse-operations-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.fuse-operations-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspen-fuse-vm-test,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "fuse-vm-test";
in
  pkgs.testers.nixosTest {
    name = "fuse-operations";
    skipLint = true;

    nodes = {
      node1 = {
        imports = [../../nix/modules/aspen-node.nix];

        services.aspen.node = {
          enable = true;
          package = aspenNodePackage;
          nodeId = 1;
          inherit cookie;
          secretKey = secretKey;
          storageBackend = "redb";
          dataDir = "/var/lib/aspen";
          logLevel = "info";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [
          aspenCliPackage
          aspen-fuse-vm-test
          pkgs.fuse3
        ];

        # FUSE requires /dev/fuse
        boot.kernelModules = ["fuse"];
        # Allow user_allow_other in fuse config
        environment.etc."fuse.conf".text = "user_allow_other";

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
      };
    };

    testScript = ''
      import time

      CLI = "${aspenCliPackage}/bin/aspen-cli"
      FUSE = "${aspen-fuse-vm-test}/bin/aspen-fuse"

      node1.start()
      node1.wait_for_unit("aspen-node.service")

      # Wait for cluster to be ready
      node1.wait_until_succeeds(
          f"{CLI} --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster status",
          timeout=60,
      )

      ticket = node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      # =====================================================================
      # Initialize cluster
      # =====================================================================
      node1.succeed(f"{CLI} --ticket {ticket} cluster init")
      node1.wait_until_succeeds(
          f"{CLI} --ticket {ticket} kv set __fuse_health_check test_value",
          timeout=30,
      )

      # =====================================================================
      # Mount FUSE filesystem
      # =====================================================================
      node1.succeed("mkdir -p /mnt/aspen")

      # Start aspen-fuse in the background via systemd-run
      node1.succeed(
          f"systemd-run --unit=aspen-fuse --property=Type=exec "
          f"bash -c '{FUSE} --mount-point /mnt/aspen --ticket {ticket} --foreground --allow-other 2>/tmp/fuse.log'"
      )

      # Wait for FUSE mount to be ready
      node1.wait_until_succeeds("mountpoint /mnt/aspen", timeout=30)

      # =====================================================================
      # Test 1: Write and read a file
      # =====================================================================
      node1.succeed("echo -n 'hello world' > /mnt/aspen/test.txt")
      result = node1.succeed("cat /mnt/aspen/test.txt")
      assert "hello world" in result, f"Expected 'hello world', got: {result}"

      # =====================================================================
      # Test 2: Directory listing
      # =====================================================================
      result = node1.succeed("ls /mnt/aspen/")
      assert "test.txt" in result, f"Expected test.txt in listing: {result}"

      # =====================================================================
      # Test 3: Create nested path via write
      # =====================================================================
      node1.succeed("mkdir -p /mnt/aspen/subdir")
      node1.succeed("echo -n 'nested content' > /mnt/aspen/subdir/file.txt")
      result = node1.succeed("cat /mnt/aspen/subdir/file.txt")
      assert "nested content" in result, f"Expected 'nested content', got: {result}"

      # Verify via KV scan
      kv_result = node1.succeed(f"{CLI} --ticket {ticket} kv scan subdir/")
      assert "subdir/" in kv_result or "file.txt" in kv_result, f"KV scan should find nested key: {kv_result}"

      # =====================================================================
      # Test 4: Delete a file
      # =====================================================================
      node1.succeed("rm /mnt/aspen/test.txt")
      # File should be gone
      node1.succeed("test ! -f /mnt/aspen/test.txt")

      # =====================================================================
      # Test 5: Large file (100KB) — tests chunked I/O
      # =====================================================================
      node1.succeed("dd if=/dev/urandom bs=1024 count=100 of=/tmp/large_input.bin 2>/dev/null")
      node1.succeed("cp /tmp/large_input.bin /mnt/aspen/large.bin")

      # Read back and compare
      node1.succeed("cp /mnt/aspen/large.bin /tmp/large_output.bin")
      input_size = node1.succeed("stat -c%s /tmp/large_input.bin").strip()
      output_size = node1.succeed("stat -c%s /tmp/large_output.bin").strip()
      assert input_size == output_size, f"Size mismatch: input={input_size}, output={output_size}"

      input_hash = node1.succeed("sha256sum /tmp/large_input.bin | cut -d' ' -f1").strip()
      output_hash = node1.succeed("sha256sum /tmp/large_output.bin | cut -d' ' -f1").strip()
      assert input_hash == output_hash, f"Content mismatch: {input_hash} != {output_hash}"

      # =====================================================================
      # Cleanup: unmount FUSE
      # =====================================================================
      node1.succeed("fusermount3 -u /mnt/aspen || umount /mnt/aspen || true")
      node1.succeed("systemctl stop aspen-fuse || true")
    '';
  }
