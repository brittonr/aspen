# Dogfood NixOS VM test: build Aspen's own Rust code through the CI pipeline.
#
# Pushes a standalone Rust crate (Tiger Style constants extracted from Aspen)
# to Forge, CI auto-triggers, NixBuildWorker compiles it with rustc, and the
# test runs the resulting binary to verify correctness.
#
# This is the self-hosting proof: Aspen compiles its own Rust code.
#
#   1. Create a Forge repo
#   2. Push a Rust crate with a Nix flake via git-remote-aspen
#   3. CI auto-triggers a nix build job
#   4. NixBuildWorker runs `nix build .#default` → rustc compiles the crate
#   5. Verify pipeline succeeds and the binary runs correctly
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-self-build-test --impure --option sandbox false
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
  secretKey = "0000000000000006000000000000000600000000000000060000000000000006";
  cookie = "ci-self-build-test";

  # CI config: single stage, one nix build job.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "aspen-self-build",
      stages = [
        {
          name = "build",
          jobs = [
            {
              name = "compile-aspen-rust",
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

  # Nix flake that compiles a Rust binary using stdenv + rustc.
  # Uses the same nixpkgs as the host, resolved via nix registry.
  selfBuildFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen self-build — compile Aspen Tiger Style Rust code via CI";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { nixpkgs, ... }:
        let pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in {
          default = pkgs.stdenv.mkDerivation {
            pname = "aspen-self-build";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = [ pkgs.rustc ];
            buildPhase = "rustc --edition 2021 src/main.rs -o aspen-self-build";
            installPhase = "mkdir -p $out/bin && cp aspen-self-build $out/bin/";
          };
        };
    }
  '';

  # Real Aspen code: Tiger Style resource bounds and compile-time assertions.
  # Extracted from crates/aspen-constants/src/lib.rs — this IS Aspen.
  selfBuildMain = pkgs.writeText "main.rs" ''
    // Tiger Style resource bounds — extracted from Aspen
    // See: crates/aspen-constants/src/lib.rs

    // Raft + KV limits
    const MAX_BATCH_SIZE: u32 = 1_000;
    const MAX_SCAN_RESULTS: u32 = 10_000;
    const MAX_KEY_SIZE: u32 = 1_024;
    const MAX_VALUE_SIZE: u32 = 1_048_576;

    // Network limits
    const MAX_PEERS: u32 = 1_000;
    const MAX_CONCURRENT_CONNECTIONS: u32 = 500;
    const CONNECT_TIMEOUT_MS: u64 = 5_000;
    const READ_TIMEOUT_MS: u64 = 10_000;

    // CI limits
    const MAX_PIPELINE_STAGES: u32 = 20;
    const MAX_JOBS_PER_STAGE: u32 = 50;
    const MAX_JOB_TIMEOUT_SECS: u64 = 7_200;

    // Compile-time Tiger Style assertions — verified at build time
    const _: () = assert!(MAX_BATCH_SIZE > 0);
    const _: () = assert!(MAX_BATCH_SIZE <= MAX_SCAN_RESULTS);
    const _: () = assert!(MAX_VALUE_SIZE >= MAX_KEY_SIZE);
    const _: () = assert!(MAX_PEERS > 0);
    const _: () = assert!(MAX_CONCURRENT_CONNECTIONS <= MAX_PEERS);
    const _: () = assert!(CONNECT_TIMEOUT_MS > 0);
    const _: () = assert!(READ_TIMEOUT_MS > CONNECT_TIMEOUT_MS);
    const _: () = assert!(MAX_PIPELINE_STAGES > 0);
    const _: () = assert!(MAX_JOBS_PER_STAGE > 0);
    const _: () = assert!(MAX_JOB_TIMEOUT_SECS > 0);

    fn main() {
        // Runtime assertions (Tiger Style: assert at every layer)
        assert!(MAX_BATCH_SIZE <= MAX_SCAN_RESULTS, "batch must fit in scan");
        assert!(MAX_KEY_SIZE < MAX_VALUE_SIZE, "key smaller than value");
        assert!(
            MAX_CONCURRENT_CONNECTIONS <= MAX_PEERS,
            "connections bounded by peers"
        );

        println!("=== Aspen Self-Build Verification ===");
        println!("MAX_BATCH_SIZE       = {}", MAX_BATCH_SIZE);
        println!("MAX_SCAN_RESULTS     = {}", MAX_SCAN_RESULTS);
        println!("MAX_KEY_SIZE         = {} bytes", MAX_KEY_SIZE);
        println!("MAX_VALUE_SIZE       = {} bytes", MAX_VALUE_SIZE);
        println!("MAX_PEERS            = {}", MAX_PEERS);
        println!("MAX_CONNECTIONS      = {}", MAX_CONCURRENT_CONNECTIONS);
        println!("CONNECT_TIMEOUT      = {} ms", CONNECT_TIMEOUT_MS);
        println!("READ_TIMEOUT         = {} ms", READ_TIMEOUT_MS);
        println!("MAX_PIPELINE_STAGES  = {}", MAX_PIPELINE_STAGES);
        println!("MAX_JOBS_PER_STAGE   = {}", MAX_JOBS_PER_STAGE);
        println!("All Tiger Style assertions passed");
        println!("Built by Aspen CI");
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
    name = "ci-dogfood-self-build";
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

      # The inner nix build downloads rustc (~1.6GB) + stdenv (~2GB) from
      # cache.nixos.org. The default writableStoreUseTmpfs=true limits the
      # writable store overlay to ~50% of RAM (not enough for rustc).
      # Using disk-backed storage instead gives us the full diskSize.
      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
      virtualisation.diskSize = 20480;
      virtualisation.writableStoreUseTmpfs = false;
    };

    testScript = ''
      import json, time

      SELF_BUILD_FLAKE = "${selfBuildFlake}"
      SELF_BUILD_MAIN = "${selfBuildMain}"
      CI_CONFIG = "${ciConfig}"

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

      def wait_for_pipeline(run_id, timeout=600):
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

      # ── verify nix works inside VM ─────────────────────────────
      with subtest("nix available"):
          node1.succeed("nix --version")
          node1.log("Nix is available in VM")

      # ── create forge repo ────────────────────────────────────────
      with subtest("create forge repo"):
          out = cli("git init aspen-self-build")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push Aspen Rust code to forge ────────────────────────────
      with subtest("push Aspen Rust crate to forge"):
          node1.succeed(
              "mkdir -p /tmp/self-build-repo/.aspen /tmp/self-build-repo/src && "
              f"cp {SELF_BUILD_FLAKE} /tmp/self-build-repo/flake.nix && "
              f"cp {SELF_BUILD_MAIN} /tmp/self-build-repo/src/main.rs && "
              f"cp {CI_CONFIG} /tmp/self-build-repo/.aspen/ci.ncl && "
              "cd /tmp/self-build-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'aspen self-build: tiger style constants'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/self-build-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          node1.log("Pushed Aspen Rust crate to Forge")

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

      with subtest("pipeline succeeds — Aspen Rust code compiles"):
          final = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Final: {json.dumps(final, indent=2)}")
          status = final.get("status")

          if status == "failed":
              for stage in final.get("stages", []):
                  for job in stage.get("jobs", []):
                      jid = job.get("id", "")
                      if job.get("status") == "failed" and jid:
                          ticket = get_ticket()
                          node1.execute(
                              f"aspen-cli --ticket '{ticket}' ci logs {run_id} {jid} "
                              f">/tmp/fail.log 2>/dev/null"
                          )
                          logs = node1.succeed("tail -80 /tmp/fail.log 2>/dev/null || echo 'no logs'")
                          node1.log(f"Job '{job.get('name')}' logs:\n{logs}")

          assert status == "success", f"Pipeline failed: {status}: {final}"

      # ── verify build logs ────────────────────────────────────────
      with subtest("ci logs captured"):
          all_jobs = []
          for stage in final.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append(job)

          assert len(all_jobs) >= 1
          for job in all_jobs:
              assert job["status"] == "success", f"Job '{job.get('name')}' not success: {job['status']}"

              ticket = get_ticket()
              node1.execute(
                  f"aspen-cli --ticket '{ticket}' ci logs {run_id} {job['id']} "
                  f">/tmp/job-{job['id']}.log 2>/dev/null"
              )
              log = node1.succeed(f"cat /tmp/job-{job['id']}.log 2>/dev/null || echo empty").strip()
              assert log, f"No log output for job '{job.get('name')}'"
              node1.log(f"Job '{job.get('name')}': {len(log)} bytes of logs")

      with subtest("ci output stored"):
          job = all_jobs[-1]
          out = cli(f"ci output {run_id} {job['id']}", check=False)
          assert isinstance(out, dict), f"Expected dict: {out}"
          assert out.get("was_found"), f"ci output not found for job '{job.get('name')}'"

      # ── run the CI-built binary ──────────────────────────────────
      with subtest("run CI-built aspen-self-build binary"):
          # Try to extract output path from job result
          job = all_jobs[-1]
          job_data = cli(f"kv get __jobs:{job['id']}", check=False)
          node1.log(f"Job KV data type: {type(job_data)}")

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
              binary = f"{output_path}/bin/aspen-self-build"
              node1.log(f"Looking for binary at: {binary}")
              node1.succeed(f"test -x {binary}")
              node1.log("Binary exists and is executable")

              output = node1.succeed(f"{binary}")
              node1.log(f"Binary output:\n{output}")
              assert "All Tiger Style assertions passed" in output, \
                  f"Tiger Style assertions missing from output: {output}"
              assert "Built by Aspen CI" in output, \
                  f"Build attribution missing from output: {output}"
              assert "MAX_BATCH_SIZE" in output, \
                  f"Constants missing from output: {output}"
              node1.log("Aspen self-build binary runs correctly!")
          else:
              # Fallback: scan nix store for the binary
              node1.log("No output_path in job result, scanning nix store...")
              output = node1.succeed(
                  "for p in /nix/store/*aspen-self-build*; do "
                  "  if [ -x \"$p/bin/aspen-self-build\" ]; then "
                  "    exec \"$p/bin/aspen-self-build\"; "
                  "  fi; "
                  "done; echo 'ERROR: binary not found'; exit 1"
              )
              node1.log(f"Binary output:\n{output}")
              assert "Built by Aspen CI" in output, \
                  f"Build attribution missing from output: {output}"

      node1.log("SELF-BUILD PASSED: Forge -> CI -> rustc compile -> run Aspen binary")
    '';
  }
