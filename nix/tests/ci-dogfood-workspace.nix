# Dogfood NixOS VM test: build a multi-crate Rust WORKSPACE through the CI pipeline.
#
# Pushes 3 real Aspen crates as a Cargo workspace to Forge:
#   - aspen-constants (2,600 lines, zero deps)
#   - aspen-hlc       (425 lines, deps: uhlc, blake3, serde, rand)
#   - aspen-kv-types  (1,493 lines, deps: aspen-constants, serde, thiserror)
#
# This proves what ci-dogfood-self-build could not:
#   - Cargo workspace resolution (3 members, path deps)
#   - crates.io dependency fetching (53 external packages)
#   - Native code compilation (blake3 has C/ASM backends)
#   - Cross-crate path deps (kv-types → constants)
#   - 93 unit tests across 3 crates via `doCheck = true`
#
# Pipeline: 2 stages (cargo check → build + test), same structure as
# ci-dogfood-self-build but with real dependency resolution.
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-workspace-test --impure --option sandbox false
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  secretKey = "0000000000000006000000000000000600000000000000060000000000000006";
  cookie = "ci-workspace-build-test";

  # Real source trees from the Aspen workspace.
  aspenConstantsSrc = ../../crates/aspen-constants/src;
  aspenHlcSrc = ../../crates/aspen-hlc/src;
  aspenKvTypesSrc = ../../crates/aspen-kv-types/src;

  # CI config: 2-stage pipeline — cargo check, then full build + test.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "aspen-workspace-build",
      stages = [
        {
          name = "check",
          jobs = [
            {
              name = "cargo-check",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.cargo-check",
              isolation = 'none,
              timeout_secs = 600,
            },
          ],
        },
        {
          name = "build",
          depends_on = ["check"],
          jobs = [
            {
              name = "build-and-test",
              type = 'nix,
              flake_url = ".",
              flake_attr = "packages.x86_64-linux.default",
              isolation = 'none,
              timeout_secs = 600,
            },
          ],
        },
      ],
    }
  '';

  # Nix flake for the workspace build.
  # Both outputs use buildRustPackage for proper Cargo vendoring —
  # raw `cargo check` fails because nix sandbox blocks crates.io HTTPS.
  workspaceFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen workspace — multi-crate self-build verification";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { nixpkgs, self, ... }:
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in {
          packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
            pname = "aspen-workspace";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            doCheck = true;
          };
          checks.x86_64-linux.cargo-check = pkgs.rustPlatform.buildRustPackage {
            pname = "aspen-workspace-check";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildPhase = "cargo check --workspace 2>&1";
            installPhase = "touch $out";
            doCheck = false;
          };
        };
    }
  '';

  # Root Cargo.toml: workspace with 3 members + binary target.
  rootCargoToml = pkgs.writeText "Cargo.toml" ''
    [workspace]
    members = ["aspen-constants", "aspen-kv-types", "aspen-hlc"]
    resolver = "3"

    [package]
    name = "aspen-workspace"
    version = "0.1.0"
    edition = "2024"
    description = "Multi-crate workspace build verification for Aspen CI"

    [[bin]]
    name = "aspen-workspace-check"
    path = "src/main.rs"

    [dependencies]
    aspen-constants = { path = "aspen-constants" }
    aspen-kv-types = { path = "aspen-kv-types" }
    aspen-hlc = { path = "aspen-hlc" }
  '';

  # Pre-generated Cargo.lock with 56 packages (3 workspace + 53 external).
  # Generated from the real crate sources with `cargo generate-lockfile`.
  workspaceCargoLock = ../../nix/tests/fixtures/workspace-build-cargo.lock;

  # Binary that exercises all 3 crates and their cross-crate interactions.
  workspaceMain = pkgs.writeText "main.rs" ''
    use aspen_constants::{api, ci, coordination, network, raft};
    use aspen_hlc::{create_hlc, new_timestamp, to_unix_ms};
    use aspen_kv_types::{ReadRequest, WriteRequest, WriteCommand, ScanRequest, KeyValueWithRevision};

    fn main() {
        println!("=== Aspen Workspace Self-Build ===");
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();

        // aspen-constants: Tiger Style resource bounds
        println!("[aspen-constants]");
        println!("  MAX_KEY_SIZE       = {} bytes", api::MAX_KEY_SIZE);
        println!("  MAX_VALUE_SIZE     = {} bytes", api::MAX_VALUE_SIZE);
        println!("  MAX_BATCH_SIZE     = {}", raft::MAX_BATCH_SIZE);
        println!("  MAX_PEERS          = {}", network::MAX_PEERS);
        println!("  MAX_CI_VMS         = {}", ci::MAX_CI_VMS_PER_NODE);
        println!();

        // aspen-hlc: HLC timestamps with blake3 node ID hashing
        println!("[aspen-hlc]");
        let hlc = create_hlc("test-node-1");
        let ts1 = new_timestamp(&hlc);
        let ts2 = new_timestamp(&hlc);
        assert!(ts2 > ts1, "HLC timestamps must be monotonically increasing");
        let ms = to_unix_ms(&ts1);
        assert!(ms > 0, "Unix ms must be positive");
        println!("  HLC timestamps:    monotonic");
        println!("  ts1 unix_ms:       {ms}");
        println!("  ts2 > ts1:         true");
        println!();

        // aspen-kv-types: cross-crate types using constants for validation
        println!("[aspen-kv-types]");
        let write = WriteRequest {
            command: WriteCommand::Set {
                key: "test-key".to_string(),
                value: "test-value".to_string(),
            },
        };
        let read = ReadRequest::new("test-key");
        let scan = ScanRequest {
            prefix: "test-".to_string(),
            limit_results: Some(api::DEFAULT_SCAN_LIMIT),
            continuation_token: None,
        };
        let kv = KeyValueWithRevision {
            key: "test-key".to_string(),
            value: "test-value".to_string(),
            version: 1,
            create_revision: 1,
            mod_revision: 1,
        };
        println!("  WriteRequest:      {:?}", write.command);
        println!("  ReadRequest:       key={}", read.key);
        println!("  ScanRequest:       prefix={}, limit={:?}", scan.prefix, scan.limit_results);
        println!("  KV entry:          {}={} (v{})", kv.key, kv.value, kv.version);
        println!();

        // Cross-crate assertions
        assert!(api::MAX_KEY_SIZE > 0);
        assert!(api::DEFAULT_SCAN_LIMIT <= api::MAX_SCAN_RESULTS);
        assert!(coordination::MAX_CAS_RETRIES > 0);

        println!("3 crates, 53 external deps, all assertions passed");
        println!("Built by Aspen CI");
    }
  '';

  # Bundle the complete workspace as a single derivation.
  workspaceRepo = pkgs.runCommand "aspen-workspace-repo" {} ''
    mkdir -p $out/src $out/.aspen
    mkdir -p $out/aspen-constants/src
    mkdir -p $out/aspen-hlc/src
    mkdir -p $out/aspen-kv-types/src

    # ── aspen-constants (11 source files, 2600+ lines) ──
    cp ${aspenConstantsSrc}/lib.rs         $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/api.rs         $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/assertions.rs  $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/ci.rs          $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/coordination.rs $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/directory.rs   $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/network.rs     $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/plugin.rs      $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/proxy.rs       $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/raft.rs        $out/aspen-constants/src/
    cp ${aspenConstantsSrc}/wasm.rs        $out/aspen-constants/src/
    cp ${../../crates/aspen-constants/Cargo.toml} $out/aspen-constants/Cargo.toml

    # ── aspen-hlc (1 source file, 425 lines, 12 tests) ──
    cp ${aspenHlcSrc}/lib.rs               $out/aspen-hlc/src/
    cp ${../../crates/aspen-hlc/Cargo.toml} $out/aspen-hlc/Cargo.toml

    # ── aspen-kv-types (7 source files, 1493 lines, 66 tests) ──
    cp ${aspenKvTypesSrc}/lib.rs           $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/batch.rs         $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/read.rs          $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/scan.rs          $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/transaction.rs   $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/validation.rs    $out/aspen-kv-types/src/
    cp ${aspenKvTypesSrc}/write.rs         $out/aspen-kv-types/src/
    cp ${../../crates/aspen-kv-types/Cargo.toml} $out/aspen-kv-types/Cargo.toml

    # ── root workspace ──
    cp ${workspaceMain}       $out/src/main.rs
    cp ${rootCargoToml}       $out/Cargo.toml
    cp ${workspaceCargoLock}  $out/Cargo.lock
    cp ${workspaceFlake}      $out/flake.nix
    cp ${ciConfig}            $out/.aspen/ci.ncl
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
    name = "ci-dogfood-workspace";
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
        enableSnix = true;
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
      virtualisation.diskSize = 20480;
      virtualisation.writableStoreUseTmpfs = false;
    };

    testScript = ''
      import json, time

      WORKSPACE_REPO = "${workspaceRepo}"

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

      def dump_failed_logs(final, run_id):
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

      # ── create forge repo ────────────────────────────────────────
      with subtest("create forge repo"):
          out = cli("git init aspen-workspace")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push workspace to forge ──────────────────────────────────
      with subtest("push 3-crate workspace to forge"):
          node1.succeed(
              f"cp -r --no-preserve=mode {WORKSPACE_REPO} /tmp/workspace-repo && "
              "cd /tmp/workspace-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'aspen workspace: constants + hlc + kv-types'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/workspace-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          # Verify workspace structure
          rs_count = node1.succeed(
              "find /tmp/workspace-repo -name '*.rs' | wc -l"
          ).strip()
          toml_count = node1.succeed(
              "find /tmp/workspace-repo -name 'Cargo.toml' | wc -l"
          ).strip()
          node1.log(f"Pushed {rs_count} .rs files, {toml_count} Cargo.toml files")
          assert int(rs_count) >= 20, f"Expected >=20 .rs files, got {rs_count}"
          assert int(toml_count) >= 4, f"Expected >=4 Cargo.toml (root + 3 crates), got {toml_count}"

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

      with subtest("2-stage pipeline succeeds"):
          final = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Final: {json.dumps(final, indent=2)}")
          status = final.get("status")

          if status == "failed":
              dump_failed_logs(final, run_id)

          assert status == "success", f"Pipeline failed: {status}: {final}"

      # ── verify both stages ───────────────────────────────────────
      with subtest("both stages completed"):
          stages = final.get("stages", [])
          assert len(stages) == 2, f"Expected 2 stages, got {len(stages)}: {stages}"

          stage_names = [s["name"] for s in stages]
          assert "check" in stage_names, f"Missing 'check' stage: {stage_names}"
          assert "build" in stage_names, f"Missing 'build' stage: {stage_names}"

          for stage in stages:
              assert stage["status"] == "success", \
                  f"Stage '{stage['name']}' status={stage['status']}, expected success"

      # ── verify build logs ────────────────────────────────────────
      with subtest("ci logs captured"):
          all_jobs = []
          for stage in final.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append(job)

          assert len(all_jobs) == 2, f"Expected 2 jobs, got {len(all_jobs)}"
          for job in all_jobs:
              assert job["status"] == "success", \
                  f"Job '{job.get('name')}' not success: {job['status']}"

              ticket = get_ticket()
              node1.execute(
                  f"aspen-cli --ticket '{ticket}' ci logs {run_id} {job['id']} "
                  f">/tmp/job-{job['id']}.log 2>/dev/null"
              )
              log = node1.succeed(f"cat /tmp/job-{job['id']}.log 2>/dev/null || echo empty").strip()
              assert log, f"No log output for job '{job.get('name')}'"
              node1.log(f"Job '{job.get('name')}': {len(log)} bytes of logs")

      # ── extract build job results ───────────────────────────────
      with subtest("extract job results"):
          build_job = [j for j in all_jobs if j.get("name") == "build-and-test"][0]
          job_data = cli(f"kv get __jobs:{build_job['id']}", check=False)

          output_path = None
          job_result_data = None
          if isinstance(job_data, dict):
              value = job_data.get("value", "")
              try:
                  job_result = json.loads(value) if isinstance(value, str) else value
                  job_result_data = job_result.get("result", {}).get("Success", {}).get("data", {})
                  paths = job_result_data.get("output_paths", [])
                  if paths:
                      output_path = paths[0]
              except (json.JSONDecodeError, ValueError, AttributeError) as e:
                  node1.log(f"Failed to parse job result: {e}")

          assert output_path, f"No output_path in job result: {job_data}"
          node1.log(f"Nix output path: {output_path}")

      # ── verify blob upload ──────────────────────────────────────
      with subtest("build output uploaded to blobs"):
          uploaded = job_result_data.get("uploaded_store_paths", [])
          assert len(uploaded) >= 1, f"No uploaded store paths: {job_result_data}"

          build_upload = [u for u in uploaded if "aspen-workspace-0.1.0" in u.get("store_path", "")]
          assert len(build_upload) == 1, f"Expected 1 build upload, got {len(build_upload)}: {uploaded}"

          blob_hash = build_upload[0].get("blob_hash")
          nar_size = build_upload[0].get("nar_size", 0)
          cache_registered = build_upload[0].get("cache_registered", False)

          assert blob_hash, f"No blob_hash in upload: {build_upload[0]}"
          assert nar_size > 0, f"NAR size is 0: {build_upload[0]}"
          assert cache_registered, f"Store path not registered in cache: {build_upload[0]}"

          node1.log(f"Blob upload: hash={blob_hash} nar_size={nar_size} cached={cache_registered}")

      # ── verify nix binary cache entry ────────────────────────────
      with subtest("build output in nix binary cache"):
          cache_result = cli(f"cache query {output_path}", check=False)
          node1.log(f"Cache query result: {json.dumps(cache_result, indent=2)}")

          assert isinstance(cache_result, dict), f"Cache query failed: {cache_result}"
          assert cache_result.get("was_found"), f"Store path not found in cache: {cache_result}"
          assert cache_result.get("blob_hash") == blob_hash, \
              f"Blob hash mismatch: cache={cache_result.get('blob_hash')} upload={blob_hash}"
          assert cache_result.get("nar_size") == nar_size, \
              f"NAR size mismatch: cache={cache_result.get('nar_size')} upload={nar_size}"

      # ── verify SNIX upload ──────────────────────────────────────
      with subtest("SNIX storage upload succeeded"):
          assert "uploaded_store_paths_snix" in job_result_data, \
              f"snix feature not compiled — missing uploaded_store_paths_snix: {job_result_data.keys()}"

          snix_log = node1.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null | grep -c 'Uploading store path to SNIX' || echo 0"
          ).strip()
          snix_attempts = int(snix_log)
          assert snix_attempts >= 1, \
              f"SNIX upload was never attempted: attempts={snix_attempts}"
          node1.log(f"SNIX upload attempts: {snix_attempts}")

          snix_uploads = job_result_data.get("uploaded_store_paths_snix", [])
          assert len(snix_uploads) >= 1, f"No SNIX uploads completed: {job_result_data}"

          snix_build = [u for u in snix_uploads if "aspen-workspace-0.1.0" in u.get("store_path", "")]
          assert len(snix_build) == 1, \
              f"Expected 1 SNIX upload for aspen-workspace, got {len(snix_build)}: {snix_uploads}"

          snix_entry = snix_build[0]
          assert snix_entry.get("nar_size", 0) == nar_size, \
              f"SNIX/blob NAR size mismatch: snix={snix_entry.get('nar_size')} blob={nar_size}"
          assert snix_entry.get("nar_sha256"), f"Missing nar_sha256 in SNIX upload: {snix_entry}"
          node1.log(f"SNIX upload verified: {snix_entry}")

      # ── run the CI-built binary ──────────────────────────────────
      with subtest("run CI-built binary"):
          binary = f"{output_path}/bin/aspen-workspace-check"
          node1.succeed(f"test -x {binary}")

          output = node1.succeed(f"{binary}")
          node1.log(f"Binary output:\n{output}")

          # Verify all 3 crates were exercised
          assert "[aspen-constants]" in output, f"Missing constants section: {output}"
          assert "[aspen-hlc]" in output, f"Missing hlc section: {output}"
          assert "[aspen-kv-types]" in output, f"Missing kv-types section: {output}"

          # Verify HLC actually ran (not just compiled)
          assert "monotonic" in output, f"HLC monotonicity check missing: {output}"
          assert "ts2 > ts1" in output, f"HLC comparison missing: {output}"

          # Verify cross-crate KV types
          assert "WriteRequest:" in output, f"WriteRequest missing: {output}"
          assert "ReadRequest:" in output, f"ReadRequest missing: {output}"

          assert "53 external deps" in output, f"Dep count missing: {output}"
          assert "Built by Aspen CI" in output, f"Build attribution missing: {output}"

      node1.log("WORKSPACE BUILD PASSED: 3 crates, 53 deps, 93 tests — Forge -> CI -> nix build -> blob+cache+snix -> run")
    '';
  }
