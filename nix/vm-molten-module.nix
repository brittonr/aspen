{ pkgs, startupSourceBinding }:

pkgs.testers.runNixOSTest {
  name = "molten-module";
  # Source admission remeasures the retained build closure on each start.
  # This profile supplies 8 vCPUs/8 GiB; both 3600-second startup waits and
  # the 7200-second aggregate driver bound remain unchanged.
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
        cores = 8;
        memorySize = 8192;
        # Admission still hashes every retained member; avoid host-store 9p I/O.
        useNixStoreImage = true;
        # Boot registration/signing needs Nix bookkeeping in a private upper layer.
        writableStore = true;
      };
      environment.systemPackages = [
        pkgs.nix
        pkgs.jq
        pkgs.util-linux
      ];
      services.molten = {
        enable = true;
        package = startupSourceBinding;
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
          StateDirectory = "molten-trust-fixture";
          StateDirectoryMode = "0700";
        };
        script = ''
          set -euo pipefail
          state=/var/lib/molten
          key=/var/lib/molten-trust-fixture/fixture-signing-key
          mkdir -p "$state"
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
          chown molten:molten "$state/fixture-trust.json"
          chmod 0644 "$state/fixture-trust.json"
        '';
      };
      system.stateVersion = "24.11";
    };
  testScript = ''
    def wait_for_started():
        try:
            machine.wait_for_unit("molten.service", timeout=3600)
        except Exception:
            for command in [
                "systemctl show molten.service --property=ActiveState,SubState,Result,MainPID,ControlPID",
                "journalctl --no-pager --lines=80 --unit=molten.service",
                "ps -eo pid,ppid,etimes,time,rss,comm,args",
                "cat /sys/fs/cgroup/system.slice/molten.service/cpu.stat /sys/fs/cgroup/system.slice/molten.service/memory.events",
            ]:
                try:
                    print(machine.execute(command, timeout=30)[1])
                except Exception as diagnostic_error:
                    print(f"startup diagnostic unavailable: {diagnostic_error}")
            raise

    start_all()
    machine.wait_for_unit("molten-trust-fixture.service", timeout=3600)
    wait_for_started()
    machine.succeed("test -s /var/lib/molten/evidence/startup.preserves")
    machine.succeed("test -s /var/lib/molten/evidence/node-config.preserves")
    machine.succeed("systemctl restart molten.service")
    wait_for_started()
    machine.succeed("test -s /var/lib/molten/evidence/startup.preserves")
    machine.succeed("systemctl stop molten.service")
    machine.succeed("test -s /var/lib/molten/evidence/shutdown.preserves")
    machine.succeed("test $(systemctl show molten.service --property=Result --value) = success")
    machine.fail("systemctl is-active --quiet molten.service")

    machine.succeed("cp /var/lib/molten/fixture-trust.json /var/lib/molten/good-trust.json")

    # Each denial starts with no startup artifact: source admission must fail
    # before it could publish protected startup evidence.
    machine.succeed("rm -f /var/lib/molten/fixture-trust.json /var/lib/molten/evidence/startup.preserves")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.succeed("systemctl stop molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # A stale selection must fail against the selected package.
    machine.succeed("jq '.selection.binding_ref = \"blake3:0000000000000000000000000000000000000000000000000000\"' /var/lib/molten/good-trust.json > /var/lib/molten/fixture-trust.json")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.succeed("systemctl stop molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # A wrong signer cannot authenticate the admitted output.
    machine.succeed("nix --extra-experimental-features 'nix-command blake3-hashes' key generate-secret --key-name molten-module-wrong > /var/lib/molten-trust-fixture/wrong-key; chmod 0600 /var/lib/molten-trust-fixture/wrong-key")
    machine.succeed("public=$(nix --extra-experimental-features 'nix-command blake3-hashes' key convert-secret-to-public < /var/lib/molten-trust-fixture/wrong-key); jq --arg key \"$public\" '.public_key = $key' /var/lib/molten/good-trust.json > /var/lib/molten/fixture-trust.json")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.succeed("systemctl stop molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # The same public key does not authenticate an unselected output.
    machine.succeed("jq --arg output '${pkgs.coreutils}' '.output = $output' /var/lib/molten/good-trust.json > /var/lib/molten/fixture-trust.json")
    machine.succeed("systemctl reset-failed molten.service")
    machine.fail("systemctl start molten.service")
    machine.succeed("systemctl stop molten.service")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # A modified copy of the selected ELF must not execute against good trust.
    machine.succeed("cp /var/lib/molten/good-trust.json /var/lib/molten/fixture-trust.json")
    machine.succeed("cp ${startupSourceBinding}/bin/molten /var/lib/molten/altered-molten; printf x >> /var/lib/molten/altered-molten; chmod 0755 /var/lib/molten/altered-molten")
    machine.fail("/var/lib/molten/altered-molten node run --state-root /var/lib/molten --startup-out /var/lib/molten/evidence/startup.preserves > /var/lib/molten/altered-binary.log 2>&1")
    machine.fail("test -e /var/lib/molten/evidence/startup.preserves")

    # A private bind mount changes a retained artifact without modifying the
    # signed store output or leaking the mutation into the VM's mount namespace.
    machine.succeed("cp ${startupSourceBinding}/policy /var/lib/molten/altered-policy; printf x >> /var/lib/molten/altered-policy")
    machine.succeed("""
      set -euo pipefail
      unshare --mount --propagation private -- sh -eu -c '
        mount --bind /var/lib/molten/altered-policy ${startupSourceBinding}/policy
        mountpoint -q ${startupSourceBinding}/policy
        if ${startupSourceBinding}/bin/molten node run --state-root /var/lib/molten --startup-out /var/lib/molten/evidence/startup.preserves > /var/lib/molten/altered-policy.log 2>&1; then
          exit 1
        fi
      '
      test ! -e /var/lib/molten/evidence/startup.preserves
    """)
  '';
}
