# End-to-end NixOS VM test: git push a Nix flake → CI nix build → verify output.
#
# Exercises the Nix build path through the self-hosting pipeline:
#   1. Create a Forge repository
#   2. Watch the repo for CI triggers
#   3. Push a simple Nix flake with `type = 'nix` CI config
#   4. Verify CI auto-triggers and runs `nix build` via NixBuildWorker
#   5. Wait for pipeline completion and verify build output
#
# This proves that Aspen can build Nix flakes through its own CI,
# which is the foundation for building Aspen from its own infrastructure.
#
# Run:
#   nix build .#checks.x86_64-linux.ci-nix-build-test --impure
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.ci-nix-build-test.driverInteractive --impure
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
}: let
  # Deterministic Iroh secret key.
  secretKey = "0000000000000004000000000000000400000000000000040000000000000004";

  # Shared cluster cookie.
  cookie = "ci-nix-build-test";

  # A trivial Nix flake that can be built inside a NixOS VM without network.
  # It creates a simple text file derivation — no fetching, no IFDs.
  testFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen CI test flake";
      inputs = {};
      outputs = { self, ... }: {
        packages.x86_64-linux.default = derivation {
          name = "aspen-ci-test-output";
          system = "x86_64-linux";
          builder = "/bin/sh";
          args = [ "-c" "echo 'Built by Aspen CI' > $out" ];
        };
      };
    }
  '';

  # CI config: a single nix job that builds the default package.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "nix-build-test",
      stages = [
        {
          name = "build",
          jobs = [
            {
              name = "nix-build",
              type = 'nix,
              flake_url = ".",
              flake_attr = "packages.x86_64-linux.default",
              timeout_secs = 300,
            },
          ],
        },
      ],
    }
  '';

  # WASM plugin helpers (KV + Forge handlers are WASM-only)
  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "kv";
        wasm = kvPluginWasm;
      }
      {
        name = "forge";
        wasm = forgePluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "ci-nix-build";
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
          logLevel = "info";
          relayMode = "disabled";
          enableWorkers = true;
          enableCi = true;
          features = ["forge" "blob"];
        };

        environment.systemPackages = [
          aspenCliPackage
          gitRemoteAspenPackage
          pkgs.git
          pkgs.curl
          pkgs.nix
        ];

        # Enable Nix daemon so `nix build` works inside the VM
        nix.settings.experimental-features = ["nix-command" "flakes"];
        # Allow building without sandbox inside VM (nested sandbox not supported)
        nix.settings.sandbox = false;

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
              err = node1.succeed("cat /tmp/_cli_err.txt 2>/dev/null || true").strip()
              node1.log(f"cli() JSON parse failed, raw={raw!r}, stderr={err}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      def wait_for_pipeline(run_id, timeout=300):
          """Poll ci status until pipeline reaches a terminal state."""
          deadline = time.time() + timeout
          while time.time() < deadline:
              result = cli(f"ci status {run_id}", check=False)
              if isinstance(result, dict):
                  status = result.get("status")
                  node1.log(f"Pipeline {run_id}: status={status}")
                  if status in ("success", "failed", "cancelled"):
                      return result
              time.sleep(5)
          raise Exception(f"Pipeline {run_id} did not complete within {timeout}s")

      def plugin_cli(cmd, check=True):
          """Run aspen-plugin-cli with cluster ticket."""
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

      # ── install WASM plugins ─────────────────────────────────────

      with subtest("install WASM plugins"):
          for _pname, _pwasm, _pmanifest in [
              ("kv", "/etc/aspen-plugins/kv-plugin.wasm", "/etc/aspen-plugins/kv-plugin.json"),
              ("forge", "/etc/aspen-plugins/forge-plugin.wasm", "/etc/aspen-plugins/forge-plugin.json"),
          ]:
              out = plugin_cli(f"plugin install {_pwasm} --manifest {_pmanifest}")
              node1.log(f"installed {_pname} plugin: {out}")

      with subtest("reload plugin runtime"):
          out = plugin_cli("plugin reload", check=False)
          node1.log(f"plugin reload: {out}")
          time.sleep(8)

      # ── verify nix works inside VM ──────────────────────────────────

      with subtest("nix command available"):
          node1.succeed("nix --version")
          node1.log("Nix is available in VM")

      # ── phase 1: create forge repo ──────────────────────────────────

      with subtest("create forge repository"):
          out = cli("git init ci-nix-build-repo")
          node1.log(f"Repo create: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id in response: {out}"
          node1.log(f"Created repo: {repo_id}")

      # ── phase 2: watch repo for auto-trigger ────────────────────────

      with subtest("ci watch enables auto-trigger"):
          result = cli(f"ci watch {repo_id}")
          node1.log(f"CI watch: {result}")
          assert isinstance(result, dict), f"expected dict: {result}"
          assert result.get("is_success"), f"ci watch failed: {result}"

      # ── phase 3: push flake with nix CI config ─────────────────────

      with subtest("push nix flake to forge"):
          ticket = get_ticket()

          # Create local repo with flake + CI config
          node1.succeed("mkdir -p /tmp/test-repo/.aspen")
          node1.succeed("cp ${testFlake} /tmp/test-repo/flake.nix")
          node1.succeed("cp ${ciConfig} /tmp/test-repo/.aspen/ci.ncl")

          # Initialize and commit
          node1.succeed(
              "cd /tmp/test-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@aspen.local' && "
              "git config user.name 'CI Nix Build Test' && "
              "git add -A && "
              "git commit -m 'initial: nix flake with ci config'"
          )

          # Push to forge
          remote_url = f"aspen://{ticket}/{repo_id}"
          node1.succeed(
              f"cd /tmp/test-repo && "
              f"git remote add aspen '{remote_url}'"
          )

          exit_code, _ = node1.execute(
              "cd /tmp/test-repo && "
              "RUST_LOG=warn git push aspen main "
              ">/tmp/git-push-stdout.txt 2>/tmp/git-push-stderr.txt"
          )
          push_stderr = node1.succeed("cat /tmp/git-push-stderr.txt 2>/dev/null || true")
          node1.log(f"Push exit={exit_code}, stderr: {push_stderr}")
          assert exit_code == 0, f"git push failed (exit {exit_code}): {push_stderr}"

      # ── phase 4: wait for auto-triggered pipeline ───────────────────

      with subtest("auto-triggered nix pipeline appears"):
          deadline = time.time() + 90
          run_id = None
          while time.time() < deadline:
              list_out = cli("ci list", check=False)
              if isinstance(list_out, dict):
                  runs = list_out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      node1.log(f"Auto-triggered nix pipeline found: {run_id}")
                      break
              time.sleep(3)
          assert run_id, "No auto-triggered pipeline appeared within 90s"

      # ── phase 5: wait for nix build completion ──────────────────────
      # Nix builds are slower than shell commands — allow 5 minutes.

      with subtest("nix build pipeline completes successfully"):
          final_status = wait_for_pipeline(run_id, timeout=300)
          node1.log(f"Pipeline final: {json.dumps(final_status, indent=2)}")
          pipeline_status = final_status.get("status")

          if pipeline_status == "failed":
              # Log detailed failure info for debugging
              node1.log("Pipeline FAILED — checking job details")
              for stage in final_status.get("stages", []):
                  for j in stage.get("jobs", []):
                      node1.log(f"  Job '{j.get('name')}': {j.get('status')} - {j.get('error', 'no error')}")

          assert pipeline_status == "success", \
              f"Nix build pipeline failed with status={pipeline_status}: {final_status}"
          node1.log("Nix build pipeline completed successfully!")

          # Verify stage-level statuses are populated (not stuck at "pending")
          for stage in final_status.get("stages", []):
              stage_name = stage.get("name")
              stage_status = stage.get("status")
              node1.log(f"  Stage '{stage_name}': status={stage_status}")
              assert stage_status == "success", \
                  f"Stage '{stage_name}' has status '{stage_status}', expected 'success'"

      # ── phase 6: verify build produced output ───────────────────────

      with subtest("verify build output in ci status"):
          # The ci status nests jobs inside stages[].jobs[].
          # Find the nix-build job by traversing the stage/job hierarchy.
          nix_job = None
          for stage in final_status.get("stages", []):
              for j in stage.get("jobs", []):
                  if j.get("name") == "nix-build":
                      nix_job = j
                      break
              if nix_job:
                  break

          assert nix_job is not None, \
              f"Could not find 'nix-build' job in pipeline stages: {json.dumps(final_status.get('stages', []), indent=2)[:2000]}"
          node1.log(f"Found nix-build job: id={nix_job.get('id')}, status={nix_job.get('status')}")
          assert nix_job.get("status") == "success", \
              f"nix-build job status is '{nix_job.get('status')}', expected 'success'"

      # ── phase 7: verify ci list shows success ───────────────────────

      with subtest("ci list shows successful nix build"):
          result = cli("ci list", check=False)
          node1.log(f"CI list: {result}")
          if isinstance(result, dict):
              runs = result.get("runs", [])
              found = any(
                  r.get("run_id") == run_id and r.get("status") == "success"
                  for r in runs
              )
              assert found, f"run {run_id} not in list with status=success: {runs}"

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All ci-nix-build tests passed!")
    '';
  }
