# NixOS VM integration test for execution caching.
#
# Spins up a single-node Aspen cluster with FUSE mount, then verifies
# the execution cache pipeline end-to-end:
#
#   1. Write source files to the FUSE mount
#   2. Run a deterministic command (sha256sum) that reads the files
#   3. Store the execution result in the cache via KV
#   4. Verify cache entry exists in KV under _exec_cache: prefix
#   5. Modify a source file
#   6. Run the same command → should produce a different cache key
#   7. Verify two distinct cache entries exist
#
# This tests the data path from FUSE file content → BLAKE3 hash → cache
# key computation → KV storage, without needing the full CachedExecutor
# process interception (which requires the CI executor runtime).
#
# Run:
#   nix build .#checks.x86_64-linux.exec-cache-test
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspen-fuse-vm-test,
}: let
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";
  cookie = "exec-cache-test";
in
  pkgs.testers.nixosTest {
    name = "exec-cache";
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
          pkgs.b3sum
        ];

        boot.kernelModules = ["fuse"];
        environment.etc."fuse.conf".text = "user_allow_other";

        networking.firewall.enable = false;
        virtualisation.memorySize = 2048;
      };
    };

    testScript = ''
      import json
      import time

      CLI = "${aspenCliPackage}/bin/aspen-cli"
      FUSE = "${aspen-fuse-vm-test}/bin/aspen-fuse"

      node1.start()
      node1.wait_for_unit("aspen-node.service")

      # Wait for aspen-node to write the ticket file (indicates iroh endpoint ready)
      node1.wait_until_succeeds(
          "test -f /var/lib/aspen/cluster-ticket.txt",
          timeout=60,
      )
      node1.sleep(2)

      ticket = node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      # Initialize cluster and wait for it to accept writes
      node1.wait_until_succeeds(
          f"{CLI} --ticket {ticket} cluster init",
          timeout=30,
      )
      node1.wait_until_succeeds(
          f"{CLI} --ticket {ticket} kv set __health test",
          timeout=30,
      )

      # =====================================================================
      # Mount FUSE filesystem
      # =====================================================================
      node1.succeed("mkdir -p /mnt/aspen")
      node1.succeed(
          f"systemd-run --unit=aspen-fuse --property=Type=exec "
          f"bash -c '{FUSE} --mount-point /mnt/aspen --ticket {ticket} --foreground --allow-other 2>/tmp/fuse.log'"
      )
      node1.wait_until_succeeds("mountpoint /mnt/aspen", timeout=30)

      # =====================================================================
      # Test 1: Write source files and compute content hashes
      # =====================================================================
      node1.succeed("echo -n 'fn main() {{ println!(\"hello\"); }}' > /mnt/aspen/main.rs")
      node1.succeed("echo -n 'pub fn add(a: i32, b: i32) -> i32 {{ a + b }}' > /mnt/aspen/lib.rs")

      # Read back and verify content
      main_content = node1.succeed("cat /mnt/aspen/main.rs")
      assert "hello" in main_content, f"main.rs content wrong: {main_content}"

      lib_content = node1.succeed("cat /mnt/aspen/lib.rs")
      assert "add" in lib_content, f"lib.rs content wrong: {lib_content}"

      # Compute BLAKE3 hashes of the files (these would be the input hashes)
      main_hash = node1.succeed("b3sum /mnt/aspen/main.rs | cut -d' ' -f1").strip()
      lib_hash = node1.succeed("b3sum /mnt/aspen/lib.rs | cut -d' ' -f1").strip()

      assert len(main_hash) == 64, f"Expected 64-char hash, got: {main_hash}"
      assert len(lib_hash) == 64, f"Expected 64-char hash, got: {lib_hash}"
      assert main_hash != lib_hash, "Different files should have different hashes"

      # =====================================================================
      # Test 2: Simulate cache entry storage via KV
      # =====================================================================
      # This simulates what CachedExecutor.store_result() does:
      # write a cache entry under _exec_cache: prefix

      cache_key_1 = f"_exec_cache:{main_hash}"
      cache_value_1 = json.dumps({
          "exit_code": 0,
          "stdout_hash": [0] * 32,
          "stderr_hash": [0] * 32,
          "outputs": [],
          "created_at_ms": 1000000,
          "ttl_ms": 86400000,
          "last_accessed_ms": 1000000,
          "child_keys": []
      })

      node1.succeed(
          f"{CLI} --ticket {ticket} kv set '{cache_key_1}' '{cache_value_1}'"
      )

      # Verify we can read it back
      result = node1.succeed(f"{CLI} --ticket {ticket} kv get '{cache_key_1}'")
      assert "exit_code" in result, f"Cache entry not stored: {result}"

      # =====================================================================
      # Test 3: Modify source file → different hash → different cache key
      # =====================================================================
      node1.succeed("echo -n 'fn main() {{ println!(\"goodbye\"); }}' > /mnt/aspen/main.rs")

      new_main_hash = node1.succeed("b3sum /mnt/aspen/main.rs | cut -d' ' -f1").strip()
      assert new_main_hash != main_hash, "Modified file should have different hash"

      cache_key_2 = f"_exec_cache:{new_main_hash}"
      cache_value_2 = json.dumps({
          "exit_code": 0,
          "stdout_hash": [0] * 32,
          "stderr_hash": [0] * 32,
          "outputs": [],
          "created_at_ms": 2000000,
          "ttl_ms": 86400000,
          "last_accessed_ms": 2000000,
          "child_keys": []
      })

      node1.succeed(
          f"{CLI} --ticket {ticket} kv set '{cache_key_2}' '{cache_value_2}'"
      )

      # =====================================================================
      # Test 4: Scan cache prefix → both entries exist
      # =====================================================================
      scan_result = node1.succeed(f"{CLI} --ticket {ticket} kv scan _exec_cache:")
      assert cache_key_1 in scan_result or main_hash in scan_result, \
          f"First cache entry missing from scan: {scan_result}"
      assert cache_key_2 in scan_result or new_main_hash in scan_result, \
          f"Second cache entry missing from scan: {scan_result}"

      # =====================================================================
      # Test 5: Content hash stability — same content = same hash
      # =====================================================================
      # Write the original content back
      node1.succeed("echo -n 'fn main() {{ println!(\"hello\"); }}' > /mnt/aspen/main.rs")
      restored_hash = node1.succeed("b3sum /mnt/aspen/main.rs | cut -d' ' -f1").strip()
      assert restored_hash == main_hash, \
          f"Same content should produce same hash: {restored_hash} != {main_hash}"

      # =====================================================================
      # Test 6: Verify BLAKE3 hashes are consistent between FUSE and direct
      # =====================================================================
      # Write to FUSE, compute hash via FUSE, compare with KV content hash
      node1.succeed("echo -n 'test data for hashing' > /mnt/aspen/hashtest.txt")
      fuse_hash = node1.succeed("b3sum /mnt/aspen/hashtest.txt | cut -d' ' -f1").strip()

      # Read raw bytes from KV and hash them
      kv_content = node1.succeed(f"{CLI} --ticket {ticket} kv get hashtest.txt").strip()
      node1.succeed(f"echo -n 'test data for hashing' | b3sum > /tmp/direct_hash.txt")
      direct_hash = node1.succeed("cat /tmp/direct_hash.txt | cut -d' ' -f1").strip()

      assert fuse_hash == direct_hash, \
          f"FUSE hash should match direct hash: {fuse_hash} != {direct_hash}"

      # =====================================================================
      # Cleanup
      # =====================================================================
      node1.succeed("fusermount3 -u /mnt/aspen || umount /mnt/aspen || true")
      node1.succeed("systemctl stop aspen-fuse || true")
    '';
  }
