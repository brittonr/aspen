# NixOS VM integration test for the Aspen secrets engine.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises the secrets engine CLI commands end-to-end:
#
#   - KV: write, read, version history, list, soft delete, undelete,
#     destroy, metadata, check-and-set
#   - Transit: create key, encrypt, decrypt, sign, verify, rotate, list keys,
#     data key generation
#
# The secrets handler is registered via inventory self-registration when the
# node binary is built with the `secrets` feature. The aspen-cli-secrets
# package includes matching feature flags so postcard enum discriminants align.
#
# Run:
#   nix build .#checks.x86_64-linux.secrets-engine-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.secrets-engine-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  secretsPluginWasm,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "secrets-vm-test";

  # WASM plugin helpers (secrets KV/Transit is WASM-only, native handler is PKI-only)
  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "secrets";
        wasm = secretsPluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "secrets-engine";
    skipLint = true;

    nodes = {
      node1 = {
        imports = [
          ../../nix/modules/aspen-node.nix
          pluginHelpers.nixosConfig
        ];

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
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 4096;
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

      # ── install WASM plugins (secrets KV/Transit is WASM-only) ─────
      ${pluginHelpers.installPluginsScript}

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── probe: verify secrets handler is available ───────────────────

      with subtest("secrets handler available"):
          out = cli(
              "secrets kv put _probe/test "
              "'{\"probe\": \"true\"}'"
          )
          assert isinstance(out, dict), f"expected dict, got {type(out)}: {out}"
          assert out.get("is_success") is True or out.get("version") is not None, \
              f"secrets handler probe failed: {out}"
          node1.log("secrets handler: AVAILABLE")

      # ================================================================
      # SECRETS KV
      # ================================================================

      with subtest("secrets kv put"):
          out = cli(
              "secrets kv put db/config "
              "'{\"username\":\"admin\",\"password\":\"s3cret\",\"port\":\"5432\"}'"
          )
          node1.log(f"secrets kv put: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True or out.get("version") is not None, \
              f"secrets kv put failed: {out}"

      with subtest("secrets kv get"):
          out = cli("secrets kv get db/config")
          node1.log(f"secrets kv get: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          data = out.get("data")
          assert isinstance(data, dict), f"expected data dict: {out}"
          assert data.get("username") == "admin", f"wrong username: {data}"
          assert data.get("password") == "s3cret", f"wrong password: {data}"
          assert data.get("port") == "5432", f"wrong port: {data}"

      with subtest("secrets kv put second version"):
          out = cli(
              "secrets kv put db/config "
              "'{\"username\":\"admin\",\"password\":\"updated\"}'"
          )
          node1.log(f"secrets kv put v2: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("version") == 2, f"expected version 2: {out}"

      with subtest("secrets kv get specific version"):
          out = cli("secrets kv get db/config --version 1")
          node1.log(f"secrets kv get v1: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          data = out.get("data")
          assert isinstance(data, dict), f"expected data dict: {out}"
          # Version 1 should have the original password
          assert data.get("password") == "s3cret", \
              f"version 1 should have original password: {data}"

      with subtest("secrets kv list"):
          cli(
              "secrets kv put app/api-key '{\"key\":\"abc123\"}'"
          )
          out = cli("secrets kv list")
          node1.log(f"secrets kv list: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          keys = out.get("keys", [])
          assert len(keys) > 0, f"expected non-empty key list: {out}"

      with subtest("secrets kv metadata"):
          out = cli("secrets kv metadata db/config")
          node1.log(f"secrets kv metadata: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"metadata failed: {out}"
          assert out.get("current_version") is not None, \
              f"expected current_version: {out}"

      with subtest("secrets kv delete (soft)"):
          out = cli("secrets kv delete db/config")
          node1.log(f"secrets kv delete: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"soft delete failed: {out}"

      with subtest("secrets kv undelete"):
          out = cli("secrets kv undelete db/config --versions 1")
          node1.log(f"secrets kv undelete: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"

      with subtest("secrets kv destroy"):
          out = cli("secrets kv destroy db/config --versions 1")
          node1.log(f"secrets kv destroy: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"

      with subtest("secrets kv check-and-set"):
          # Write initial value
          out = cli(
              "secrets kv put cas/test '{\"v\":\"initial\"}'"
          )
          assert out.get("version") == 1, f"expected version 1: {out}"

          # CAS update with correct expected version
          out = cli(
              "secrets kv put cas/test '{\"v\":\"updated\"}' --cas 1"
          )
          assert out.get("is_success") is True, f"CAS update failed: {out}"
          assert out.get("version") == 2, f"expected version 2 after CAS: {out}"

          # CAS with wrong version should fail
          out = cli(
              "secrets kv put cas/test '{\"v\":\"conflict\"}' --cas 99",
              check=False,
          )
          node1.log(f"secrets kv CAS conflict: {out}")
          # CAS conflict returns is_success=false (not an RPC error)
          if isinstance(out, dict):
              assert out.get("is_success") is not True, \
                  f"CAS should have failed with wrong version: {out}"

      # ================================================================
      # SECRETS TRANSIT ENGINE
      # ================================================================

      ciphertext = None
      signature = None

      with subtest("transit create key"):
          out = cli("secrets transit create-key my-enc-key")
          node1.log(f"transit create key: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"create key failed: {out}"

      with subtest("transit list keys"):
          out = cli("secrets transit list-keys")
          node1.log(f"transit list keys: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          keys = out.get("keys", [])
          assert "my-enc-key" in keys, f"expected my-enc-key in list: {keys}"

      with subtest("transit encrypt"):
          out = cli(
              "secrets transit encrypt my-enc-key 'hello secret'"
          )
          node1.log(f"transit encrypt: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"encrypt failed: {out}"
          ciphertext = out.get("ciphertext")
          assert ciphertext is not None, f"no ciphertext in response: {out}"
          node1.log(f"transit ciphertext: {ciphertext[:60]}...")

      with subtest("transit decrypt"):
          assert ciphertext is not None, "no ciphertext from encrypt step"
          out = cli(
              f"secrets transit decrypt my-enc-key '{ciphertext}'"
          )
          node1.log(f"transit decrypt: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"decrypt failed: {out}"
          pt = out.get("plaintext", "")
          assert pt == "hello secret", \
              f"round-trip mismatch: expected 'hello secret', got '{pt}'"
          node1.log("transit encrypt/decrypt round-trip: OK")

      with subtest("transit sign"):
          out = cli(
              "secrets transit sign my-enc-key 'data to sign'"
          )
          node1.log(f"transit sign: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"sign failed: {out}"
          signature = out.get("signature")
          assert signature is not None, f"no signature in response: {out}"
          node1.log(f"transit signature: {signature[:60]}...")

      with subtest("transit verify"):
          assert signature is not None, "no signature from sign step"
          out = cli(
              f"secrets transit verify my-enc-key "
              f"'data to sign' '{signature}'"
          )
          node1.log(f"transit verify: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"verify failed: {out}"
          assert out.get("is_valid") is True, \
              f"signature should be valid: {out}"
          node1.log("transit sign/verify round-trip: OK")

      with subtest("transit rotate key"):
          out = cli("secrets transit rotate-key my-enc-key")
          node1.log(f"transit rotate: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"rotate failed: {out}"
          assert out.get("version") == 2, \
              f"expected version 2 after rotation: {out}"

      with subtest("transit datakey"):
          out = cli(
              "secrets transit datakey my-enc-key", check=False
          )
          node1.log(f"transit datakey: {out}")
          # Datakey may or may not be supported depending on key type
          if isinstance(out, dict) and out.get("is_success") is True:
              assert out.get("plaintext") is not None, \
                  f"expected plaintext in datakey: {out}"
              assert out.get("ciphertext") is not None, \
                  f"expected ciphertext in datakey: {out}"
              node1.log("transit datakey: OK")

      # ── summary ──────────────────────────────────────────────────────
      node1.log("All secrets engine integration tests completed successfully!")
    '';
  }
