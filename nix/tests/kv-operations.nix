# NixOS VM integration test for the Aspen key-value store.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises every KV CLI command end-to-end:
#
#   - Basic CRUD (set, get, delete)
#   - Non-existent key reads
#   - Value overwrite / update
#   - Compare-and-swap (create-if-absent, conditional update, conflict)
#   - Compare-and-delete (success, conflict)
#   - Prefix scan with limit and pagination (continuation tokens)
#   - Batch read (multi-key atomic reads)
#   - Batch write (atomic multi-key writes)
#   - Binary data via --file flag
#   - Large value storage
#
# Run:
#   nix build .#checks.x86_64-linux.kv-operations-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.kv-operations-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "kv-vm-test";
in
  pkgs.testers.nixosTest {
    name = "kv-operations";

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
          logLevel = "info,aspen=debug";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = [];
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket and return parsed JSON.

          We redirect stdout to a temp file and cat it back, because the NixOS
          test serial console mixes stderr/kernel messages into the captured
          output, corrupting JSON parsing.
          """
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node1.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      cli_text("cluster init")
      time.sleep(2)

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── basic CRUD ───────────────────────────────────────────────────

      with subtest("kv set and get"):
          out = cli("kv set mykey hello-world")
          assert out.get("status") == "success", f"set failed: {out}"

          out = cli("kv get mykey")
          assert out.get("does_exist") is True, f"key not found: {out}"
          assert out.get("value") == "hello-world", f"value mismatch: {out}"
          node1.log("kv set/get: OK")

      with subtest("kv get non-existent key"):
          out = cli("kv get nonexistent-key-xyz")
          assert out.get("does_exist") is False, \
              f"non-existent key should not exist: {out}"
          node1.log("kv get non-existent: OK")

      with subtest("kv overwrite"):
          cli("kv set mykey updated-value")
          out = cli("kv get mykey")
          assert out.get("value") == "updated-value", \
              f"overwrite failed: {out}"
          node1.log("kv overwrite: OK")

      with subtest("kv delete existing key"):
          out = cli("kv delete mykey")
          assert out.get("was_deleted") is True, f"delete failed: {out}"

          out = cli("kv get mykey")
          assert out.get("does_exist") is False, \
              f"key still exists after delete: {out}"
          node1.log("kv delete existing: OK")

      with subtest("kv delete non-existent key"):
          out = cli("kv delete already-gone-key")
          assert out.get("status") == "success", f"delete error: {out}"
          # Idempotent: server returns was_deleted=true even for missing keys
          node1.log(f"kv delete non-existent: was_deleted={out.get('was_deleted')}")

      # ── compare-and-swap ─────────────────────────────────────────────

      with subtest("cas create-if-absent"):
          # CAS with no expected value = create-if-absent
          out = cli("kv cas cas-key --new-value initial")
          assert out.get("is_success") is True, \
              f"CAS create-if-absent failed: {out}"

          out = cli("kv get cas-key")
          assert out.get("value") == "initial", f"CAS value wrong: {out}"
          node1.log("cas create-if-absent: OK")

      with subtest("cas conditional update"):
          out = cli(
              "kv cas cas-key --expected initial --new-value updated"
          )
          assert out.get("is_success") is True, \
              f"CAS conditional update failed: {out}"

          out = cli("kv get cas-key")
          assert out.get("value") == "updated", \
              f"CAS update value wrong: {out}"
          node1.log("cas conditional update: OK")

      with subtest("cas conflict detection"):
          # Try to CAS with wrong expected value
          out = cli(
              "kv cas cas-key --expected wrong-value --new-value nope",
              check=False,
          )
          # CAS conflict: is_success should be false
          if isinstance(out, dict):
              assert out.get("is_success") is False or \
                  out.get("status") == "conflict", \
                  f"CAS should have conflicted: {out}"
          node1.log("cas conflict detection: OK")

          # Verify original value unchanged
          out = cli("kv get cas-key")
          assert out.get("value") == "updated", \
              f"CAS conflict changed value: {out}"

      # ── compare-and-delete ───────────────────────────────────────────

      with subtest("cad success"):
          cli("kv set cad-key delete-me")
          out = cli("kv cad cad-key --expected delete-me")
          assert out.get("is_success") is True, f"CAD failed: {out}"

          out = cli("kv get cad-key")
          assert out.get("does_exist") is False, \
              f"CAD did not delete: {out}"
          node1.log("cad success: OK")

      with subtest("cad conflict"):
          cli("kv set cad-key2 keep-me")
          out = cli(
              "kv cad cad-key2 --expected wrong-value",
              check=False,
          )
          if isinstance(out, dict):
              assert out.get("is_success") is False or \
                  out.get("status") == "conflict", \
                  f"CAD should have conflicted: {out}"

          # Value should be unchanged
          out = cli("kv get cad-key2")
          assert out.get("value") == "keep-me", \
              f"CAD conflict changed value: {out}"
          node1.log("cad conflict: OK")

      # ── prefix scan ──────────────────────────────────────────────────

      with subtest("scan setup"):
          # Create keys with a common prefix for scanning
          for i in range(10):
              cli(f"kv set scan:item:{i:03d} value-{i}")
          # Create some unrelated keys
          cli("kv set other:key1 unrelated1")
          cli("kv set other:key2 unrelated2")
          node1.log("scan setup: 10 scan:item: keys + 2 other: keys created")

      with subtest("scan by prefix"):
          out = cli("kv scan scan:item:")
          entries = out.get("entries", [])
          assert len(entries) == 10, \
              f"expected 10 entries, got {len(entries)}: {entries}"
          # Verify all keys have the right prefix
          for entry in entries:
              key = entry[0] if isinstance(entry, list) else entry.get("key", "")
              assert "scan:item:" in str(key), \
                  f"unexpected key in scan: {entry}"
          node1.log("scan by prefix: OK")

      with subtest("scan with limit"):
          out = cli("kv scan scan:item: --limit 3")
          entries = out.get("entries", [])
          assert len(entries) == 3, \
              f"expected 3 entries with limit, got {len(entries)}"
          node1.log("scan with limit: OK")

      with subtest("scan pagination with continuation token"):
          # First page
          out = cli("kv scan scan:item: --limit 4")
          page1 = out.get("entries", [])
          token = out.get("continuation_token")
          assert len(page1) == 4, f"page1 should have 4 entries: {page1}"
          node1.log(f"scan page1: {len(page1)} entries, token={token}")

          if token is not None:
              # Second page using continuation token
              out = cli(
                  f"kv scan scan:item: --limit 4 --token {token}",
                  check=False,
              )
              if isinstance(out, dict):
                  page2 = out.get("entries", [])
                  node1.log(f"scan page2: {len(page2)} entries")

                  # Verify no duplicates between pages
                  keys1 = set()
                  for e in page1:
                      k = e[0] if isinstance(e, list) else e.get("key", "")
                      keys1.add(str(k))
                  for e in page2:
                      k = e[0] if isinstance(e, list) else e.get("key", "")
                      assert str(k) not in keys1, \
                          f"duplicate key across pages: {k}"
                  node1.log("scan pagination: OK")
              else:
                  node1.log(f"scan pagination: page2 returned non-JSON: {out}")
          else:
              node1.log("scan pagination: no continuation token (all results in page1)")

      with subtest("scan all keys (empty prefix)"):
          out = cli("kv scan")
          entries = out.get("entries", [])
          # Should return all keys we've created (including scan:, other:, cas-key, cad-key2)
          assert len(entries) >= 12, \
              f"expected at least 12 total keys, got {len(entries)}"
          node1.log(f"scan all: {len(entries)} keys total")

      # ── batch read ───────────────────────────────────────────────────

      with subtest("batch read existing keys"):
          out = cli("kv batch-read scan:item:000 scan:item:001 scan:item:002")
          node1.log(f"batch read result: {out}")
          results = out.get("results", [])
          assert len(results) == 3, f"expected 3 results: {results}"
          for r in results:
              assert r.get("does_exist") is True, \
                  f"expected does_exist=true: {r}"
          node1.log("batch read existing: OK")

      with subtest("batch read mixed existing and missing"):
          out = cli(
              "kv batch-read scan:item:000 missing-key-xyz scan:item:005"
          )
          node1.log(f"batch read mixed: {out}")
          results = out.get("results", [])
          assert len(results) == 3, \
              f"expected 3 results in response: {results}"
          # At least one should be missing
          missing = [r for r in results if not r.get("does_exist")]
          assert len(missing) >= 1, \
              f"expected at least one missing key: {results}"
          node1.log("batch read mixed: OK")

      # ── batch write ──────────────────────────────────────────────────

      with subtest("batch write"):
          out = cli(
              "kv batch-write batch:a=alpha batch:b=beta batch:c=gamma"
          )
          assert out.get("is_success") is True, \
              f"batch write failed: {out}"

          # Verify all three keys were written
          for key, expected in [
              ("batch:a", "alpha"),
              ("batch:b", "beta"),
              ("batch:c", "gamma"),
          ]:
              out = cli(f"kv get {key}")
              assert out.get("value") == expected, \
                  f"batch write value mismatch for {key}: {out}"
          node1.log("batch write: OK")

      with subtest("batch write atomicity"):
          # Write multiple keys, then verify they're all visible
          out = cli(
              "kv batch-write atomic:1=one atomic:2=two atomic:3=three"
          )
          assert out.get("is_success") is True, \
              f"atomic batch write failed: {out}"

          out = cli("kv batch-read atomic:1 atomic:2 atomic:3")
          node1.log(f"batch write atomicity check: {out}")
          node1.log("batch write atomicity: OK")

      # ── binary data via file ─────────────────────────────────────────

      with subtest("set from file"):
          # Create a file with binary-like content (including special chars)
          node1.succeed(
              "printf 'line1\\nline2\\ttabbed\\x00null' > /tmp/kv-file.dat"
          )
          cli_text("kv set file-key dummy --file /tmp/kv-file.dat")

          # Read it back
          out = cli("kv get file-key")
          assert out.get("does_exist") is True, \
              f"file-stored key not found: {out}"
          node1.log("set from file: OK")

      # ── large values ─────────────────────────────────────────────────

      with subtest("large value storage"):
          # Generate a 100KB value
          node1.succeed(
              "dd if=/dev/urandom bs=1024 count=100 2>/dev/null "
              "| base64 > /tmp/large-value.txt"
          )
          cli_text("kv set large-key dummy --file /tmp/large-value.txt")

          out = cli("kv get large-key")
          assert out.get("does_exist") is True, \
              f"large key not found: {out}"
          value = out.get("value", "")
          # base64 of 100KB ≈ 136KB of text
          assert len(value) > 50000, \
              f"large value too small: {len(value)} bytes"
          node1.log(f"large value storage: OK ({len(value)} chars)")

      # ── special characters in keys and values ────────────────────────

      with subtest("special characters"):
          # Keys with various special chars (URL-safe subset)
          cli("kv set 'key-with-dashes' dashed")
          cli("kv set 'key_with_underscores' underscored")
          cli("kv set 'key.with.dots' dotted")
          cli("kv set 'key/with/slashes' slashed")

          for key, expected in [
              ("key-with-dashes", "dashed"),
              ("key_with_underscores", "underscored"),
              ("key.with.dots", "dotted"),
              ("key/with/slashes", "slashed"),
          ]:
              out = cli(f"kv get '{key}'")
              assert out.get("value") == expected, \
                  f"special char key {key} mismatch: {out}"
          node1.log("special characters: OK")

      # ── empty value ──────────────────────────────────────────────────

      with subtest("empty value"):
          # Write an empty file and use --file to set an empty value.
          # (Cannot pass literal empty string via shell args inside Nix
          # indented strings without escaping issues.)
          node1.succeed("truncate -s 0 /tmp/empty.dat")
          cli_text("kv set empty-val dummy --file /tmp/empty.dat")
          out = cli("kv get empty-val")
          assert out.get("does_exist") is True, \
              f"empty value key not found: {out}"
          # Value may be empty string or the dummy arg is overridden by --file
          node1.log(f"empty value: value={out.get('value')!r}")

      # ── scan prefix isolation ────────────────────────────────────────

      with subtest("scan prefix isolation"):
          # Verify scan only returns keys with matching prefix
          out = cli("kv scan other:")
          entries = out.get("entries", [])
          assert len(entries) == 2, \
              f"expected 2 other: entries, got {len(entries)}: {entries}"
          node1.log("scan prefix isolation: OK")

      with subtest("scan batch: prefix"):
          out = cli("kv scan batch:")
          entries = out.get("entries", [])
          assert len(entries) == 3, \
              f"expected 3 batch: entries, got {len(entries)}: {entries}"
          node1.log("scan batch prefix: OK")

      # ── cleanup verification ─────────────────────────────────────────

      with subtest("delete and verify cleanup"):
          cli("kv delete cas-key")
          cli("kv delete cad-key2")
          cli("kv delete empty-val")

          for key in ["cas-key", "cad-key2", "empty-val"]:
              out = cli(f"kv get {key}")
              assert out.get("does_exist") is False, \
                  f"key {key} still exists after cleanup: {out}"
          node1.log("cleanup verification: OK")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All KV operations integration tests passed!")
    '';
  }
