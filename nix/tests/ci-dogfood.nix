# Dogfood NixOS VM test: push Aspen's source to Forge, COMPILE it via NixBuildWorker.
#
# This is the self-hosting litmus test. It:
#   1. Creates a Forge repository
#   2. Pushes Aspen's ACTUAL source tree (80+ crates, ~23MB) via git-remote-aspen
#   3. CI auto-triggers with 3-stage pipeline using `type = 'nix` jobs:
#      a. Validate: structural check (source tree has 70+ crates)
#      b. Build: `cargo check` on aspen-constants + aspen-time (zero-dep crates)
#      c. Test: `cargo check -p aspen-core` (real crate with vendored dependencies)
#   4. Verifies pipeline completion with all stages passing
#   5. Verifies NixBuildWorker log chunks are written to KV store
#
# Pre-staged in the VM (no network needed for builds):
#   - Rust nightly toolchain at /etc/aspen-ci/rust/
#   - Vendored cargo deps via cargoVendorDir (crane-built)
#   - Cargo config at /etc/aspen-ci/cargo-config.toml
#
# This proves Aspen can host its own source code AND compile real Rust
# through its own CI using the NixBuildWorker (the production executor path).
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-test --impure
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.ci-dogfood-test.driverInteractive --impure
#   ./result/bin/nixos-test-driver
{
  pkgs,
  lib,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
  aspenSource,
  rustToolChain,
  cargoVendorDir,
}: let
  # Deterministic Iroh secret key.
  secretKey = "0000000000000005000000000000000500000000000000050000000000000005";

  cookie = "ci-dogfood-test";

  # Crane's vendor dir has config.toml at root with absolute Nix store paths.
  # We use it directly — it already points to the vendored crates subdirectory
  # using full /nix/store/... paths, so no path rewriting needed.
  cargoConfig = "${cargoVendorDir}/config.toml";

  # Minimal workspace Cargo.toml for the cargo-check-core build.
  # Only includes aspen-core and its transitive path deps — avoids loading
  # workspace members that depend on git sources (aspen-net, aspen-proxy, etc.)
  # which would require vendoring git deps too.
  minimalWorkspaceToml = pkgs.writeText "minimal-workspace-Cargo.toml" ''
    [workspace]
    members = [
      "crates/aspen-core",
      "crates/aspen-constants",
      "crates/aspen-hlc",
      "crates/aspen-cluster-types",
      "crates/aspen-kv-types",
      "crates/aspen-traits",
      "crates/aspen-time",
      "crates/aspen-disk",
      "crates/aspen-storage-types",
      "crates/aspen-layer",
    ]
    resolver = "3"

    [workspace.dependencies]
    aspen-constants = { path = "crates/aspen-constants" }
    aspen-core = { path = "crates/aspen-core" }
    aspen-hlc = { path = "crates/aspen-hlc" }
    aspen-kv-types = { path = "crates/aspen-kv-types" }
    aspen-cluster-types = { path = "crates/aspen-cluster-types" }
    aspen-traits = { path = "crates/aspen-traits" }
    aspen-time = { path = "crates/aspen-time" }
    aspen-disk = { path = "crates/aspen-disk" }
    aspen-storage-types = { path = "crates/aspen-storage-types" }
    aspen-layer = { path = "crates/aspen-layer" }
  '';

  # Override CI config for the dogfood test.
  #
  # Uses `type = 'nix` jobs to exercise the real NixBuildWorker code path.
  # The pushed flake defines REAL cargo check derivations that compile Rust code
  # using pre-staged toolchain and vendored deps (isolation = 'none for filesystem access).
  #
  # Three stages:
  #   1. Validate: structural check (source tree has 70+ crates)
  #   2. Build: two parallel `cargo check` builds (aspen-constants + aspen-time)
  #   3. Test: `cargo check -p aspen-core` (real crate with dependencies)
  #
  # This proves the full NixBuildWorker path: payload parsing, `nix build`
  # execution, log streaming via KV chunks, and output path collection —
  # with REAL Rust compilation, not just file-existence checks.
  dogfoodCiConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "aspen-dogfood-build",
      description = "Build Aspen from its own Forge via its own CI (Nix executor)",
      stages = [
        {
          name = "validate",
          jobs = [
            {
              name = "check-source-tree",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.source-tree",
              timeout_secs = 120,
            },
          ],
        },
        {
          name = "build",
          depends_on = ["validate"],
          jobs = [
            {
              name = "cargo-check-constants",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.cargo-check-constants",
              isolation = 'none,
              timeout_secs = 180,
            },
            {
              name = "cargo-check-time",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.cargo-check-time",
              isolation = 'none,
              timeout_secs = 180,
            },
          ],
        },
        {
          name = "test",
          depends_on = ["build"],
          jobs = [
            {
              name = "cargo-check-core",
              type = 'nix,
              flake_url = ".",
              flake_attr = "checks.x86_64-linux.cargo-check-core",
              isolation = 'none,
              timeout_secs = 600,
            },
          ],
        },
      ],
    }
  '';

  # A self-contained flake.nix to be pushed to Forge alongside Aspen source.
  #
  # Defines REAL cargo check derivations that compile actual Rust code.
  # Uses isolation='none (--no-sandbox) to access pre-staged tools:
  #   - Rust nightly toolchain at /etc/aspen-ci/rust/bin/
  #   - Vendored cargo deps config at /etc/aspen-ci/cargo-config.toml
  #
  # Three types of checks:
  #   1. source-tree: structural validation (fast, sandboxed)
  #   2. cargo-check-constants/time: standalone crate builds (no deps)
  #   3. cargo-check-core: workspace build with vendored dependencies
  #
  # Uses double-quoted args strings to avoid nested ''...'' escaping issues.
  dogfoodFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen CI dogfood checks — real Rust compilation";
      inputs = {};
      outputs = { self, ... }: {
        checks.x86_64-linux = {

          # ── Stage 1: structural validation (sandboxed, fast) ─────────
          source-tree = derivation {
            name = "check-source-tree";
            system = "x86_64-linux";
            builder = "/bin/sh";
            src = self;
            args = [ "-c" "cd $src && test -f Cargo.toml && test -d crates && echo PASS > $out" ];
          };

          # ── Stage 2: standalone cargo check (no-sandbox, zero deps) ──
          #
          # Copy individual crates to isolated dirs outside the workspace
          # so cargo doesn't try to resolve all workspace deps.
          # These crates have ZERO external dependencies — only rustc needed.

          cargo-check-constants = derivation {
            name = "cargo-check-aspen-constants";
            system = "x86_64-linux";
            builder = "/bin/sh";
            src = self;
            args = [ "-c" "export PATH=/etc/aspen-ci/rust/bin:/run/current-system/sw/bin:$PATH && export CARGO_HOME=/tmp/cargo-home && mkdir -p $CARGO_HOME /tmp/build && cp -r $src/crates/aspen-constants /tmp/build/aspen-constants && chmod -R u+w /tmp/build/aspen-constants && cd /tmp/build/aspen-constants && cargo check 2>&1 && echo PASS > $out" ];
          };

          cargo-check-time = derivation {
            name = "cargo-check-aspen-time";
            system = "x86_64-linux";
            builder = "/bin/sh";
            src = self;
            args = [ "-c" "export PATH=/etc/aspen-ci/rust/bin:/run/current-system/sw/bin:$PATH && export CARGO_HOME=/tmp/cargo-home && mkdir -p $CARGO_HOME /tmp/build && cp -r $src/crates/aspen-time /tmp/build/aspen-time && chmod -R u+w /tmp/build/aspen-time && cd /tmp/build/aspen-time && cargo check 2>&1 && echo PASS > $out" ];
          };

          # ── Stage 3: workspace cargo check with vendored deps ────────
          #
          # Build aspen-core (real crate with dependencies: serde, tokio, iroh, etc.)
          # Uses the full workspace source with vendored cargo deps.
          # A fresh .cargo/config.toml replaces the dev config (mold linker etc.)
          # with the vendor source replacement pointing to pre-staged deps.

          cargo-check-core = derivation {
            name = "cargo-check-aspen-core";
            system = "x86_64-linux";
            builder = "/bin/sh";
            src = self;
            args = [ "-c" "set -e && export PATH=/etc/aspen-ci/rust/bin:/run/current-system/sw/bin:$PATH && export CARGO_HOME=/tmp/cargo-home && export CARGO_TARGET_DIR=/tmp/cargo-target && mkdir -p $CARGO_HOME $CARGO_TARGET_DIR /tmp/workspace && cp -r $src/. /tmp/workspace/ && chmod -R u+w /tmp/workspace && cd /tmp/workspace && cp /etc/aspen-ci/minimal-workspace.toml Cargo.toml && mkdir -p .cargo && cp /etc/aspen-ci/cargo-config.toml .cargo/config.toml && cargo check -p aspen-core 2>&1 && echo PASS > $out" ];
          };
        };
      };
    }
  '';

  # Create a tarball of Aspen's source for the VM.
  # We use the actual git-tracked source tree.
  aspenSourceTar = pkgs.runCommand "aspen-source-tar" {} ''
    cd ${aspenSource}
    ${pkgs.gnutar}/bin/tar czf $out --transform='s,^\./,aspen/,' .
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
          # Nix is needed on PATH for NixBuildWorker to run `nix build`
          pkgs.nix
          # Rust toolchain for cargo check builds (accessed via /etc/aspen-ci/rust)
          rustToolChain
          # C compiler and linker for crates with build scripts
          pkgs.gcc
        ];

        # Pre-stage Rust toolchain and vendored deps at well-known paths.
        # The pushed flake's derivations (built with --no-sandbox) access these
        # to compile Rust code without network access.
        environment.etc."aspen-ci/rust".source = rustToolChain;
        environment.etc."aspen-ci/cargo-config.toml".source = cargoConfig;
        environment.etc."aspen-ci/minimal-workspace.toml".source = minimalWorkspaceToml;

        networking.firewall.enable = false;

        # Enable nix-command and flakes for NixBuildWorker's `nix build`
        nix.settings.experimental-features = ["nix-command" "flakes"];
        # Disable sandbox — builds run inside a VM which is already sandboxed
        nix.settings.sandbox = false;

        # More memory for cargo check with dependencies (aspen-core build)
        virtualisation.memorySize = 6144;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      SOURCE_TAR = "${aspenSourceTar}"
      DOGFOOD_CI = "${dogfoodCiConfig}"
      DOGFOOD_FLAKE = "${dogfoodFlake}"

      # ── helpers ──────────────────────────────────────────────────────

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
              err = node1.succeed("cat /tmp/_cli_err.txt 2>/dev/null || true").strip()
              node1.log(f"cli() JSON parse failed, raw={raw!r}, stderr={err}")
              return raw.strip()

      def cli_text(cmd):
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      def wait_for_pipeline(run_id, timeout=300):
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

      # ── phase 1: create forge repo ──────────────────────────────────

      with subtest("create forge repository for aspen"):
          out = cli("git init aspen")
          node1.log(f"Repo create: {out}")
          assert isinstance(out, dict), f"expected dict: {out}"
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id in response: {out}"
          node1.log(f"Created aspen repo: {repo_id}")

      # ── phase 2: watch repo for auto-trigger ────────────────────────

      with subtest("ci watch enables auto-trigger"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── phase 3: prepare and push real aspen source ─────────────────

      with subtest("extract aspen source"):
          node1.succeed(f"mkdir -p /tmp/aspen-dogfood")
          node1.succeed(f"tar xzf {SOURCE_TAR} -C /tmp/aspen-dogfood")
          # Replace CI config with our dogfood-specific one
          node1.succeed(f"mkdir -p /tmp/aspen-dogfood/aspen/.aspen")
          node1.succeed(f"cp {DOGFOOD_CI} /tmp/aspen-dogfood/aspen/.aspen/ci.ncl")
          # Add the dogfood flake for nix check derivations
          node1.succeed(f"cp {DOGFOOD_FLAKE} /tmp/aspen-dogfood/aspen/flake.nix")

          # Verify source looks right
          crate_count = node1.succeed(
              "ls -d /tmp/aspen-dogfood/aspen/crates/*/ 2>/dev/null | wc -l"
          ).strip()
          node1.log(f"Extracted source: {crate_count} crates")
          assert int(crate_count) >= 70, f"Expected 70+ crates, got {crate_count}"

          file_count = node1.succeed(
              "find /tmp/aspen-dogfood/aspen -type f | wc -l"
          ).strip()
          node1.log(f"Total files: {file_count}")

      with subtest("push aspen source to forge"):
          ticket = get_ticket()

          # Initialize git repo from the extracted source
          node1.succeed(
              "cd /tmp/aspen-dogfood/aspen && "
              "git init --initial-branch=main && "
              "git config user.email 'dogfood@aspen.local' && "
              "git config user.name 'Aspen Dogfood' && "
              "git add -A && "
              "git commit -m 'dogfood: aspen self-hosts'"
          )

          # Count git objects to understand the push size
          obj_count = node1.succeed(
              "cd /tmp/aspen-dogfood/aspen && "
              "git count-objects -v | grep 'in-pack\\|count' || true"
          ).strip()
          node1.log(f"Git objects: {obj_count}")

          # Push to forge
          remote_url = f"aspen://{ticket}/{repo_id}"
          node1.succeed(
              f"cd /tmp/aspen-dogfood/aspen && "
              f"git remote add aspen '{remote_url}'"
          )

          node1.log("Starting git push of full Aspen source...")
          push_start = time.time()

          exit_code, _ = node1.execute(
              "cd /tmp/aspen-dogfood/aspen && "
              "RUST_LOG=warn git push aspen main "
              ">/tmp/git-push-stdout.txt 2>/tmp/git-push-stderr.txt"
          )
          push_elapsed = time.time() - push_start

          push_stderr = node1.succeed("cat /tmp/git-push-stderr.txt 2>/dev/null || true").strip()
          node1.log(f"Push completed in {push_elapsed:.1f}s, exit={exit_code}")
          if push_stderr:
              node1.log(f"Push stderr (last 500 chars): {push_stderr[-500:]}")
          assert exit_code == 0, f"git push failed (exit {exit_code}): {push_stderr[-1000:]}"
          node1.log(f"Successfully pushed Aspen source to Forge in {push_elapsed:.1f}s")

      # ── phase 4: wait for auto-triggered pipeline ───────────────────

      with subtest("auto-triggered dogfood pipeline appears"):
          deadline = time.time() + 90
          run_id = None
          while time.time() < deadline:
              list_out = cli("ci list", check=False)
              if isinstance(list_out, dict):
                  runs = list_out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      node1.log(f"Dogfood pipeline triggered: {run_id}")
                      break
              time.sleep(3)
          assert run_id, "No auto-triggered pipeline appeared within 90s"

      # ── phase 4b: start real-time log streaming on a running job ────

      with subtest("start real-time log streaming"):
          # Wait until at least one job is visible in the pipeline status
          stream_job_id = None
          stream_job_name = None
          deadline = time.time() + 60
          while time.time() < deadline:
              status_out = cli(f"ci status {run_id}", check=False)
              if isinstance(status_out, dict):
                  for stage in status_out.get("stages", []):
                      for job in stage.get("jobs", []):
                          if job.get("id"):
                              stream_job_id = job["id"]
                              stream_job_name = job.get("name", "unknown")
                              break
                      if stream_job_id:
                          break
              if stream_job_id:
                  break
              time.sleep(3)

          if stream_job_id:
              node1.log(f"Starting --follow log stream for job '{stream_job_name}' ({stream_job_id})")
              ticket = get_ticket()
              # Launch ci logs --follow as a background systemd unit.
              # It will stream chunks to a file until the job completes, then exit.
              # Use absolute path — systemd-run starts a transient unit that does
              # NOT inherit environment.systemPackages PATH.
              cli_path = "/run/current-system/sw/bin/aspen-cli"
              node1.succeed(
                  f"systemd-run --unit=ci-log-stream "
                  f"bash -c \""
                  f"{cli_path} --ticket '{ticket}' ci logs {run_id} {stream_job_id} --follow "
                  f">/tmp/ci-stream-output.txt 2>/tmp/ci-stream-err.txt"
                  f"\""
              )
              node1.log("Background log stream started")
          else:
              node1.log("WARNING: No job IDs visible yet, skipping real-time stream test")

      # ── phase 5: wait for pipeline completion ───────────────────────

      with subtest("dogfood pipeline completes successfully"):
          # Three stages: validate (quick) + build (2 parallel cargo checks) + test (cargo check core)
          # cargo check -p aspen-core with vendored deps can take several minutes
          final_status = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Pipeline final: {json.dumps(final_status, indent=2)}")
          pipeline_status = final_status.get("status")

          if pipeline_status == "failed":
              node1.log("Pipeline FAILED — dumping details")
              stages = final_status.get("stages", [])
              for s in stages:
                  for j in s.get("jobs", []):
                      node1.log(f"  Job '{j.get('name')}': {j.get('status')}")

          assert pipeline_status == "success", \
              f"Dogfood pipeline failed: {pipeline_status}: {final_status}"
          node1.log("Aspen dogfood pipeline completed successfully!")

          # Verify stage-level statuses are populated (not stuck at "pending")
          for stage in final_status.get("stages", []):
              stage_name = stage.get("name")
              stage_status = stage.get("status")
              node1.log(f"  Stage '{stage_name}': status={stage_status}")
              assert stage_status == "success", \
                  f"Stage '{stage_name}' has status '{stage_status}', expected 'success'"

      # ── phase 5b: verify real-time log stream captured output ───────
      # NixBuildWorker writes CI log chunks to _ci:logs:* KV keys during
      # execution, enabling real-time streaming via `ci logs --follow`.
      # Since this test now uses type='nix jobs, log content should be non-empty.

      with subtest("real-time log stream captured output"):
          if stream_job_id:
              # Give the follow stream a few seconds to finish after pipeline completion
              time.sleep(5)

              # Check if the stream unit finished
              exit_code, _ = node1.execute(
                  "systemctl is-active ci-log-stream 2>/dev/null"
              )
              stream_status = node1.succeed(
                  "systemctl show ci-log-stream --property=ActiveState --value 2>/dev/null || echo unknown"
              ).strip()
              node1.log(f"Log stream unit state: {stream_status}")

              # Read captured output
              stream_output = node1.succeed(
                  "cat /tmp/ci-stream-output.txt 2>/dev/null || echo empty"
              ).strip()
              stream_err = node1.succeed(
                  "cat /tmp/ci-stream-err.txt 2>/dev/null || echo empty"
              ).strip()

              if stream_output and stream_output != "empty":
                  output_lines = stream_output.count("\n")
                  node1.log(f"Real-time stream captured {len(stream_output)} bytes, {output_lines} lines")
                  node1.log(f"Stream output (first 500 chars): {stream_output[:500]}")
              else:
                  # NixBuildWorker writes log chunks — if we got nothing, the stream failed.
                  if stream_err and stream_err != "empty":
                      node1.log(f"Stream stderr: {stream_err[:500]}")
                  assert False, (
                      f"Real-time log stream captured no output. "
                      f"Stream unit state: {stream_status}, stderr: {stream_err[:300]}"
                  )

              # Clean up
              node1.execute("systemctl stop ci-log-stream 2>/dev/null || true")
          else:
              node1.log("Skipped: no stream_job_id was captured")

      # ── phase 6: verify job tracking for all completed jobs ─────────

      with subtest("all pipeline jobs completed"):
          # Collect all job IDs from the final pipeline status
          all_jobs = []
          for stage in final_status.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append({
                          "id": job["id"],
                          "name": job.get("name", "unknown"),
                          "status": job.get("status", "unknown"),
                      })

          node1.log(f"Pipeline had {len(all_jobs)} jobs")
          assert len(all_jobs) >= 4, f"Expected at least 4 jobs (validate + 2 build + test), got {len(all_jobs)}"

          # Verify all jobs succeeded
          for job in all_jobs:
              assert job["status"] == "success", \
                  f"Job '{job['name']}' has status '{job['status']}', expected 'success'"
          node1.log("All 4 pipeline jobs completed successfully")

      # ── phase 6b: verify ci output for completed jobs ───────────────

      with subtest("ci output retrieves full job output"):
          # Pick the first job to check full output
          assert len(all_jobs) > 0, "No jobs to check output for"
          first_job = all_jobs[0]
          output_result = cli(
              f"ci output {run_id} {first_job['id']}",
              check=False
          )
          node1.log(f"ci output for '{first_job['name']}': {type(output_result)}")
          assert isinstance(output_result, dict), \
              f"Expected dict from ci output, got: {output_result}"
          was_found = output_result.get("was_found", False)
          has_stdout = bool(output_result.get("stdout"))
          has_error = bool(output_result.get("error"))
          node1.log(
              f"  was_found={was_found}, has_stdout={has_stdout}, "
              f"error={output_result.get('error', 'none')}"
          )
          assert was_found, f"ci output was_found=False for completed job '{first_job['name']}'"

      # ── phase 6c: diagnostic log retrieval (non-blocking) ───────────
      # Shell worker doesn't write _ci:logs:* keys, so ci logs will return
      # empty. Log this for diagnostic purposes but don't fail.

      with subtest("ci logs diagnostic check"):
          jobs_with_logs = 0
          for job in all_jobs:
              job_id = job["id"]
              job_name = job["name"]
              ticket = get_ticket()

              exit_code, _ = node1.execute(
                  f"aspen-cli --ticket '{ticket}' ci logs {run_id} {job_id} "
                  f">/tmp/ci-logs-{job_name}.txt 2>/tmp/ci-logs-{job_name}-err.txt"
              )

              log_content = node1.succeed(
                  f"cat /tmp/ci-logs-{job_name}.txt 2>/dev/null || echo """
              ).strip()

              if log_content:
                  jobs_with_logs += 1
                  node1.log(f"  Job '{job_name}': {len(log_content)} bytes of logs")
              else:
                  node1.log(f"  Job '{job_name}': no CI log chunks")

          node1.log(f"Jobs with CI log chunks: {jobs_with_logs}/{len(all_jobs)}")
          # NixBuildWorker writes log chunks — ALL nix jobs must have them.
          assert jobs_with_logs == len(all_jobs), \
              f"Expected all {len(all_jobs)} nix jobs to have CI log chunks, only {jobs_with_logs} did"
          node1.log(f"Log streaming confirmed: {jobs_with_logs}/{len(all_jobs)} jobs with CI log chunks")

      # ── phase 7: verify CI list shows success ───────────────────────

      with subtest("ci list shows successful dogfood run"):
          result = cli("ci list", check=False)
          if isinstance(result, dict):
              runs = result.get("runs", [])
              found = any(
                  r.get("run_id") == run_id and r.get("status") == "success"
                  for r in runs
              )
              assert found, f"run {run_id} not in list with status=success: {runs}"

      # ── done ─────────────────────────────────────────────────────────
      node1.log("Dogfood test passed: Forge → CI trigger → NixBuildWorker → cargo check (real Rust compilation)")
    '';
  }
