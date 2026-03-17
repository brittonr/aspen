# NixOS VM integration test: fully-native npins build pipeline (zero subprocesses).
#
# Proves the end-to-end pipeline with no `nix` subprocess:
#   1. Boot aspen-node with snix-build + snix-eval features
#   2. Create a minimal npins project (no nixpkgs, just derivation literals)
#   3. Submit a ci_nix_build job pointing at the npins project
#   4. Verify the build completes via snix-eval + LocalStoreBuildService
#   5. Confirm "zero subprocesses" appears in logs (no nix eval fallback)
#
# The npins project uses a hand-written default.nix + sources.json — no
# actual npins CLI needed. The default.nix returns an attrset with a
# trivial derivation, matching what `import ./npins` would return for a
# real project that then gets nixpkgs from the sources.
#
# Run:
#   nix build .#checks.x86_64-linux.npins-native-eval-test --impure
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  gatewayPackage,
}: let
  secretKey = "0000000000000009000000000000000900000000000000090000000000000009";
  cookie = "npins-native-eval-test";

  # Minimal default.nix that returns an attrset of derivations.
  # This is what a real npins project would look like after
  # `let sources = import ./npins; pkgs = import sources.nixpkgs {};`
  # except we skip nixpkgs entirely and use raw derivation builtins.
  projectDefaultNix = pkgs.writeText "default.nix" ''
    {
      packages.x86_64-linux.default = derivation {
        name = "npins-eval-test-output";
        system = "x86_64-linux";
        builder = "/bin/sh";
        args = [ "-c" "echo 'Built via snix-eval + npins (zero subprocesses)' > $out" ];
      };
    }
  '';

  # Empty npins sources.json — the default.nix above doesn't use npins
  # inputs, but npins/sources.json must exist for project type detection.
  projectSourcesJson = pkgs.writeText "sources.json" ''
    { "pins": {}, "version": 7 }
  '';

  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "kv";
        wasm = kvPluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "npins-native-eval";
    skipLint = true;
    skipTypeCheck = true;

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
          logLevel = "info,aspen_ci_executor_nix=debug";
          relayMode = "disabled";
          enableWorkers = true;
          enableCi = true;
          enableSnix = true;
          features = ["blob"];
        };

        environment.systemPackages = [
          aspenCliPackage
          gatewayPackage
          pkgs.bubblewrap
        ];

        networking.firewall.enable = false;
        virtualisation.memorySize = 4096;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      def get_ticket():
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/tmp/_cli_err.txt"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def plugin_cli(cmd, check=True):
          ticket = get_ticket()
          run = (
              f"aspen-plugin-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_plugin_cli_out.json 2>/tmp/_plugin_cli_err.txt"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_plugin_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      # ── boot ─────────────────────────────────────────────────────────
      start_all()
      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      # cluster init — use raw CLI (not --json) since init may not return JSON.
      ticket = get_ticket()
      node1.succeed(
          f"aspen-cli --ticket '{ticket}' cluster init >/dev/null 2>&1 || true"
      )
      time.sleep(2)

      # ── install KV plugin ────────────────────────────────────────────
      with subtest("install plugins"):
          plugin_cli(
              "plugin install /etc/aspen-plugins/kv-plugin.wasm "
              "--manifest /etc/aspen-plugins/kv-plugin.json"
          )
          plugin_cli("plugin reload", check=False)
          time.sleep(8)

      # ── create npins project ─────────────────────────────────────────
      with subtest("create npins project"):
          node1.succeed("mkdir -p /root/npins-project/npins")
          node1.succeed("cp ${projectDefaultNix} /root/npins-project/default.nix")
          node1.succeed("cp ${projectSourcesJson} /root/npins-project/npins/sources.json")
          node1.succeed("ls -la /root/npins-project/")
          node1.succeed("ls -la /root/npins-project/npins/")
          node1.succeed("cat /root/npins-project/default.nix")

      # ── submit build job ─────────────────────────────────────────────
      with subtest("submit npins build job"):
          payload = json.dumps({
              "flake_url": "/root/npins-project",
              "attribute": "packages.x86_64-linux.default",
              "extra_args": [],
              "timeout_secs": 300,
              "sandbox": False,
              "should_upload_result": False,
              "publish_to_cache": True,
              "cache_outputs": [],
              "artifacts": [],
          })
          result = cli(f"job submit ci_nix_build '{payload}'")
          job_id = result.get("job_id") or result.get("id")
          assert job_id, f"no job_id: {result}"
          node1.log(f"Submitted npins build job: {job_id}")

      # ── wait for completion ──────────────────────────────────────────
      with subtest("npins build completes"):
          deadline = time.time() + 120
          final_status = None
          while time.time() < deadline:
              result = cli(f"job status {job_id}", check=False)
              if isinstance(result, dict):
                  job = result.get("job") or result
                  state = job.get("status")
                  if state in ("success", "completed", "failed", "dead"):
                      final_status = result
                      break
              time.sleep(3)

          assert final_status is not None, f"Job did not complete within 120s"
          job = final_status.get("job") or final_status
          state = job.get("status")

          if state == "failed":
              node1.log(f"Job FAILED: {json.dumps(final_status, indent=2)[:3000]}")
              logs = node1.succeed(
                  "journalctl -u aspen-node.service --no-pager -n 50 "
                  "| grep -i 'npins\\|native\\|eval\\|build' || true"
              )
              node1.log(f"Relevant logs:\n{logs}")

          assert state in ("success", "completed"), f"Job failed: state={state}"
          node1.log("npins build job completed!")

      # ── verify zero-subprocess path ──────────────────────────────────
      with subtest("verify zero-subprocess native path"):
          logs = node1.succeed(
              "journalctl -u aspen-node.service --no-pager "
              "| grep -i 'zero subprocesses' || true"
          )
          node1.log(f"Zero-subprocess log lines:\n{logs}")
          assert "zero subprocesses" in logs, \
              f"Expected 'zero subprocesses' in logs — npins eval path not used"

      node1.log("All npins-native-eval tests passed!")
    '';
  }
