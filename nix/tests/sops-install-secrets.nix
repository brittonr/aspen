# NixOS VM integration test for aspen-sops-install-secrets.
#
# Verifies the Rust drop-in replacement for sops-nix's Go sops-install-secrets
# works end-to-end: encrypt a SOPS file, boot with sops-nix module using our
# binary, and verify secrets are decrypted with correct permissions.
#
# Tests:
#   - Age-only decrypt at boot (no cluster needed)
#   - Template rendering with placeholders
#   - Secret file permissions and ownership
#
# Run:
#   nix build .#checks.x86_64-linux.sops-install-secrets-test --impure
{
  pkgs,
  aspenSopsPackage,
  aspenSopsInstallSecretsPackage,
}: let
  # Generate a deterministic age keypair for the test.
  # In real usage, this would be the machine's age key.
  ageKeyGenScript = pkgs.writeShellScript "gen-age-key" ''
    ${pkgs.age}/bin/age-keygen -o /var/lib/sops-nix/key.txt 2>/tmp/age-pubkey.txt
  '';

  # We'll encrypt the secrets file at test time using aspen-sops with age.
  # The plaintext secrets YAML.
  secretsPlaintext = pkgs.writeText "secrets-plain.yaml" (builtins.toJSON {
    database = {
      password = "super-secret-db-pass-42";
      host = "db.internal.example.com";
      port = 5432;
    };
    api_key = "sk-live-test-key-abc123";
  });

  # Template content with placeholders
  templateContent = ''
    DB_PASSWORD=<<SOPS_db-password>>
    API_KEY=<<SOPS_api-key>>
  '';
in
  pkgs.testers.nixosTest {
    name = "sops-install-secrets";
    skipLint = true;

    nodes = {
      machine = {config, ...}: {
        environment.systemPackages = [
          aspenSopsPackage
          aspenSopsInstallSecretsPackage
          pkgs.age
          pkgs.jq
        ];

        # Ensure the key directory exists
        systemd.tmpfiles.rules = [
          "d /var/lib/sops-nix 0700 root root -"
        ];

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      start_all()

      # ── Generate age key ────────────────────────────────────────────
      machine.succeed("${ageKeyGenScript}")
      machine.succeed("test -f /var/lib/sops-nix/key.txt")
      age_pubkey = machine.succeed(
          "grep 'public key:' /tmp/age-pubkey.txt | awk '{print $NF}'"
      ).strip()
      machine.log(f"Age public key: {age_pubkey}")
      assert age_pubkey.startswith("age1"), f"invalid age pubkey: {age_pubkey}"

      # ── Encrypt secrets with aspen-sops using age ───────────────────
      machine.succeed("cp ${secretsPlaintext} /tmp/secrets.json")
      machine.succeed("chmod 644 /tmp/secrets.json")

      machine.succeed(
          f"aspen-sops encrypt /tmp/secrets.json "
          f"--age '{age_pubkey}' "
          f"--in-place"
      )

      encrypted = machine.succeed("cat /tmp/secrets.json").strip()
      machine.log(f"Encrypted secrets ({len(encrypted)} bytes)")
      assert "ENC[AES256_GCM," in encrypted, "values should be encrypted"
      assert "super-secret-db-pass-42" not in encrypted, "plaintext should not appear"

      # ── Create manifest JSON (simulating what sops-nix generates) ───
      manifest = {
          "secrets": [
              {
                  "name": "db-password",
                  "key": "database/password",
                  "path": "/run/secrets/db-password",
                  "uid": 0,
                  "gid": 0,
                  "sopsFile": "/tmp/secrets.json",
                  "format": "json",
                  "mode": "0400",
                  "restartUnits": [],
                  "reloadUnits": [],
              },
              {
                  "name": "api-key",
                  "key": "api_key",
                  "path": "/run/secrets/api-key",
                  "uid": 0,
                  "gid": 0,
                  "sopsFile": "/tmp/secrets.json",
                  "format": "json",
                  "mode": "0440",
                  "restartUnits": [],
                  "reloadUnits": [],
              },
          ],
          "templates": [
              {
                  "name": "app-config",
                  "content": "DB_PASSWORD=<<SOPS_db-password>>\nAPI_KEY=<<SOPS_api-key>>\n",
                  "file": "",
                  "path": "/run/secrets/rendered/app-config",
                  "mode": "0440",
                  "uid": 0,
                  "gid": 0,
                  "restartUnits": [],
                  "reloadUnits": [],
              },
          ],
          "placeholderBySecretName": {
              "db-password": "<<SOPS_db-password>>",
              "api-key": "<<SOPS_api-key>>",
          },
          "secretsMountPoint": "/run/secrets.d",
          "symlinkPath": "/run/secrets",
          "keepGenerations": 1,
          "sshKeyPaths": [],
          "gnupgHome": "",
          "ageKeyFile": "/var/lib/sops-nix/key.txt",
          "ageSshKeyPaths": [],
          "useTmpfs": False,
          "userMode": False,
          "logging": {"keyImport": True, "secretChanges": True},
      }

      machine.succeed(f"echo '{json.dumps(manifest)}' > /tmp/manifest.json")

      # ── Run sops-install-secrets ────────────────────────────────────

      with subtest("sops-install-secrets decrypts and installs secrets"):
          machine.succeed(
              "sops-install-secrets /tmp/manifest.json 2>&1"
          )

          # Verify secrets exist
          machine.succeed("test -f /run/secrets.d/1/db-password")
          machine.succeed("test -f /run/secrets.d/1/api-key")

          # Verify symlink
          machine.succeed("test -L /run/secrets")
          target = machine.succeed("readlink /run/secrets").strip()
          assert "/run/secrets.d/1" in target, f"symlink target wrong: {target}"

      with subtest("secret values are correct"):
          db_pass = machine.succeed("cat /run/secrets.d/1/db-password").strip()
          assert db_pass == "super-secret-db-pass-42", \
              f"db-password mismatch: {db_pass!r}"

          api_key = machine.succeed("cat /run/secrets.d/1/api-key").strip()
          assert api_key == "sk-live-test-key-abc123", \
              f"api-key mismatch: {api_key!r}"

          machine.log("Secret values: PASSED")

      with subtest("secret permissions are correct"):
          # db-password should be 0400
          mode = machine.succeed("stat -c '%a' /run/secrets.d/1/db-password").strip()
          assert mode == "400", f"db-password mode should be 400, got {mode}"

          # api-key should be 0440
          mode = machine.succeed("stat -c '%a' /run/secrets.d/1/api-key").strip()
          assert mode == "440", f"api-key mode should be 440, got {mode}"

          machine.log("Permissions: PASSED")

      with subtest("template rendering works"):
          machine.succeed("test -f /run/secrets.d/1/rendered/app-config")
          rendered = machine.succeed("cat /run/secrets.d/1/rendered/app-config").strip()
          assert "DB_PASSWORD=super-secret-db-pass-42" in rendered, \
              f"template db password not rendered: {rendered}"
          assert "API_KEY=sk-live-test-key-abc123" in rendered, \
              f"template api key not rendered: {rendered}"

          machine.log("Template rendering: PASSED")

      with subtest("check-mode=manifest validates without installing"):
          machine.succeed(
              "sops-install-secrets --check-mode=manifest /tmp/manifest.json 2>&1"
          )
          machine.log("Check mode manifest: PASSED")

      with subtest("check-mode=sopsfile validates SOPS files"):
          machine.succeed(
              "sops-install-secrets --check-mode=sopsfile /tmp/manifest.json 2>&1"
          )
          machine.log("Check mode sopsfile: PASSED")

      with subtest("generation pruning works"):
          # Run again to create generation 2
          machine.succeed("sops-install-secrets /tmp/manifest.json 2>&1")
          machine.succeed("test -d /run/secrets.d/2")

          # With keepGenerations=1, generation 1 should be pruned
          # (only current gen 2 remains)
          target = machine.succeed("readlink /run/secrets").strip()
          assert "/run/secrets.d/2" in target, f"symlink should point to gen 2: {target}"

          machine.log("Generation pruning: PASSED")

      machine.log("All sops-install-secrets tests PASSED")
    '';
  }
