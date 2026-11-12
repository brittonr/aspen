{ pkgs, moltenPkg, startupSourceBinding }:

pkgs.testers.runNixOSTest {
  name = "molten-module";
  # Source-gate measurement rehashes the retained build closure on each start;
  # the default one-hour test timeout is shorter than two measured starts in a
  # single-core VM.
  globalTimeout = 7200;
  nodes.machine =
    { ... }:
    {
      imports = [ ./modules/molten.nix ];
      # The test VM has no route to the operator binary cache. Disabling
      # substituters keeps the source-gate Nix evaluation offline and bounded
      # instead of retrying an unreachable cache for tens of minutes.
      nix.settings.substituters = [ ];
      nix.settings.connect-timeout = 1;
      virtualisation = {
        cores = 2;
        memorySize = 2048;
      };
      environment.systemPackages = [
        pkgs.nix
        pkgs.jq
      ];
      services.molten = {
        enable = true;
        package = moltenPkg;
        nodeId = "node:module-vm";
        sourceTrustFile = "/var/lib/molten/fixture-trust.json";
      };
      systemd.services.molten-trust-fixture = {
        description = "Molten module VM fixture trust";
        wantedBy = [ "multi-user.target" ];
        before = [ "molten.service" ];
        requiredBy = [ "molten.service" ];
        path = [
          pkgs.nix
          pkgs.b3sum
          pkgs.coreutils
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          StateDirectory = "molten";
          StateDirectoryMode = "0700";
        };
        script = ''
          set -euo pipefail
          state=/var/lib/molten
          key="$state/fixture-signing-key"
          nix --extra-experimental-features 'nix-command blake3-hashes' \
            key generate-secret --key-name molten-module-fixture > "$key"
          chmod 600 "$key"
          public=$(nix --extra-experimental-features 'nix-command blake3-hashes' \
            key convert-secret-to-public < "$key")
          nix --extra-experimental-features 'nix-command blake3-hashes' \
            store sign --key-file "$key" ${startupSourceBinding}
          binding=$(b3sum ${startupSourceBinding}/binding.json | cut -d' ' -f1)
          policy=$(b3sum ${startupSourceBinding}/policy | cut -d' ' -f1)
          profile=$(b3sum ${startupSourceBinding}/profile | cut -d' ' -f1)
          printf '{"schema":"molten-node-source-trust-v1","nix":"%s","output":"%s","public_key":"%s","selection":{"binding_ref":"blake3:%s","policy_ref":"blake3:%s","profile_ref":"blake3:%s"}}\n' \
            "${pkgs.nix}/bin/nix" "${startupSourceBinding}" "$public" "$binding" "$policy" "$profile" \
            > "$state/fixture-trust.json"
          chown molten:molten "$state/fixture-trust.json" "$key"
          chmod 0644 "$state/fixture-trust.json"
        '';
      };
      system.stateVersion = "24.11";
    };
  testScript = ''
    start_all()
    machine.wait_for_unit("molten-trust-fixture.service", timeout=3600)
    machine.wait_for_unit("molten.service", timeout=3600)
    machine.succeed("test -s /var/lib/molten/evidence/startup.preserves")
    machine.succeed("test -s /var/lib/molten/evidence/node-config.preserves")
    machine.succeed("systemctl restart molten.service")
    machine.wait_for_unit("molten.service", timeout=3600)
    machine.succeed("test -s /var/lib/molten/evidence/startup.preserves")
    machine.succeed("systemctl stop molten.service")
    machine.succeed("test -s /var/lib/molten/evidence/shutdown.preserves")

    machine.succeed("cp /var/lib/molten/fixture-trust.json /var/lib/molten/good-trust.json")

    # An absent operator-selected trust document must fail before startup evidence.
    machine.succeed("rm -f /var/lib/molten/fixture-trust.json")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.fail("systemctl is-active --quiet molten.service")
    machine.succeed("journalctl -u molten.service --no-pager | grep -q 'fixture-trust.json'")

    # A stale selection must fail before protected startup effects.
    machine.succeed("jq '.selection.binding_ref = \"blake3:0000000000000000000000000000000000000000000000000000000000000000\"' /var/lib/molten/good-trust.json > /var/lib/molten/stale.json")
    machine.succeed("mv /var/lib/molten/stale.json /var/lib/molten/fixture-trust.json")
    machine.succeed("rm -f /var/lib/molten/evidence/startup.preserves")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # A wrong signer must fail before protected startup effects.
    machine.succeed("nix --extra-experimental-features 'nix-command blake3-hashes' key generate-secret --key-name molten-module-wrong > /var/lib/molten/wrong-key")
    machine.succeed("public=$(nix --extra-experimental-features 'nix-command blake3-hashes' key convert-secret-to-public < /var/lib/molten/wrong-key); jq --arg key \"$public\" '.public_key = $key' /var/lib/molten/good-trust.json > /var/lib/molten/wrong.json")
    machine.succeed("mv /var/lib/molten/wrong.json /var/lib/molten/fixture-trust.json")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")
  '';
}
