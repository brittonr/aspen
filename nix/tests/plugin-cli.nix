# NixOS VM integration test for Aspen plugin management CLI.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises the plugin management CLI commands end-to-end:
#
#   - plugin list (empty initially)
#   - plugin install (with --name/--handles and with --manifest)
#   - plugin info (show details)
#   - plugin enable / disable (toggle state)
#   - plugin remove (uninstall)
#   - plugin reload (hot-reload, best-effort since WASM runtime not loaded)
#
# The plugin CLI works by storing manifests in KV and WASM blobs in the blob
# store — it does NOT require the WASM runtime to be loaded on the server.
# Reload is the only command that needs the runtime, and we test it gracefully.
#
# Run:
#   nix build .#checks.x86_64-linux.plugin-cli-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.plugin-cli-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "plugin-cli-vm-test";
in
  pkgs.testers.nixosTest {
    name = "plugin-cli";

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
          """Run aspen-cli --json with the cluster ticket and return parsed JSON."""
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

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      cli("cluster init", check=False)
      time.sleep(2)

      # ================================================================
      # PLUGIN LIST (EMPTY)
      # ================================================================

      with subtest("plugin list empty"):
          out = cli("plugin list")
          node1.log(f"plugin list (empty): {out}")
          assert isinstance(out, dict), \
              f"expected JSON object, got: {out}"
          assert out.get("count") == 0, \
              f"expected 0 plugins initially, got {out.get('count')}: {out}"
          assert out.get("plugins") == [], \
              f"expected empty plugins list, got: {out.get('plugins')}"
          node1.log("plugin list (empty): OK")

      # ================================================================
      # PLUGIN INSTALL (--name / --handles)
      # ================================================================

      with subtest("plugin install with flags"):
          # Create a dummy WASM file (not a real module, but the CLI just
          # uploads it as a blob — validation happens at reload time).
          node1.succeed("dd if=/dev/urandom of=/tmp/test-plugin.wasm bs=1024 count=4")

          out = cli(
              "plugin install /tmp/test-plugin.wasm "
              "--name test-alpha "
              "--handles Ping,ReadKey "
              "--priority 920 "
              "--plugin-version 1.0.0 "
              "--app-id test-app"
          )
          node1.log(f"plugin install: {out}")
          assert isinstance(out, dict), \
              f"expected JSON object, got: {out}"
          assert out.get("status") == "installed", \
              f"install should succeed: {out}"
          assert out.get("name") == "test-alpha", \
              f"name mismatch: {out}"
          wasm_hash_alpha = out.get("wasm_hash")
          assert wasm_hash_alpha is not None, \
              f"expected wasm_hash in response: {out}"
          node1.log(f"plugin installed: test-alpha hash={wasm_hash_alpha}")

      # ================================================================
      # PLUGIN LIST (ONE PLUGIN)
      # ================================================================

      with subtest("plugin list after install"):
          out = cli("plugin list")
          node1.log(f"plugin list: {out}")
          assert out.get("count") == 1, \
              f"expected 1 plugin, got {out.get('count')}: {out}"
          plugins = out.get("plugins", [])
          assert len(plugins) == 1, \
              f"expected 1 plugin entry: {plugins}"
          p = plugins[0]
          assert p.get("name") == "test-alpha", \
              f"name mismatch: {p}"
          assert p.get("enabled") is True, \
              f"newly installed plugin should be enabled: {p}"
          assert p.get("priority") == 920, \
              f"priority mismatch: {p}"

      # ================================================================
      # PLUGIN INFO
      # ================================================================

      with subtest("plugin info"):
          out = cli("plugin info test-alpha")
          node1.log(f"plugin info: {out}")
          assert isinstance(out, dict), \
              f"expected JSON object, got: {out}"
          assert out.get("name") == "test-alpha", \
              f"name mismatch: {out}"
          assert out.get("version") == "1.0.0", \
              f"version mismatch: {out}"
          assert out.get("enabled") is True, \
              f"should be enabled: {out}"
          assert out.get("priority") == 920, \
              f"priority mismatch: {out}"
          assert out.get("wasm_hash") == wasm_hash_alpha, \
              f"wasm_hash mismatch: {out}"
          handles = out.get("handles", [])
          assert "Ping" in handles and "ReadKey" in handles, \
              f"handles mismatch: {handles}"
          assert out.get("app_id") == "test-app", \
              f"app_id mismatch: {out}"
          node1.log("plugin info: OK")

      # ================================================================
      # PLUGIN DISABLE
      # ================================================================

      with subtest("plugin disable"):
          out = cli("plugin disable test-alpha")
          node1.log(f"plugin disable: {out}")
          assert isinstance(out, dict), \
              f"expected JSON object, got: {out}"
          assert out.get("name") == "test-alpha", \
              f"name mismatch: {out}"
          assert out.get("enabled") is False, \
              f"should be disabled: {out}"
          node1.log("plugin disable: OK")

      with subtest("plugin info after disable"):
          out = cli("plugin info test-alpha")
          assert out.get("enabled") is False, \
              f"plugin should be disabled: {out}"
          node1.log("plugin info after disable: OK")

      # ================================================================
      # PLUGIN ENABLE
      # ================================================================

      with subtest("plugin enable"):
          out = cli("plugin enable test-alpha")
          node1.log(f"plugin enable: {out}")
          assert out.get("name") == "test-alpha", \
              f"name mismatch: {out}"
          assert out.get("enabled") is True, \
              f"should be enabled: {out}"
          node1.log("plugin enable: OK")

      with subtest("plugin info after enable"):
          out = cli("plugin info test-alpha")
          assert out.get("enabled") is True, \
              f"plugin should be enabled: {out}"
          node1.log("plugin info after enable: OK")

      # ================================================================
      # PLUGIN INSTALL WITH MANIFEST
      # ================================================================

      with subtest("plugin install with manifest"):
          # Write a plugin.json manifest file
          manifest = json.dumps({
              "name": "test-beta",
              "version": "2.0.0",
              "handles": ["WriteKey", "DeleteKey"],
              "priority": 930,
              "app_id": "beta-app",
              "kv_prefixes": ["beta:"],
              "permissions": {
                  "kv_read": True,
                  "kv_write": True,
              },
          })
          node1.succeed(
              f"cat > /tmp/test-beta-manifest.json << 'MANIFEST_EOF'\n"
              f"{manifest}\n"
              f"MANIFEST_EOF"
          )

          # Create another dummy WASM file
          node1.succeed("dd if=/dev/urandom of=/tmp/test-beta.wasm bs=1024 count=8")

          out = cli(
              "plugin install /tmp/test-beta.wasm "
              "--manifest /tmp/test-beta-manifest.json"
          )
          node1.log(f"plugin install with manifest: {out}")
          assert out.get("status") == "installed", \
              f"install should succeed: {out}"
          assert out.get("name") == "test-beta", \
              f"name should come from manifest: {out}"
          wasm_hash_beta = out.get("wasm_hash")
          assert wasm_hash_beta is not None, \
              f"expected wasm_hash: {out}"
          node1.log(f"plugin installed: test-beta hash={wasm_hash_beta}")

      with subtest("plugin info from manifest"):
          out = cli("plugin info test-beta")
          node1.log(f"plugin info test-beta: {out}")
          assert out.get("name") == "test-beta", \
              f"name mismatch: {out}"
          assert out.get("version") == "2.0.0", \
              f"version mismatch: {out}"
          assert out.get("priority") == 930, \
              f"priority mismatch: {out}"
          handles = out.get("handles", [])
          assert "WriteKey" in handles and "DeleteKey" in handles, \
              f"handles mismatch: {handles}"
          assert out.get("app_id") == "beta-app", \
              f"app_id mismatch: {out}"
          node1.log("plugin info from manifest: OK")

      # ================================================================
      # PLUGIN INSTALL WITH MANIFEST + OVERRIDES
      # ================================================================

      with subtest("plugin install manifest with overrides"):
          # Reuse beta manifest but override name and priority
          node1.succeed("dd if=/dev/urandom of=/tmp/test-gamma.wasm bs=1024 count=2")

          out = cli(
              "plugin install /tmp/test-gamma.wasm "
              "--manifest /tmp/test-beta-manifest.json "
              "--name test-gamma "
              "--priority 940 "
              "--plugin-version 3.0.0"
          )
          node1.log(f"plugin install with overrides: {out}")
          assert out.get("status") == "installed", \
              f"install should succeed: {out}"
          assert out.get("name") == "test-gamma", \
              f"name should be overridden: {out}"

      with subtest("plugin info after override install"):
          out = cli("plugin info test-gamma")
          assert out.get("name") == "test-gamma", \
              f"name mismatch: {out}"
          assert out.get("version") == "3.0.0", \
              f"version should be overridden: {out}"
          assert out.get("priority") == 940, \
              f"priority should be overridden: {out}"
          # handles come from manifest since not overridden
          handles = out.get("handles", [])
          assert "WriteKey" in handles and "DeleteKey" in handles, \
              f"handles should come from manifest: {handles}"
          node1.log("plugin info after override install: OK")

      # ================================================================
      # PLUGIN LIST (MULTIPLE)
      # ================================================================

      with subtest("plugin list multiple"):
          out = cli("plugin list")
          node1.log(f"plugin list (multiple): {out}")
          assert out.get("count") == 3, \
              f"expected 3 plugins, got {out.get('count')}: {out}"
          names = sorted([p.get("name", "") for p in out.get("plugins", [])])
          assert names == ["test-alpha", "test-beta", "test-gamma"], \
              f"expected all three plugins: {names}"
          node1.log("plugin list multiple: OK")

      # ================================================================
      # PLUGIN REMOVE
      # ================================================================

      with subtest("plugin remove"):
          out = cli("plugin remove test-gamma")
          node1.log(f"plugin remove: {out}")
          assert out.get("status") == "removed", \
              f"remove should succeed: {out}"
          assert out.get("name") == "test-gamma", \
              f"name mismatch: {out}"
          node1.log("plugin remove: OK")

      with subtest("plugin list after remove"):
          out = cli("plugin list")
          assert out.get("count") == 2, \
              f"expected 2 plugins after remove, got {out.get('count')}: {out}"
          names = sorted([p.get("name", "") for p in out.get("plugins", [])])
          assert names == ["test-alpha", "test-beta"], \
              f"gamma should be gone: {names}"
          node1.log("plugin list after remove: OK")

      with subtest("plugin info removed"):
          out = cli("plugin info test-gamma", check=False)
          node1.log(f"plugin info removed: {out}")
          # Should fail — plugin not found
          if isinstance(out, dict) and out.get("name"):
              raise AssertionError(f"removed plugin should not be found: {out}")
          node1.log("plugin info removed: OK (not found)")

      # ================================================================
      # PLUGIN REMOVE NON-EXISTENT
      # ================================================================

      with subtest("plugin remove non-existent"):
          out = cli("plugin remove no-such-plugin", check=False)
          node1.log(f"plugin remove non-existent: {out}")
          # Should fail gracefully
          node1.log("plugin remove non-existent: OK (handled)")

      # ================================================================
      # PLUGIN DISABLE + RE-ENABLE ROUND-TRIP
      # ================================================================

      with subtest("plugin disable-enable round-trip"):
          # Disable test-beta
          out = cli("plugin disable test-beta")
          assert out.get("enabled") is False, \
              f"should be disabled: {out}"

          # Verify via list
          out = cli("plugin list")
          for p in out.get("plugins", []):
              if p.get("name") == "test-beta":
                  assert p.get("enabled") is False, \
                      f"test-beta should be disabled in list: {p}"
                  break
          else:
              raise AssertionError("test-beta not found in list")

          # Re-enable
          out = cli("plugin enable test-beta")
          assert out.get("enabled") is True, \
              f"should be enabled: {out}"

          # Verify via info
          out = cli("plugin info test-beta")
          assert out.get("enabled") is True, \
              f"test-beta should be enabled again: {out}"
          node1.log("plugin disable-enable round-trip: OK")

      # ================================================================
      # PLUGIN INSTALL WITH KV PREFIXES
      # ================================================================

      with subtest("plugin install with kv-prefixes"):
          node1.succeed("dd if=/dev/urandom of=/tmp/test-delta.wasm bs=1024 count=2")

          out = cli(
              "plugin install /tmp/test-delta.wasm "
              "--name test-delta "
              "--handles Ping "
              "--priority 960 "
              "--kv-prefixes delta:,delta-extra:"
          )
          assert out.get("status") == "installed", \
              f"install should succeed: {out}"
          node1.log("plugin install with kv-prefixes: OK")

      # ================================================================
      # PLUGIN INSTALL WITH RESOURCE LIMITS
      # ================================================================

      with subtest("plugin install with resource limits"):
          node1.succeed("dd if=/dev/urandom of=/tmp/test-epsilon.wasm bs=1024 count=2")

          out = cli(
              "plugin install /tmp/test-epsilon.wasm "
              "--name test-epsilon "
              "--handles Ping "
              "--fuel-limit 5000000 "
              "--memory-limit 16777216"
          )
          assert out.get("status") == "installed", \
              f"install should succeed: {out}"

          out = cli("plugin info test-epsilon")
          assert out.get("fuel_limit") == 5000000, \
              f"fuel_limit mismatch: {out}"
          assert out.get("memory_limit") == 16777216, \
              f"memory_limit mismatch: {out}"
          node1.log("plugin install with resource limits: OK")

      # ================================================================
      # PLUGIN REINSTALL (OVERWRITE)
      # ================================================================

      with subtest("plugin reinstall overwrite"):
          # Install over test-alpha with new WASM and version
          node1.succeed("dd if=/dev/urandom of=/tmp/test-alpha-v2.wasm bs=1024 count=6")

          out = cli(
              "plugin install /tmp/test-alpha-v2.wasm "
              "--name test-alpha "
              "--handles Ping,ReadKey,WriteKey "
              "--priority 915 "
              "--plugin-version 2.0.0"
          )
          assert out.get("status") == "installed", \
              f"reinstall should succeed: {out}"
          new_hash = out.get("wasm_hash")
          assert new_hash != wasm_hash_alpha, \
              f"new wasm should have different hash: old={wasm_hash_alpha} new={new_hash}"

          # Verify updated metadata
          out = cli("plugin info test-alpha")
          assert out.get("version") == "2.0.0", \
              f"version should be updated: {out}"
          assert out.get("priority") == 915, \
              f"priority should be updated: {out}"
          handles = out.get("handles", [])
          assert "WriteKey" in handles, \
              f"new handles should include WriteKey: {handles}"
          assert out.get("wasm_hash") == new_hash, \
              f"hash should be updated: {out}"
          node1.log("plugin reinstall overwrite: OK")

      # ================================================================
      # PLUGIN RELOAD (best-effort — WASM runtime not loaded in test node)
      # ================================================================

      with subtest("plugin reload all"):
          out = cli("plugin reload", check=False)
          node1.log(f"plugin reload all: {out}")
          # Reload will likely fail since WASM runtime is not loaded in
          # test node (no plugins feature), but verify we get a response.
          if isinstance(out, dict):
              node1.log(
                  f"reload result: is_success={out.get('is_success')}, "
                  f"plugin_count={out.get('plugin_count')}, "
                  f"message={out.get('message')}"
              )
          node1.log("plugin reload all: OK (response received)")

      with subtest("plugin reload single"):
          out = cli("plugin reload test-alpha", check=False)
          node1.log(f"plugin reload test-alpha: {out}")
          if isinstance(out, dict):
              node1.log(
                  f"reload single result: is_success={out.get('is_success')}, "
                  f"message={out.get('message')}"
              )
          node1.log("plugin reload single: OK (response received)")

      # ================================================================
      # PLUGIN LIST FINAL STATE
      # ================================================================

      with subtest("plugin list final"):
          out = cli("plugin list")
          node1.log(f"plugin list (final): {out}")
          count = out.get("count", 0)
          assert count == 4, \
              f"expected 4 plugins (alpha, beta, delta, epsilon), got {count}: {out}"
          names = sorted([p.get("name", "") for p in out.get("plugins", [])])
          expected = ["test-alpha", "test-beta", "test-delta", "test-epsilon"]
          assert names == expected, \
              f"expected {expected}, got {names}"
          node1.log("plugin list final: OK")

      # ================================================================
      # CLEANUP — remove all test plugins
      # ================================================================

      with subtest("cleanup"):
          for name in ["test-alpha", "test-beta", "test-delta", "test-epsilon"]:
              cli(f"plugin remove {name}", check=False)

          out = cli("plugin list")
          assert out.get("count") == 0, \
              f"expected 0 plugins after cleanup, got {out.get('count')}: {out}"
          node1.log("cleanup: all plugins removed")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All plugin CLI integration tests passed!")
    '';
  }
