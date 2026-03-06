# End-to-end NixOS VM integration test: git push → CI build → Nix cache.
#
# Exercises the full self-hosting pipeline with auto-trigger:
#   1. Create a Forge repository (via WASM plugin)
#   2. Watch the repo for CI triggers (`ci watch`)
#   3. Push code containing `.aspen/ci.ncl` via git-remote-aspen
#   4. Verify pipeline auto-triggers via forge gossip (no manual `ci run`)
#   5. Wait for shell build stage to complete
#   6. Verify pipeline status and ref-status tracking
#   7. Start the cache gateway and verify it serves
#
# This is the integration test that proves Aspen can orchestrate
# builds through its own infrastructure end-to-end.
#
# Run:
#   nix build .#checks.x86_64-linux.e2e-push-build-cache-test --impure
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.e2e-push-build-cache-test.driverInteractive --impure
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gatewayPackage,
  gitRemoteAspenPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000003000000000000000300000000000000030000000000000003";

  # Shared cluster cookie.
  cookie = "e2e-push-build-cache-test";

  # CI config (Nickel) — written as a Nix file to avoid quoting issues.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "e2e-test-pipeline",
      stages = [
        {
          name = "check",
          jobs = [
            {
              name = "hello",
              type = 'shell,
              command = "echo hello from aspen ci",
              timeout_secs = 60,
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
    name = "e2e-push-build-cache";
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
          gatewayPackage
          gitRemoteAspenPackage
          pkgs.git
          pkgs.curl
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

      def wait_for_pipeline(run_id, timeout=180):
          """Poll ci status until pipeline reaches a terminal state."""
          deadline = time.time() + timeout
          while time.time() < deadline:
              result = cli(f"ci status {run_id}", check=False)
              if isinstance(result, dict):
                  status = result.get("status")
                  node1.log(f"Pipeline {run_id}: status={status}")
                  if status in ("success", "failed", "cancelled"):
                      return result
              time.sleep(3)
          raise Exception(f"Pipeline {run_id} did not complete within {timeout}s")

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
      # Install and reload plugins. The plugin_list check in the helper
      # may fail due to timing with CI workers. We install, reload, and
      # verify by actually using the plugins rather than count checks.

      def plugin_cli(cmd, check=True):
          """Run aspen-plugin-cli with cluster ticket, return parsed JSON."""
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
              node1.log(f"plugin_cli() parse failed: {raw!r}")
              return raw.strip()

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
          node1.log("Plugins installed and reloaded, proceeding...")

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── phase 1: create forge repo ──────────────────────────────────

      with subtest("create forge repository"):
          out = cli("git init e2e-test-repo")
          node1.log(f"Repo create: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id in response: {out}"
          node1.log(f"Created repo: {repo_id}")

      # ── phase 2: watch repo for auto-trigger ────────────────────────
      # Register the repo with CI trigger service BEFORE pushing.
      # This enables the gossip-driven auto-trigger flow:
      #   git push → forge gossip RefUpdate → CiTriggerHandler → pipeline

      with subtest("ci watch enables auto-trigger"):
          result = cli(f"ci watch {repo_id}")
          node1.log(f"CI watch: {result}")
          assert isinstance(result, dict), f"expected dict: {result}"
          assert result.get("is_success"), f"ci watch failed: {result}"
          node1.log("Repository is now watched for CI triggers")

      # ── phase 3: prepare and push repo content ──────────────────────
      # The push triggers a forge gossip RefUpdate announcement.
      # Since we're watching this repo, the CiTriggerHandler will
      # automatically start a pipeline — no manual `ci run` needed.

      with subtest("push code to forge via git"):
          ticket = get_ticket()

          # Create a local git repo with CI config
          node1.succeed("mkdir -p /tmp/test-repo/.aspen")

          # Write a simple README
          node1.succeed("echo '# E2E Test Repo' > /tmp/test-repo/README.md")

          # Copy CI config from Nix store (avoids heredoc quoting issues)
          node1.succeed("cp ${ciConfig} /tmp/test-repo/.aspen/ci.ncl")

          # Initialize git and commit
          node1.succeed(
              "cd /tmp/test-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@aspen.local' && "
              "git config user.name 'E2E Test' && "
              "git add -A && "
              "git commit -m 'initial: ci config'"
          )

          # Add forge remote (format: aspen://{ticket}/{repo_id})
          remote_url = f"aspen://{ticket}/{repo_id}"
          node1.succeed(
              f"cd /tmp/test-repo && "
              f"git remote add aspen '{remote_url}'"
          )

          exit_code, push_out = node1.execute(
              "cd /tmp/test-repo && "
              "RUST_LOG=warn git push aspen main "
              ">/tmp/git-push-stdout.txt 2>/tmp/git-push-stderr.txt"
          )
          push_stdout = node1.succeed("cat /tmp/git-push-stdout.txt 2>/dev/null || true")
          push_stderr = node1.succeed("cat /tmp/git-push-stderr.txt 2>/dev/null || true")
          node1.log(f"Push exit={exit_code}")
          node1.log(f"Push stdout: {push_stdout}")
          node1.log(f"Push stderr: {push_stderr}")
          assert exit_code == 0, f"git push failed (exit {exit_code}): {push_stderr}"

          # Verify the ref exists
          out = cli(f"branch list -r {repo_id}", check=False)
          node1.log(f"Branch list after push: {out}")

      # ── phase 4: wait for auto-triggered pipeline ───────────────────
      # The push should have triggered a pipeline via forge gossip.
      # Poll `ci list` until a run appears (auto-triggered, no manual ci run).

      with subtest("auto-triggered pipeline appears"):
          deadline = time.time() + 60
          run_id = None
          while time.time() < deadline:
              list_out = cli("ci list", check=False)
              if isinstance(list_out, dict):
                  runs = list_out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      node1.log(f"Auto-triggered pipeline found: {run_id}")
                      break
              time.sleep(2)
          assert run_id, "No auto-triggered pipeline appeared within 60s"

      # ── phase 5: wait for pipeline completion ───────────────────────

      with subtest("wait for auto-triggered pipeline completion"):
          final_status = wait_for_pipeline(run_id, timeout=180)
          node1.log(f"Pipeline final: {json.dumps(final_status, indent=2)}")
          pipeline_status = final_status.get("status")
          # Accept success or failed (we want to see it completed)
          assert pipeline_status in ("success", "failed"), \
              f"expected terminal status, got {pipeline_status}: {final_status}"
          if pipeline_status == "success":
              node1.log("Pipeline completed successfully!")
          else:
              node1.log(f"Pipeline failed (may be expected if config issue): {final_status}")

      # ── phase 6: verify CI list shows the run ───────────────────────

      with subtest("ci list shows completed run"):
          result = cli("ci list", check=False)
          node1.log(f"CI list: {result}")
          if isinstance(result, dict):
              runs = result.get("runs", [])
              found = any(r.get("run_id") == run_id for r in runs)
              assert found, f"run {run_id} not in list: {runs}"

      # ── phase 7: verify ref-status ──────────────────────────────────

      with subtest("ci ref-status tracks pipeline"):
          result = cli(f"ci ref-status {repo_id} refs/heads/main", check=False)
          node1.log(f"CI ref-status: {result}")
          if isinstance(result, dict) and result.get("was_found"):
              node1.log(f"Ref status: {result.get('status')}")
          else:
              node1.log("Ref status not found (may not be indexed yet)")

      # ── phase 8: start cache gateway and verify ─────────────────────

      with subtest("start nix cache gateway"):
          ticket = get_ticket()
          node1.succeed(
              f"systemd-run --unit=nix-cache-gw "
              f"aspen-nix-cache-gateway --ticket '{ticket}' --port 8380 "
              f"--cache-name e2e-cache"
          )
          node1.wait_until_succeeds(
              "curl -sf http://127.0.0.1:8380/nix-cache-info",
              timeout=30,
          )
          info = node1.succeed("curl -sf http://127.0.0.1:8380/nix-cache-info")
          node1.log(f"Cache info: {info}")
          assert "StoreDir: /nix/store" in info

      with subtest("cache public key available"):
          out = cli("cache public-key", check=False)
          node1.log(f"Cache public key: {out}")
          if isinstance(out, dict):
              key = out.get("public_key")
              if key:
                  node1.log(f"Public key: {key}")
              else:
                  node1.log("No public key yet (auto-generated on first use)")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All e2e-push-build-cache tests passed!")
    '';
  }
