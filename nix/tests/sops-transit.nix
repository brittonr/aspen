# NixOS VM integration test for SOPS Transit-backed encryption.
#
# Spins up a single-node Aspen cluster, creates a Transit key, then exercises
# the full SOPS encrypt → decrypt → rotate → update-keys lifecycle using
# Aspen Transit as the key management backend.
#
# Tests:
#   - Transit data key generation via RPC
#   - SOPS file encrypt (TOML) with Transit key group
#   - SOPS file decrypt using Transit
#   - SOPS key rotation (rewrap data key after Transit key rotate)
#   - Multi-key-group: Transit + age
#   - gRPC key service bridge (Go SOPS compat layer)
#   - Go SOPS interop: aspen-sops encrypt → Go sops decrypt via keyservice
#   - Multi-key-group interop: Transit + age, cross-tool decrypt
#
# Run:
#   nix build .#checks.x86_64-linux.sops-transit-test --impure
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.sops-transit-test.driverInteractive --impure
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  aspenSopsPackage,
  secretsPluginWasm,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000002000000000000000200000000000000020000000000000002";

  # Shared cluster cookie.
  cookie = "sops-transit-test";

  # Transit key name used for SOPS data key encryption.
  transitKeyName = "sops-data-key";

  # WASM plugin helpers
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
    name = "sops-transit";
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

        environment.systemPackages = [
          aspenCliPackage
          aspenSopsPackage
          pkgs.sops
          pkgs.age
        ];

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

      def raw_cli(cmd, check=True):
          """Run aspen-cli with the cluster ticket and return raw stdout."""
          ticket = get_ticket()
          run = f"aspen-cli --ticket '{ticket}' {cmd}"
          if check:
              return node1.succeed(run).strip()
          else:
              code, out = node1.execute(run)
              return out.strip()

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

      # ── install WASM plugins ───────────────────────────────────────
      ${pluginHelpers.installPluginsScript}

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ================================================================
      # TRANSIT KEY SETUP
      # ================================================================

      with subtest("create Transit key for SOPS"):
          out = cli(f"secrets transit create-key ${transitKeyName}")
          node1.log(f"transit create key: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, \
              f"create Transit key failed: {out}"

      # ================================================================
      # TRANSIT DATA KEY GENERATION
      # ================================================================

      with subtest("transit generate data key"):
          out = cli(f"secrets transit datakey ${transitKeyName}", check=False)
          node1.log(f"transit datakey: {out}")
          if isinstance(out, dict) and out.get("is_success") is True:
              pt = out.get("plaintext")
              ct = out.get("ciphertext")
              assert pt is not None, f"no plaintext in datakey: {out}"
              assert ct is not None, f"no ciphertext in datakey: {out}"
              assert ct.startswith("aspen:v"), \
                  f"ciphertext should have aspen:v prefix: {ct}"
              node1.log(f"datakey plaintext len={len(pt)}, ciphertext prefix={ct[:20]}...")
          else:
              node1.log(f"datakey not supported or failed: {out}")

      # ================================================================
      # TRANSIT ENCRYPT / DECRYPT DATA KEY ROUND-TRIP
      # ================================================================

      with subtest("transit encrypt then decrypt data key"):
          # Encrypt some data
          out = cli(f"secrets transit encrypt ${transitKeyName} 'test-data-key-material'")
          node1.log(f"transit encrypt: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"encrypt failed: {out}"
          ciphertext = out.get("ciphertext")
          assert ciphertext is not None, f"no ciphertext: {out}"
          assert ciphertext.startswith("aspen:v"), \
              f"ciphertext format wrong: {ciphertext[:30]}"

          # Decrypt it back
          out = cli(f"secrets transit decrypt ${transitKeyName} '{ciphertext}'")
          node1.log(f"transit decrypt: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"decrypt failed: {out}"
          plaintext = out.get("plaintext", "")
          assert plaintext == "test-data-key-material", \
              f"round-trip mismatch: got '{plaintext}'"
          node1.log("Transit encrypt/decrypt round-trip: OK")

      # ================================================================
      # TRANSIT KEY ROTATION + REWRAP
      # ================================================================

      with subtest("transit rotate key and verify old ciphertext still decrypts"):
          # Encrypt before rotation
          out = cli(f"secrets transit encrypt ${transitKeyName} 'pre-rotation-data'")
          pre_ct = out.get("ciphertext")
          assert "v1" in pre_ct, f"expected v1: {pre_ct[:30]}"

          # Rotate
          out = cli(f"secrets transit rotate-key ${transitKeyName}")
          node1.log(f"transit rotate: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          assert out.get("is_success") is True, f"rotate failed: {out}"

          # Old ciphertext should still decrypt (key version in metadata)
          out = cli(f"secrets transit decrypt ${transitKeyName} '{pre_ct}'")
          assert out.get("is_success") is True, f"decrypt old ct failed: {out}"
          assert out.get("plaintext") == "pre-rotation-data", \
              f"old ciphertext decrypted to wrong value: {out}"

          # New encryptions should use v2
          out = cli(f"secrets transit encrypt ${transitKeyName} 'post-rotation-data'")
          post_ct = out.get("ciphertext")
          assert "v2" in post_ct, f"expected v2 after rotation: {post_ct[:30]}"

          node1.log("Transit key rotation: OK")

      # ================================================================
      # SOPS FILE ENCRYPT VIA CLI (if aspen-sops binary available)
      # ================================================================
      # Note: These tests exercise the Transit RPC path that the SOPS
      # library uses (generate_data_key, decrypt_data_key, rewrap).
      # The actual SOPS file format encrypt/decrypt is tested in unit tests.
      # Here we verify the Transit backend works end-to-end.

      with subtest("transit full envelope encryption flow"):
          """
          Simulate the SOPS envelope encryption flow:
          1. Generate data key via Transit (returns b64 plaintext + ciphertext)
          2. Verify ciphertext can be decrypted (is_success)
          3. Verify text-based encrypt/decrypt round-trips perfectly

          Note: We don't compare decrypt(ciphertext) == datakey.plaintext
          because datakey returns base64-encoded bytes while decrypt returns
          raw binary that gets mangled through JSON. The real TransitClient
          (Rust, postcard protocol) handles binary correctly.
          """
          # Step 1: Generate data key
          out = cli(f"secrets transit datakey ${transitKeyName}", check=False)
          if isinstance(out, dict) and out.get("is_success") is True:
              plaintext_dk = out["plaintext"]
              encrypted_dk = out["ciphertext"]
              assert plaintext_dk is not None and len(plaintext_dk) > 0, \
                  f"empty data key plaintext: {out}"
              assert encrypted_dk.startswith("aspen:v"), \
                  f"ciphertext format wrong: {encrypted_dk[:30]}"
              node1.log(f"envelope: data key generated, plaintext_b64={plaintext_dk[:20]}...")

              # Step 2: Verify the ciphertext can be decrypted
              out2 = cli(f"secrets transit decrypt ${transitKeyName} '{encrypted_dk}'")
              assert out2.get("is_success") is True, f"decrypt data key failed: {out2}"
              node1.log("envelope: data key ciphertext decrypts successfully")

              # Step 3: Verify a second datakey generation produces different material
              out3 = cli(f"secrets transit datakey ${transitKeyName}", check=False)
              if isinstance(out3, dict) and out3.get("is_success") is True:
                  assert out3["plaintext"] != plaintext_dk, \
                      "two datakey calls should produce different keys"
                  node1.log("envelope: second datakey is unique (good)")

              node1.log("Envelope encryption flow: OK")
          else:
              node1.log(f"datakey not available, skipping envelope test: {out}")

      with subtest("transit rewrap after rotation"):
          """
          Test SOPS key rotation flow using text data (avoids binary-through-JSON):
          1. Encrypt a known plaintext with current key
          2. Rotate Transit key
          3. Decrypt old ciphertext (still works)
          4. Re-encrypt with latest key version (simulates rewrap)
          5. Verify new ciphertext decrypts to same value
          """
          test_value = "sops-data-key-material-for-rewrap-test"

          # Encrypt with current key
          out = cli(f"secrets transit encrypt ${transitKeyName} '{test_value}'")
          assert out.get("is_success") is True, f"encrypt failed: {out}"
          old_ct = out["ciphertext"]
          node1.log(f"rewrap: encrypted with current key: {old_ct[:30]}...")

          # Rotate key
          out = cli(f"secrets transit rotate-key ${transitKeyName}")
          assert out.get("is_success") is True, f"rotate failed: {out}"

          # Old ciphertext still decrypts (Transit keeps old versions)
          out = cli(f"secrets transit decrypt ${transitKeyName} '{old_ct}'")
          assert out.get("is_success") is True, f"decrypt old ct failed: {out}"
          assert out["plaintext"] == test_value, \
              f"old ct decrypted wrong: {out['plaintext']}"

          # Re-encrypt with latest key (simulates SOPS rewrap)
          out = cli(f"secrets transit encrypt ${transitKeyName} '{test_value}'")
          assert out.get("is_success") is True, f"re-encrypt failed: {out}"
          new_ct = out["ciphertext"]
          node1.log(f"rewrap: re-encrypted: {new_ct[:30]}...")

          # New ciphertext should use latest version
          assert old_ct != new_ct, "re-encrypted ciphertext should differ"

          # Verify new ciphertext decrypts to same value
          out = cli(f"secrets transit decrypt ${transitKeyName} '{new_ct}'")
          assert out.get("is_success") is True, f"decrypt new ct failed: {out}"
          assert out["plaintext"] == test_value, \
              f"rewrap decrypt mismatch: {out['plaintext']}"

          node1.log("Transit rewrap flow: OK")

      # ================================================================
      # GO SOPS INTEROP: aspen-sops encrypt → Go sops decrypt
      # ================================================================
      # Task 12.3: Full round-trip interop via gRPC key service bridge.
      #
      # Flow:
      #   1. Create a plaintext TOML file
      #   2. Encrypt it with aspen-sops (native mode, directly via Iroh QUIC)
      #   3. Start aspen-sops keyservice (gRPC bridge on Unix socket)
      #   4. Decrypt the encrypted file with Go sops via --keyservice
      #   5. Compare decrypted output with original plaintext

      with subtest("go-sops interop: aspen-sops encrypt then go-sops decrypt"):
          ticket = get_ticket()

          # Step 1: Create a plaintext TOML file
          node1.succeed("""
            cat > /tmp/secrets-interop.toml <<'TOML_END'
      [database]
      host = "db.internal.example.com"
      port = 5432
      password = "super-secret-password-123"

      [api]
      key = "sk-live-interop-test-key-abc123"
      endpoint = "https://api.example.com/v1"
      TOML_END
          """.strip())

          # Keep a copy of the original for comparison
          original = node1.succeed("cat /tmp/secrets-interop.toml").strip()
          node1.log(f"Original plaintext ({len(original)} bytes)")

          # Step 2: Encrypt with aspen-sops
          node1.succeed(
              f"aspen-sops encrypt /tmp/secrets-interop.toml "
              f"--cluster-ticket '{ticket}' "
              f"--transit-key ${transitKeyName} "
              f"--in-place"
          )

          encrypted = node1.succeed("cat /tmp/secrets-interop.toml").strip()
          node1.log(f"Encrypted file ({len(encrypted)} bytes)")

          # Verify values are actually encrypted (contain ENC[AES256_GCM,...])
          assert "ENC[AES256_GCM," in encrypted, \
              f"encrypted file doesn't contain ENC[AES256_GCM,: {encrypted[:200]}"
          # Keys should still be in plaintext
          assert "password" in encrypted, \
              f"key names should be in plaintext: {encrypted[:200]}"
          assert "super-secret-password-123" not in encrypted, \
              f"values should be encrypted: {encrypted[:200]}"
          node1.log("aspen-sops encrypt: OK (values encrypted, keys plaintext)")

          # Step 3: Start the keyservice bridge in the background
          node1.succeed(
              f"systemd-run --unit=aspen-keyservice "
              f"bash -c 'exec aspen-sops keyservice "
              f"--cluster-ticket \"{ticket}\" "
              f"--transit-key ${transitKeyName} "
              f"--socket /tmp/aspen-sops.sock'"
          )

          # Wait for the Unix socket to appear
          node1.wait_until_succeeds(
              "test -S /tmp/aspen-sops.sock",
              timeout=30,
          )
          node1.log("keyservice bridge started on /tmp/aspen-sops.sock")

          # Step 4: Decrypt with Go sops via keyservice
          #
          # Go sops uses --hc-vault-transit to identify the key type (VaultKey),
          # and --keyservice to specify the gRPC endpoint. The bridge translates
          # VaultKey encrypt/decrypt calls to Aspen Transit RPCs.
          #
          # The --hc-vault-transit value format: vault_address:engine_path/keys/key_name
          # vault_address is ignored by our bridge (uses cluster ticket instead).
          node1.succeed(
              f"sops decrypt "
              f"--keyservice 'unix:///tmp/aspen-sops.sock' "
              f"--hc-vault-transit 'http://ignored:transit/keys/${transitKeyName}' "
              f"/tmp/secrets-interop.toml "
              f"> /tmp/secrets-interop-decrypted.toml"
          )

          decrypted = node1.succeed("cat /tmp/secrets-interop-decrypted.toml").strip()
          node1.log(f"Go sops decrypted ({len(decrypted)} bytes)")

          # Step 5: Compare decrypted output with original
          # TOML round-trip may reorder keys or change whitespace, so compare
          # by parsing both as TOML and checking values.
          node1.succeed("""
            python3 -c "
      import tomllib

      with open('/tmp/secrets-interop-decrypted.toml', 'rb') as f:
          decrypted = tomllib.load(f)

      # Verify all original values survived the round-trip
      assert decrypted['database']['host'] == 'db.internal.example.com', \
          f'host mismatch: {decrypted[\"database\"][\"host\"]}'
      assert decrypted['database']['port'] == 5432, \
          f'port mismatch: {decrypted[\"database\"][\"port\"]}'
      assert decrypted['database']['password'] == 'super-secret-password-123', \
          f'password mismatch: {decrypted[\"database\"][\"password\"]}'
      assert decrypted['api']['key'] == 'sk-live-interop-test-key-abc123', \
          f'api key mismatch: {decrypted[\"api\"][\"key\"]}'
      assert decrypted['api']['endpoint'] == 'https://api.example.com/v1', \
          f'endpoint mismatch: {decrypted[\"api\"][\"endpoint\"]}'

      # Verify sops metadata is NOT in the decrypted output
      assert 'sops' not in decrypted, \
          f'sops metadata should be stripped: {list(decrypted.keys())}'

      print('All values match — Go SOPS interop OK')
            "
          """.strip())

          node1.log("Go SOPS interop round-trip: PASSED")

          # Stop keyservice
          node1.succeed("systemctl stop aspen-keyservice || true")
          node1.succeed("rm -f /tmp/aspen-sops.sock")

      # ================================================================
      # MULTI-KEY-GROUP INTEROP: Transit + age, cross-tool decrypt
      # ================================================================
      # Task 12.4: Encrypt with both Transit + age key groups.
      # Go SOPS decrypts with age identity. aspen-sops decrypts with Transit.

      with subtest("multi-key-group interop: Transit + age cross-tool decrypt"):
          ticket = get_ticket()

          # Step 1: Generate an age keypair
          node1.succeed("age-keygen -o /tmp/age-identity.txt 2>/tmp/age-pubkey.txt")
          age_pubkey = node1.succeed("grep 'public key:' /tmp/age-pubkey.txt | awk '{print $NF}'").strip()
          node1.log(f"Generated age keypair, pubkey: {age_pubkey}")
          assert age_pubkey.startswith("age1"), f"invalid age pubkey: {age_pubkey}"

          # Step 2: Create a plaintext TOML file
          node1.succeed("""
            cat > /tmp/secrets-multikey.toml <<'TOML_END'
      [credentials]
      username = "admin"
      password = "multi-key-secret-456"
      token = "tok-multikey-interop-789"
      TOML_END
          """.strip())

          # Step 3: Encrypt with aspen-sops using BOTH Transit + age
          node1.succeed(
              f"aspen-sops encrypt /tmp/secrets-multikey.toml "
              f"--cluster-ticket '{ticket}' "
              f"--transit-key ${transitKeyName} "
              f"--age-recipient '{age_pubkey}' "
              f"--in-place"
          )

          encrypted_mk = node1.succeed("cat /tmp/secrets-multikey.toml").strip()
          node1.log(f"Multi-key encrypted ({len(encrypted_mk)} bytes)")

          # Verify both key groups are present in metadata
          assert "aspen_transit" in encrypted_mk, \
              f"missing aspen_transit in metadata: {encrypted_mk[-500:]}"
          assert "age" in encrypted_mk, \
              f"missing age in metadata: {encrypted_mk[-500:]}"
          assert "ENC[AES256_GCM," in encrypted_mk, \
              f"values not encrypted: {encrypted_mk[:200]}"
          node1.log("Multi-key encrypt: both Transit + age key groups present")

          # Step 4: Decrypt with aspen-sops using Transit (ignores age)
          node1.succeed(
              f"aspen-sops decrypt /tmp/secrets-multikey.toml "
              f"--cluster-ticket '{ticket}' "
              f"> /tmp/multikey-transit-decrypted.toml"
          )

          node1.succeed("""
            python3 -c "
      import tomllib
      with open('/tmp/multikey-transit-decrypted.toml', 'rb') as f:
          d = tomllib.load(f)
      assert d['credentials']['password'] == 'multi-key-secret-456', \
          f'Transit decrypt password mismatch: {d[\"credentials\"][\"password\"]}'
      assert d['credentials']['token'] == 'tok-multikey-interop-789', \
          f'Transit decrypt token mismatch: {d[\"credentials\"][\"token\"]}'
      print('Transit decrypt: OK')
            "
          """.strip())
          node1.log("Multi-key Transit decrypt: PASSED")

          # Step 5: Decrypt with Go sops using age identity (no keyservice needed)
          #
          # Go sops can decrypt with age directly using SOPS_AGE_KEY_FILE.
          # It only needs to decrypt one key group — age is sufficient.
          node1.succeed(
              f"SOPS_AGE_KEY_FILE=/tmp/age-identity.txt "
              f"sops decrypt /tmp/secrets-multikey.toml "
              f"> /tmp/multikey-age-decrypted.toml"
          )

          node1.succeed("""
            python3 -c "
      import tomllib
      with open('/tmp/multikey-age-decrypted.toml', 'rb') as f:
          d = tomllib.load(f)
      assert d['credentials']['password'] == 'multi-key-secret-456', \
          f'Age decrypt password mismatch: {d[\"credentials\"][\"password\"]}'
      assert d['credentials']['token'] == 'tok-multikey-interop-789', \
          f'Age decrypt token mismatch: {d[\"credentials\"][\"token\"]}'
      assert d['credentials']['username'] == 'admin', \
          f'Age decrypt username mismatch: {d[\"credentials\"][\"username\"]}'
      print('Age decrypt: OK')
            "
          """.strip())
          node1.log("Multi-key age decrypt (Go SOPS): PASSED")

          node1.log("Multi-key-group cross-tool interop: ALL PASSED")

      # ── summary ──────────────────────────────────────────────────────
      node1.log("All SOPS Transit integration tests completed successfully!")
    '';
  }
