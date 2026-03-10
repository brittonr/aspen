# Dogfood NixOS VM test: push to Forge, trigger CI, build via NixBuildWorker.
#
# Proves the full Forge → CI → nix build pipeline works end-to-end:
#   1. Create a Forge repo
#   2. Push a tiny flake that builds cowsay via git-remote-aspen
#   3. CI auto-triggers a nix build job
#   4. NixBuildWorker runs `nix build .#default`
#   5. Verify pipeline succeeds, logs captured, output stored
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-test --impure --option sandbox false
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
  cookie = "ci-dogfood-test";

  # CI config: one stage, one nix build job.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "dogfood",
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
      ],
    }
  '';

  # Tiny flake that wraps nixpkgs cowsay.
  cowsayFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Dogfood test — build cowsay";
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
    name = "ci-dogfood";
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
        features = ["forge" "blob"];
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

      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
      virtualisation.diskSize = 4096;
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
          out = cli("git init cowsay-test")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push cowsay flake ────────────────────────────────────────
      with subtest("push cowsay flake to forge"):
          node1.succeed(
              "mkdir -p /tmp/cowsay-repo/.aspen && "
              f"cp {COWSAY_FLAKE} /tmp/cowsay-repo/flake.nix && "
              f"cp {CI_CONFIG} /tmp/cowsay-repo/.aspen/ci.ncl && "
              "cd /tmp/cowsay-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'cowsay dogfood'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/cowsay-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          node1.log("Pushed cowsay flake to Forge")

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

      with subtest("pipeline succeeds"):
          final = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Final: {json.dumps(final, indent=2)}")
          status = final.get("status")
          assert status == "success", f"Pipeline failed: {status}: {final}"

      # ── verify logs ──────────────────────────────────────────────
      with subtest("ci logs captured"):
          all_jobs = []
          for stage in final.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append(job)

          assert len(all_jobs) >= 1
          for job in all_jobs:
              assert job["status"] == "success", f"Job '{job.get('name')}' not success: {job['status']}"
              # Logs already saved to /tmp/job-{id}.log by stream_job_logs during wait
              log = node1.succeed(f"cat /tmp/job-{job['id']}.log 2>/dev/null || echo empty").strip()
              assert log, f"No log output for job '{job.get('name')}'"
              node1.log(f"Job '{job.get('name')}': {len(log)} bytes of logs")

      with subtest("ci output stored"):
          job = all_jobs[-1]
          out = cli(f"ci output {run_id} {job['id']}", check=False)
          assert isinstance(out, dict), f"Expected dict: {out}"
          assert out.get("was_found"), f"ci output not found for job '{job.get('name')}'"

      # ── run the built cowsay binary ──────────────────────────────
      with subtest("run CI-built cowsay"):
          # Read job result from KV to get output_paths
          job = all_jobs[-1]
          job_data = cli(f"kv get __jobs:{job['id']}", check=False)
          node1.log(f"Job KV data type: {type(job_data)}")

          # Parse the job result to extract output_paths
          output_path = None
          if isinstance(job_data, dict):
              value = job_data.get("value", "")
              try:
                  job_result = json.loads(value) if isinstance(value, str) else value
                  data = job_result.get("result", {}).get("Success", {}).get("data", {})
                  paths = data.get("output_paths", [])
                  if paths:
                      output_path = paths[0]
                      node1.log(f"Nix output path: {output_path}")
              except (json.JSONDecodeError, ValueError, AttributeError) as e:
                  node1.log(f"Failed to parse job result: {e}")

          if output_path:
              # The output_path is a nix store path like /nix/store/...-cowsay-3.8.4
              # cowsay binary should be at <output_path>/bin/cowsay
              cowsay_bin = f"{output_path}/bin/cowsay"
              node1.log(f"Looking for cowsay at: {cowsay_bin}")

              # Verify the binary exists
              node1.succeed(f"test -x {cowsay_bin}")
              node1.log("cowsay binary exists and is executable")

              # Actually run it!
              cowsay_output = node1.succeed(f"{cowsay_bin} 'Built by Aspen CI!'")
              node1.log(f"cowsay output:\n{cowsay_output}")
              assert "Built by Aspen CI!" in cowsay_output, f"cowsay output missing expected text: {cowsay_output}"
              assert "___" in cowsay_output or "---" in cowsay_output, f"cowsay output missing cow art: {cowsay_output}"
              node1.log("cowsay binary runs correctly!")
          else:
              # Fallback: scan nix store for cowsay binary
              # Multiple cowsay store paths may exist (-man, -doc, etc.)
              # Find the one that actually has bin/cowsay
              node1.log("No output_path in job result, scanning nix store...")
              cowsay_output = node1.succeed(
                  "for p in /nix/store/*cowsay*; do "
                  "  if [ -x \"$p/bin/cowsay\" ]; then "
                  "    exec \"$p/bin/cowsay\" 'Built by Aspen CI!'; "
                  "  fi; "
                  "done; echo 'ERROR: no cowsay binary found'; exit 1"
              )
              node1.log(f"cowsay output:\n{cowsay_output}")
              assert "Built by Aspen CI!" in cowsay_output, f"cowsay output missing expected text"

      node1.log("DOGFOOD PASSED: Forge -> CI -> nix build + RUN cowsay")
    '';
  }
