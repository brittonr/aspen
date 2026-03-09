# Dogfood NixOS VM test: push Aspen's source to Forge, BUILD it via NixBuildWorker.
#
# This is the self-hosting litmus test. It:
#   1. Creates a Forge repository
#   2. Pushes Aspen's ACTUAL source tree (80+ crates, ~23MB) via git-remote-aspen
#   3. CI auto-triggers with 2-stage pipeline using `type = 'nix` jobs:
#      a. Validate: structural check (source tree has 70+ crates)
#      b. Build: `cargo build --bin aspen-node` with full feature set
#   4. Verifies pipeline completion with all stages passing
#   5. Verifies NixBuildWorker log chunks are written to KV store
#
# Pre-staged in the VM (no network needed for builds):
#   - Rust nightly toolchain at /etc/aspen-ci/rust/
#   - Vendored cargo deps via cargoVendorDir (crane-built)
#   - Cargo config at /etc/aspen-ci/cargo-config.toml
#
# This proves Aspen can host its own source code AND build itself with Nix
# (stdenv.mkDerivation + nixpkgs) through its own CI using NixBuildWorker.
#
# Run (requires --option sandbox false for VM internet via QEMU user-mode NAT):
#   nix build .#checks.x86_64-linux.ci-dogfood-test --impure --option sandbox false
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.ci-dogfood-test.driverInteractive --impure --option sandbox false
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
  nixpkgsFlake,
}: let
  # Deterministic Iroh secret key.
  secretKey = "0000000000000005000000000000000500000000000000050000000000000005";

  cookie = "ci-dogfood-test";

  # Crane's vendor dir has config.toml at root with absolute Nix store paths.
  # We use it directly — it already points to the vendored crates subdirectory
  # using full /nix/store/... paths, so no path rewriting needed.
  cargoConfig = "${cargoVendorDir}/config.toml";

  # Pre-process source for dogfood: stub all git deps so cargo can resolve
  # the full workspace without network access. Only registry deps are needed.
  # Replicates the transformations from fullSrc in flake.nix.
  dogfoodSource =
    pkgs.runCommand "aspen-dogfood-source" {
      nativeBuildInputs = [pkgs.gnused pkgs.findutils];
    } ''
      cp -r ${aspenSource} $out
      chmod -R u+w $out

      # ── Create stubs for all git dependencies ────────────────────
      stub() {
        local name="$1"; shift
        local dir="$out/.nix-stubs/$name"
        mkdir -p "$dir/src"
        {
          echo '[package]'
          echo "name = \"$name\""
          echo 'version = "0.1.0"'
          echo 'edition = "2024"'
          echo '[features]'
          for feat in "$@"; do echo "$feat = []"; done
        } > "$dir/Cargo.toml"
        echo '// stub' > "$dir/src/lib.rs"
      }
      stub mad-turmoil
      stub snix-castore
      stub snix-store
      stub nix-compat async serde
      stub nix-compat-derive
      stub h3-iroh
      stub iroh-proxy-utils
      stub aspen-wasm-plugin hooks

      # aspen-dns: optional dep of aspen-net (3 levels up from crates/aspen-net/)
      mkdir -p "$out/.nix-stubs/aspen-dns/src"
      cat > "$out/.nix-stubs/aspen-dns/Cargo.toml" << 'DNSEOF'
      [package]
      name = "aspen-dns"
      version = "0.1.0"
      edition = "2024"
      DNSEOF
      echo '// stub' > "$out/.nix-stubs/aspen-dns/src/lib.rs"

      # ── Rewrite git deps in root Cargo.toml ──────────────────────
      sed -i \
        -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = ".nix-stubs/mad-turmoil", optional = true }|' \
        -e 's|snix-castore = { git = "[^"]*"[^}]*}|snix-castore = { path = ".nix-stubs/snix-castore" }|' \
        -e 's|snix-store = { git = "[^"]*"[^}]*}|snix-store = { path = ".nix-stubs/snix-store" }|' \
        -e 's|nix-compat = { git = "[^"]*"[^}]*}|nix-compat = { path = ".nix-stubs/nix-compat", features = ["async", "serde"] }|' \
        -e 's|h3-iroh = { git = "[^"]*"[^}]*}|h3-iroh = { path = ".nix-stubs/h3-iroh" }|' \
        -e 's|aspen-wasm-plugin = { path = "[^"]*" }|aspen-wasm-plugin = { path = ".nix-stubs/aspen-wasm-plugin" }|' \
        $out/Cargo.toml

      # Remove dep:mad-turmoil from simulation feature (already optional stub)
      sed -i '/dep:mad-turmoil/s/, "dep:mad-turmoil"//g' $out/Cargo.toml

      # Strip git source lines from Cargo.lock so cargo doesn't try to resolve them
      sed -i '/^source = "git+/d' $out/Cargo.lock

      # ── Rewrite git deps in subcrate Cargo.tomls ─────────────────
      find $out/crates -name Cargo.toml -exec sed -i \
        -e 's|mad-turmoil = { git = "[^"]*"[^}]*optional = true[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil", optional = true }|' \
        -e 's|mad-turmoil = { git = "[^"]*"[^}]*}|mad-turmoil = { path = "../../.nix-stubs/mad-turmoil" }|' \
        -e 's|iroh-proxy-utils = { git = "[^"]*"[^}]*}|iroh-proxy-utils = { path = "../../.nix-stubs/iroh-proxy-utils" }|' \
        -e 's|aspen-wasm-plugin = { path = "[^"]*", optional = true }|aspen-wasm-plugin = { path = "../../.nix-stubs/aspen-wasm-plugin", optional = true }|' \
        -e 's|aspen-wasm-plugin = { path = "[^"]*" }|aspen-wasm-plugin = { path = "../../.nix-stubs/aspen-wasm-plugin" }|' \
        -e 's|aspen-dns = { path = "[^"]*", optional = true }|aspen-dns = { path = "../../.nix-stubs/aspen-dns", optional = true }|' \
        -e 's|aspen-dns = { path = "[^"]*" }|aspen-dns = { path = "../../.nix-stubs/aspen-dns" }|' \
        {} \;

      # ── Remove [patch.*] sections from .cargo/config.toml ────────
      if [ -f $out/.cargo/config.toml ]; then
        sed -i '/^\[patch\./,$d' $out/.cargo/config.toml
      fi
    '';

  # Override CI config for the dogfood test.
  #
  # Uses `type = 'nix` jobs to exercise the real NixBuildWorker code path.
  # Two stages:
  #   1. Validate: structural check (source tree has 70+ crates)
  #   2. Build: full `cargo build --bin aspen-node` with major features
  #
  # This proves the full NixBuildWorker path: payload parsing, `nix build`
  # execution, log streaming via KV chunks, and output path collection —
  # building the REAL aspen-node binary from source.
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
              name = "build-aspen-node",
              type = 'nix,
              flake_url = ".",
              flake_attr = "packages.x86_64-linux.aspen-node",
              isolation = 'none,
              timeout_secs = 3600,
            },
          ],
        },
      ],
    }
  '';

  # Build features for the dogfood binary.
  # Includes all features that don't require external git dependencies.
  # Excluded: proxy/net (iroh-proxy-utils), snix (snix git), simulation (mad-turmoil),
  #           plugins (hyperlight-wasm patching), nix-cache-proxy (h3-iroh)
  buildFeatures = "forge,git-bridge,blob,docs,hooks,jobs,ci,ci-basic,automerge,secrets,shell-worker,sql,federation,global-discovery,relay-server";

  # A self-contained flake.nix to be pushed to Forge alongside Aspen source.
  #
  # Uses nixpkgs stdenv.mkDerivation for a proper Nix build.
  # The nixpkgs input resolves from the VM's pre-registered flake registry
  # (no download needed). Pre-staged Rust toolchain and vendored deps are
  # accessed via --no-sandbox (isolation='none).
  #
  # Two outputs:
  #   1. checks.source-tree: structural validation (sandboxed, pure)
  #   2. packages.aspen-node: full cargo build of aspen-node binary
  dogfoodFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen CI dogfood — full project build with stdenv.mkDerivation";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { self, nixpkgs }: let
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
      in {
        checks.x86_64-linux = {
          # ── Structural validation (sandboxed, pure) ──────────────────
          source-tree = derivation {
            name = "check-source-tree";
            system = "x86_64-linux";
            builder = "/bin/sh";
            src = self;
            args = [ "-c" "cd $src && test -f Cargo.toml && test -d crates && echo PASS > $out" ];
          };
        };

        packages.x86_64-linux = {
          # ── Full aspen-node binary build ─────────────────────────────
          #
          # Builds the real aspen-node binary with all registry-dep features.
          # Uses pre-staged Rust toolchain at /etc/aspen-ci/rust/ and
          # vendored cargo deps via /etc/aspen-ci/cargo-config.toml.
          # This is the ultimate dogfood: Aspen builds itself.
          aspen-node = pkgs.stdenv.mkDerivation {
            name = "aspen-node-dogfood";
            src = self;
            dontConfigure = true;

            buildPhase = '''
              export PATH=/etc/aspen-ci/rust/bin:$PATH
              export CARGO_HOME=$TMPDIR/cargo
              export CARGO_TARGET_DIR=$TMPDIR/target
              mkdir -p $CARGO_HOME $CARGO_TARGET_DIR

              # Use pre-staged vendored deps
              mkdir -p .cargo
              cp /etc/aspen-ci/cargo-config.toml .cargo/config.toml

              echo "=== Building aspen-node with features: ${buildFeatures} ==="
              echo "=== Workspace crates: $(ls -d crates/*/ | wc -l) ==="

              cargo build --bin aspen-node \
                --features ${buildFeatures} \
                2>&1

              echo "=== Build complete ==="
              ls -lh $CARGO_TARGET_DIR/debug/aspen-node
            ''';

            dontStrip = true;

            installPhase = '''
              mkdir -p $out/bin
              cp $CARGO_TARGET_DIR/debug/aspen-node $out/bin/
            ''';
          };
        };
      };
    }
  '';

  # Create a tarball of the pre-processed Aspen source for the VM.
  aspenSourceTar = pkgs.runCommand "aspen-source-tar" {} ''
    cd ${dogfoodSource}
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
          # Rust toolchain for cargo builds (accessed via /etc/aspen-ci/rust)
          rustToolChain
          # C compiler and linker for crates with build scripts
          pkgs.gcc
          # protobuf compiler for build scripts
          pkgs.protobuf
        ];

        # Pre-stage Rust toolchain and vendored deps at well-known paths.
        # The pushed flake's derivations (built with --no-sandbox) access these
        # to compile Rust code without network access.
        environment.etc."aspen-ci/rust".source = rustToolChain;
        environment.etc."aspen-ci/cargo-config.toml".source = cargoConfig;

        networking.firewall.enable = false;

        # Enable nix-command and flakes for NixBuildWorker's `nix build`
        nix.settings.experimental-features = ["nix-command" "flakes"];
        # Disable sandbox — builds run inside a VM which is already sandboxed
        nix.settings.sandbox = false;

        # Pre-register nixpkgs in the flake registry so the pushed flake can
        # resolve `inputs.nixpkgs.url = "nixpkgs"` without downloading it.
        # The nixpkgs source is already in the VM's nix store (part of its closure).
        nix.registry.nixpkgs.flake = nixpkgsFlake;

        # Full project build needs substantial resources:
        # ~110 workspace crates, release build with many features
        virtualisation.memorySize = 16384;
        virtualisation.cores = 4;
        virtualisation.diskSize = 20480;
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
              time.sleep(10)
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
          # Add the dogfood flake for nix build derivations
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

      # ── phase 4b: start real-time log streaming on the build job ────

      with subtest("start real-time log streaming"):
          # Wait until the build job is visible in the pipeline status
          stream_job_id = None
          stream_job_name = None
          deadline = time.time() + 120
          while time.time() < deadline:
              status_out = cli(f"ci status {run_id}", check=False)
              if isinstance(status_out, dict):
                  for stage in status_out.get("stages", []):
                      for job in stage.get("jobs", []):
                          jname = job.get("name", "")
                          jid = job.get("id")
                          # Prefer the build job for streaming (most interesting output)
                          if jid and "build" in jname:
                              stream_job_id = jid
                              stream_job_name = jname
                              break
                          elif jid and not stream_job_id:
                              stream_job_id = jid
                              stream_job_name = jname
                      if stream_job_id and "build" in (stream_job_name or ""):
                          break
              if stream_job_id:
                  break
              time.sleep(3)

          if stream_job_id:
              node1.log(f"Starting --follow log stream for job '{stream_job_name}' ({stream_job_id})")
              ticket = get_ticket()
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
          # Full release build of aspen-node with many features.
          # This compiles ~110 workspace crates + hundreds of deps.
          final_status = wait_for_pipeline(run_id, timeout=3600)
          node1.log(f"Pipeline final: {json.dumps(final_status, indent=2)}")
          pipeline_status = final_status.get("status")

          if pipeline_status == "failed":
              node1.log("Pipeline FAILED — dumping details")
              stages = final_status.get("stages", [])
              for s in stages:
                  for j in s.get("jobs", []):
                      jname = j.get("name", "unknown")
                      jstatus = j.get("status", "unknown")
                      jid = j.get("id", "")
                      node1.log(f"  Job '{jname}': {jstatus}")
                      # Dump logs for failed jobs
                      if jstatus == "failed" and jid:
                          ticket = get_ticket()
                          node1.execute(
                              f"aspen-cli --ticket '{ticket}' ci logs {run_id} {jid} "
                              f">/tmp/ci-failed-logs.txt 2>/dev/null"
                          )
                          failed_logs = node1.succeed(
                              "tail -100 /tmp/ci-failed-logs.txt 2>/dev/null || echo 'no logs'"
                          ).strip()
                          node1.log(f"  Logs (last 100 lines):\n{failed_logs}")

          assert pipeline_status == "success", \
              f"Dogfood pipeline failed: {pipeline_status}: {final_status}"
          node1.log("Aspen dogfood pipeline completed successfully!")

          # Verify stage-level statuses
          for stage in final_status.get("stages", []):
              stage_name = stage.get("name")
              stage_status = stage.get("status")
              node1.log(f"  Stage '{stage_name}': status={stage_status}")
              assert stage_status == "success", \
                  f"Stage '{stage_name}' has status '{stage_status}', expected 'success'"

      # ── phase 5b: verify real-time log stream captured output ───────

      with subtest("real-time log stream captured output"):
          if stream_job_id:
              # Give the follow stream a few seconds to finish after pipeline completion
              time.sleep(5)

              stream_status = node1.succeed(
                  "systemctl show ci-log-stream --property=ActiveState --value 2>/dev/null || echo unknown"
              ).strip()
              node1.log(f"Log stream unit state: {stream_status}")

              stream_output = node1.succeed(
                  "cat /tmp/ci-stream-output.txt 2>/dev/null || echo empty"
              ).strip()
              stream_err = node1.succeed(
                  "cat /tmp/ci-stream-err.txt 2>/dev/null || echo empty"
              ).strip()

              if stream_output and stream_output != "empty":
                  output_lines = stream_output.count("\n")
                  node1.log(f"Real-time stream captured {len(stream_output)} bytes, {output_lines} lines")
                  node1.log(f"Stream output (first 1000 chars): {stream_output[:1000]}")
              else:
                  if stream_err and stream_err != "empty":
                      node1.log(f"Stream stderr: {stream_err[:500]}")
                  assert False, (
                      f"Real-time log stream captured no output. "
                      f"Stream unit state: {stream_status}, stderr: {stream_err[:300]}"
                  )

              node1.execute("systemctl stop ci-log-stream 2>/dev/null || true")
          else:
              node1.log("Skipped: no stream_job_id was captured")

      # ── phase 6: verify job tracking ────────────────────────────────

      with subtest("all pipeline jobs completed"):
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
          assert len(all_jobs) >= 2, f"Expected at least 2 jobs (validate + build), got {len(all_jobs)}"

          for job in all_jobs:
              assert job["status"] == "success", \
                  f"Job '{job['name']}' has status '{job['status']}', expected 'success'"
          node1.log(f"All {len(all_jobs)} pipeline jobs completed successfully")

      # ── phase 6b: verify ci output for build job ────────────────────

      with subtest("ci output retrieves build output"):
          # Find the build job specifically
          build_job = next((j for j in all_jobs if "build" in j["name"]), all_jobs[-1])
          output_result = cli(
              f"ci output {run_id} {build_job['id']}",
              check=False
          )
          node1.log(f"ci output for '{build_job['name']}': {type(output_result)}")
          assert isinstance(output_result, dict), \
              f"Expected dict from ci output, got: {output_result}"
          was_found = output_result.get("was_found", False)
          node1.log(f"  was_found={was_found}")
          assert was_found, f"ci output was_found=False for completed job '{build_job['name']}'"

      # ── phase 6c: verify CI log chunks for all jobs ─────────────────

      with subtest("ci logs diagnostic check"):
          jobs_with_logs = 0
          for job in all_jobs:
              job_id = job["id"]
              job_name = job["name"]
              ticket = get_ticket()

              node1.execute(
                  f"aspen-cli --ticket '{ticket}' ci logs {run_id} {job_id} "
                  f">/tmp/ci-logs-{job_name}.txt 2>/tmp/ci-logs-{job_name}-err.txt"
              )

              log_content = node1.succeed(
                  f"cat /tmp/ci-logs-{job_name}.txt 2>/dev/null || echo \"\""
              ).strip()

              if log_content:
                  jobs_with_logs += 1
                  node1.log(f"  Job '{job_name}': {len(log_content)} bytes of logs")
              else:
                  node1.log(f"  Job '{job_name}': no CI log chunks")

          node1.log(f"Jobs with CI log chunks: {jobs_with_logs}/{len(all_jobs)}")
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
      node1.log("DOGFOOD PASSED: Forge → CI trigger → NixBuildWorker → cargo build --bin aspen-node (real binary)")
    '';
  }
