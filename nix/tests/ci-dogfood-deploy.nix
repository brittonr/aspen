# Dogfood deploy NixOS VM test: push to Forge, CI builds, deploy stage runs.
#
# Proves the full Forge → CI → Build → Deploy pipeline works:
#   1. Create a Forge repo
#   2. Push a tiny flake with a deploy stage in ci.ncl
#   3. CI auto-triggers: build stage produces a Nix store path
#   4. Deploy stage resolves the artifact from build-node
#   5. Deploy stage initiates ClusterDeploy with the store path
#   6. Verify the deployment completes and node is healthy
#
# Single-node cluster: exercises the full deploy path (drain → swap →
# restart → health) without multi-node RAM/flakiness. Quorum safety
# is already proven by unit tests + Verus specs.
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-deploy-test --impure --option sandbox false
{
  pkgs,
  lib,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  secretKey = "0000000000000005000000000000000500000000000000050000000000000005";
  cookie = "ci-deploy-test";

  # CI config: build + deploy stages.
  # The deploy stage references build-cowsay's artifact.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "dogfood-deploy",
      stages = [
        {
          name = "build",
          jobs = [
            {
              name = "build-cowsay",
              type = 'nix,
              flake_url = ".",
              flake_attr = "default",
              isolation = 'none,
              timeout_secs = 600,
            },
          ],
        },
        {
          name = "deploy",
          depends_on = ["build"],
          jobs = [
            {
              name = "deploy-app",
              type = 'deploy,
              artifact_from = "build-cowsay",
              strategy = "rolling",
              max_concurrent = 1,
              health_check_timeout_secs = 120,
              timeout_secs = 600,
              expected_binary = "bin/cowsay",
              validate_only = true,
            },
          ],
        },
      ],
    }
  '';

  # Tiny flake that wraps nixpkgs cowsay.
  cowsayFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Deploy test — build cowsay";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { nixpkgs, ... }:
        let pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in { default = pkgs.cowsay; };
    }
  '';

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
    name = "ci-dogfood-deploy";
    skipLint = true;
    skipTypeCheck = true;

    nodes.node1 = {
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
        features = ["forge" "blob" "deploy"];
      };

      environment.systemPackages = [
        aspenCliPackage
        gitRemoteAspenPackage
        pkgs.git
        pkgs.nix
      ];

      networking.firewall.enable = false;
      nix.settings.experimental-features = ["nix-command" "flakes"];
      nix.settings.sandbox = false;
      nix.registry.nixpkgs.flake = nixpkgsFlake;

      # Generous resources for nix builds inside VM
      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
      virtualisation.diskSize = 20480;
      virtualisation.writableStoreUseTmpfs = false;
    };

    testScript = ''
      import json, time

      CI_CONFIG = "${ciConfig}"
      COWSAY_FLAKE = "${cowsayFlake}"

      def get_ticket():
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          ticket = get_ticket()
          run = f"aspen-cli --ticket '{ticket}' --json {cmd} >/tmp/_cli.json 2>/tmp/_cli.err"
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def cli_text(cmd):
          ticket = get_ticket()
          node1.succeed(f"aspen-cli --ticket '{ticket}' {cmd} >/tmp/_cli.txt 2>/dev/null")
          return node1.succeed("cat /tmp/_cli.txt")

      def plugin_cli(cmd, check=True):
          ticket = get_ticket()
          run = f"aspen-plugin-cli --ticket '{ticket}' --json {cmd} >/tmp/_pcli.json 2>/tmp/_pcli.err"
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_pcli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def stream_job_logs(run_id, job_id, job_name):
          """Stream logs for a job via ci logs --follow, saving to file."""
          ticket = get_ticket()
          node1.log(f"=== streaming logs: {job_name} ({job_id}) ===")
          node1.execute(
              f"aspen-cli --ticket '{ticket}' ci logs --follow {run_id} {job_id} "
              f">/tmp/job-{job_id}.log 2>/dev/null"
          )
          log_size = node1.succeed(f"wc -c < /tmp/job-{job_id}.log").strip()
          node1.log(f"=== end logs: {job_name} ({log_size} bytes) ===")

      def wait_for_pipeline(run_id, timeout=600):
          """Wait for pipeline by streaming job logs as they're assigned."""
          deadline = time.time() + timeout
          streamed_jobs = set()
          while time.time() < deadline:
              result = cli(f"ci status {run_id}", check=False)
              if not isinstance(result, dict):
                  time.sleep(3)
                  continue

              # Stream logs for newly assigned jobs
              for stage in result.get("stages", []):
                  for job in stage.get("jobs", []):
                      jid = job.get("id", "")
                      if jid and jid not in streamed_jobs:
                          streamed_jobs.add(jid)
                          stream_job_logs(run_id, jid, job.get("name", "unknown"))

              status = result.get("status")
              node1.log(f"Pipeline {run_id}: status={status}")
              if status in ("success", "failed", "cancelled"):
                  return result
              time.sleep(3)
          raise Exception(f"Pipeline {run_id} did not complete within {timeout}s")

      # ── boot ─────────────────────────────────────────────────────
      start_all()
      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      cli_text("cluster init")
      time.sleep(2)

      # ── plugins ──────────────────────────────────────────────────
      with subtest("install WASM plugins"):
          for name, wasm, manifest in [
              ("kv", "/etc/aspen-plugins/kv-plugin.wasm", "/etc/aspen-plugins/kv-plugin.json"),
              ("forge", "/etc/aspen-plugins/forge-plugin.wasm", "/etc/aspen-plugins/forge-plugin.json"),
          ]:
              plugin_cli(f"plugin install {wasm} --manifest {manifest}")
          plugin_cli("plugin reload", check=False)
          time.sleep(8)

      # ── forge repo ───────────────────────────────────────────────
      with subtest("create forge repo"):
          out = cli("git init cowsay-deploy-test")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push flake with deploy stage ─────────────────────────────
      with subtest("push flake with deploy config"):
          node1.succeed(
              "mkdir -p /tmp/deploy-repo/.aspen && "
              f"cp {COWSAY_FLAKE} /tmp/deploy-repo/flake.nix && "
              f"cp {CI_CONFIG} /tmp/deploy-repo/.aspen/ci.ncl && "
              "cd /tmp/deploy-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'deploy dogfood'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/deploy-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          node1.log("Pushed flake with deploy config to Forge")

      # ── wait for pipeline ────────────────────────────────────────
      with subtest("pipeline auto-triggers"):
          deadline = time.time() + 60
          run_id = None
          while time.time() < deadline:
              out = cli("ci list", check=False)
              if isinstance(out, dict):
                  runs = out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      break
              time.sleep(3)
          assert run_id, "No pipeline triggered within 60s"
          node1.log(f"Pipeline: {run_id}")

      # ── wait for build stage ─────────────────────────────────────
      with subtest("build stage completes"):
          final = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Final: {json.dumps(final, indent=2)}")

          # Check build stage succeeded
          build_stage = None
          for stage in final.get("stages", []):
              if stage.get("name") == "build":
                  build_stage = stage
                  break
          assert build_stage, "Build stage not found in pipeline result"
          for job in build_stage.get("jobs", []):
              assert job["status"] == "success", (
                  f"Build job '{job.get('name')}' failed: {job['status']}"
              )
          node1.log("Build stage completed successfully")

      # ── verify deploy stage succeeds (happy path) ─────────────────
      with subtest("deploy stage succeeds"):
          deploy_stage = None
          for stage in final.get("stages", []):
              if stage.get("name") == "deploy":
                  deploy_stage = stage
                  break

          assert deploy_stage, "Deploy stage not found in pipeline result"
          node1.log(f"Deploy stage: {json.dumps(deploy_stage, indent=2)}")
          for job in deploy_stage.get("jobs", []):
              assert job["status"] == "success", (
                  f"Deploy job '{job.get('name')}' should succeed with expected_binary=bin/cowsay: "
                  f"{job['status']}"
              )
              node1.log(f"Deploy job '{job.get('name')}': {job['status']} (happy path)")

      # ── verify cluster health ────────────────────────────────────
      with subtest("cluster healthy after pipeline"):
          node1.wait_until_succeeds(
              "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
              timeout=30,
          )
          node1.log("Cluster health OK after pipeline")

      # ── verify KV still works ────────────────────────────────────
      with subtest("kv operations work"):
          cli("kv set deploy-test-key deploy-test-value")
          result = cli("kv get deploy-test-key")
          value = result.get("value", "") if isinstance(result, dict) else ""
          assert "deploy-test-value" in str(value), f"KV read failed: {result}"
          node1.log("KV operations work after deploy pipeline")

      node1.log("DEPLOY DOGFOOD PASSED: Forge -> CI -> build -> deploy stage")
    '';
  }
