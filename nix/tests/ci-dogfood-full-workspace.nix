# Dogfood NixOS VM test: build the ENTIRE 80-crate Aspen workspace through CI.
#
# Pushes all 80 workspace crates (343K lines of Rust, 658 cargo packages)
# to Forge via git-remote-aspen, triggers the CI pipeline, and builds the
# full `aspen-node` binary via `nix build` inside the test VM.
#
# Job type is `vm` with `ciLocalExecutor = true`, so the LocalExecutorWorker
# handles the job directly (no nested Cloud Hypervisor VMs needed). The
# build runs `nix build .#default` using a flake.nix injected into the repo
# that wraps `rustPlatform.buildRustPackage` with the pre-vendored deps
# from the host's nix store.
#
# Uses the pre-built `fullSrc` derivation which already handles:
#   - All 80 workspace crates under crates/
#   - Vendored openraft at openraft/openraft
#   - Stubbed git deps (snix, mad-turmoil, h3-iroh)
#   - External deps (iroh-proxy-utils, aspen-wasm-plugin) as path deps
#   - Patched Cargo.lock with git source lines stripped
#
# Pipeline: single-stage, single `nix build` job producing aspen-node.
# Features: ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob
# (everything except plugins-rpc which needs nested KVM for hyperlight)
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-full-workspace-test --impure --option sandbox false
{
  pkgs,
  fullSrc,
  fullCargoVendorDir,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
  nixpkgsFlake,
}: let
  secretKey = "0000000000000007000000000000000700000000000000070000000000000007";
  cookie = "ci-full-workspace-test";

  ciConfig = ./fixtures/full-workspace-ci.ncl;

  # Inner flake.nix that builds aspen-node from the pushed workspace.
  #
  # Uses rustPlatform.buildRustPackage with the pre-vendored cargo deps
  # from the host's nix store (accessible via builtins.storePath since
  # nix sandbox is disabled in the test VM).
  #
  # The @VENDOR_DIR@ placeholder is substituted at repo assembly time
  # with the actual nix store path of fullCargoVendorDir.
  innerFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Aspen full workspace build";
      inputs.nixpkgs.url = "nixpkgs";
      outputs = { nixpkgs, self, ... }:
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
          vendorDir = builtins.storePath "@VENDOR_DIR@";
        in {
          packages.x86_64-linux.default = pkgs.stdenv.mkDerivation {
            pname = "aspen-node";
            version = "0.1.0";
            src = ./.;

            nativeBuildInputs = with pkgs; [
              cargo
              rustc
              pkg-config
              perl
              cmake
              python3
              clang
              mold
            ];

            buildInputs = with pkgs; [
              openssl
            ];

            # Use mold linker for faster linking
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "clang";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=''${pkgs.mold}/bin/mold";
            OPENSSL_NO_VENDOR = "1";

            configurePhase = "cd aspen && mkdir -p .cargo && cp ''${vendorDir}/config.toml .cargo/config.toml";

            buildPhase = "export HOME=$(mktemp -d) && cargo build --release --bin aspen-node --features ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob --offline 2>&1";

            installPhase = "mkdir -p $out/bin && cp target/release/aspen-node $out/bin/";

            meta.mainProgram = "aspen-node";
          };
        };
    }
  '';

  # Build the repo from fullSrc. fullSrc has:
  #   $fullSrc/aspen/           — workspace root (80 crates)
  #   $fullSrc/iroh-proxy-utils/ — sibling dep (../iroh-proxy-utils from workspace)
  #   $fullSrc/aspen-wasm-plugin/ — sibling dep (../aspen-wasm-plugin)
  #   $fullSrc/aspen-dns/       — sibling stub (../../../aspen-dns from aspen-net)
  #
  # The repo contains ONLY source code — no vendor dir. The inner flake
  # references the pre-vendored deps from the VM's nix store directly via
  # builtins.storePath, avoiding the need to push 500MB+ through git.
  fullWorkspaceRepo = pkgs.runCommand "aspen-full-workspace-repo" {} ''
    # Copy entire fullSrc structure (preserves sibling path deps)
    cp -r ${fullSrc}/. $out/
    chmod -R u+w $out

    # Remove .cargo/config.toml — the inner flake's vendorDir handles this
    rm -rf $out/aspen/.cargo

    # Inject flake.nix at repo root, with @VENDOR_DIR@ substituted
    ${pkgs.gnused}/bin/sed "s|@VENDOR_DIR@|${fullCargoVendorDir}|g" \
      ${innerFlake} > $out/flake.nix

    # Inject CI config
    mkdir -p $out/.aspen
    cp ${ciConfig} $out/.aspen/ci.ncl

    # Remove old flake.lock to avoid confusion with stale inputs
    rm -f $out/aspen/flake.lock

    # Verify structure
    crate_count=$(ls -d $out/aspen/crates/*/ 2>/dev/null | wc -l)
    if [ "$crate_count" -lt 70 ]; then
      echo "ERROR: Expected >=70 crate dirs, found $crate_count"
      exit 1
    fi

    test -f $out/aspen/Cargo.toml  || { echo "ERROR: missing Cargo.toml"; exit 1; }
    test -f $out/aspen/Cargo.lock  || { echo "ERROR: missing Cargo.lock"; exit 1; }
    test -d $out/aspen/openraft    || { echo "ERROR: missing openraft/"; exit 1; }
    test -d $out/iroh-proxy-utils  || { echo "ERROR: missing iroh-proxy-utils/"; exit 1; }
    test -f $out/flake.nix         || { echo "ERROR: missing flake.nix"; exit 1; }
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
    name = "ci-dogfood-full-workspace";
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
        # Use local executor: handles ci_vm jobs directly without nested VMs.
        # The test VM is already isolated (QEMU), no need for Cloud Hypervisor.
        ciLocalExecutor = true;
        ciWorkspaceDir = "/workspace";
      };

      environment.systemPackages = [
        aspenCliPackage
        gitRemoteAspenPackage
        pkgs.git
        pkgs.nix
      ];

      networking.firewall.enable = false;
      nix.settings.experimental-features = ["nix-command" "flakes"];
      # Sandbox disabled so the inner flake can use builtins.storePath
      # to reference the pre-vendored cargo deps from the host's nix store.
      nix.settings.sandbox = false;
      nix.registry.nixpkgs.flake = nixpkgsFlake;

      # Full workspace nix build needs substantial resources:
      # - rustc + cargo + stdenv download: ~3GB from cache.nixos.org
      # - cargo build peak: ~4GB resident
      # - linker peak: ~2GB
      # - 658 crate sources + build artifacts
      virtualisation.memorySize = 8192;
      virtualisation.cores = 4;
      virtualisation.diskSize = 40960;
      virtualisation.writableStoreUseTmpfs = false;
    };

    # Lightweight second node: downloads blob from node1 and runs the binary.
    # Separate single-node cluster — only shares iroh-blobs P2P, not raft.
    nodes.client = {
      imports = [
        ../../nix/modules/aspen-node.nix
      ];

      services.aspen.node = {
        enable = true;
        package = aspenNodePackage;
        nodeId = 1;
        cookie = "client-cluster";
        storageBackend = "redb";
        dataDir = "/var/lib/aspen";
        logLevel = "info";
        relayMode = "disabled";
        enableWorkers = false;
        enableCi = false;
        features = ["blob"];
      };

      environment.systemPackages = [
        aspenCliPackage
      ];

      networking.firewall.enable = false;
      virtualisation.memorySize = 2048;
      virtualisation.cores = 2;
    };

    testScript = ''
      import json, time

      WORKSPACE_REPO = "${fullWorkspaceRepo}"

      # Reference vendor dir so it's included in VM's nix store closure.
      # The inner flake accesses it via builtins.storePath at this exact path.
      VENDOR_DIR = "${fullCargoVendorDir}"

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

      def wait_for_pipeline(run_id, timeout=1800):
          """Wait for pipeline by streaming job logs as they're assigned."""
          deadline = time.time() + timeout
          streamed_jobs = set()
          while time.time() < deadline:
              result = cli(f"ci status {run_id}", check=False)
              if not isinstance(result, dict):
                  time.sleep(5)
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
              time.sleep(5)
          raise Exception(f"Pipeline {run_id} did not complete within {timeout}s")

      # ── boot ─────────────────────────────────────────────────────
      start_all()

      # Register the pre-vendored cargo deps in the VM's nix database.
      # Store paths from the test closure are physically present via 9p mount
      # but NOT registered in the nix DB. builtins.storePath requires both.
      node1.succeed(f"nix-store --register-validity <<'EOF'\n{VENDOR_DIR}\n\n0\nEOF")

      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      cli_text("cluster init")
      time.sleep(2)

      # Boot client node (separate single-node cluster for blob operations)
      client.wait_for_unit("aspen-node.service")
      client.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      client.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      client.succeed(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster init"
      )
      time.sleep(2)

      def client_cli(cmd, check=True):
          """Run aspen-cli against the client's own cluster."""
          t = client.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()
          run = f"aspen-cli --ticket '{t}' --json {cmd} >/tmp/_cli.json 2>/tmp/_cli.err"
          if check:
              client.succeed(run)
          else:
              client.execute(run)
          raw = client.succeed("cat /tmp/_cli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

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
          out = cli("git init aspen-full-build")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("ci watch"):
          result = cli(f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), f"ci watch failed: {result}"

      # ── push full workspace to forge ─────────────────────────────
      with subtest("push full 80-crate workspace to forge"):
          node1.succeed(
              f"cp -r --no-preserve=mode {WORKSPACE_REPO} /tmp/full-workspace && "
              "cd /tmp/full-workspace && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'aspen: full 80-crate workspace dogfood build'"
          )
          ticket = get_ticket()
          node1.succeed(
              f"cd /tmp/full-workspace && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          # Verify workspace structure (crates are under aspen/ subdirectory)
          crate_count = node1.succeed(
              "ls -d /tmp/full-workspace/aspen/crates/*/ | wc -l"
          ).strip()
          rs_count = node1.succeed(
              "find /tmp/full-workspace/aspen -name '*.rs' | wc -l"
          ).strip()
          node1.log(f"Pushed {crate_count} crate dirs, {rs_count} .rs files")
          assert int(crate_count) >= 70, f"Expected >=70 crate dirs, got {crate_count}"
          assert int(rs_count) >= 200, f"Expected >=200 .rs files, got {rs_count}"

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

      with subtest("full workspace build succeeds"):
          final = wait_for_pipeline(run_id, timeout=1800)
          node1.log(f"Final: {json.dumps(final, indent=2)}")
          status = final.get("status")
          assert status == "success", f"Pipeline failed: {status}: {final}"

      # ── verify nix output in job result ──────────────────────────
      with subtest("verify nix output recorded in job result"):
          all_jobs = []
          for stage in final.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("id"):
                      all_jobs.append(job)
          assert len(all_jobs) >= 1, f"Expected >=1 jobs, got {len(all_jobs)}"

          build_job = [j for j in all_jobs if j.get("name") == "build-aspen-node"][0]
          assert build_job["status"] == "success", \
              f"Build job not success: {build_job['status']}"

          job_data = cli(f"kv get __jobs:{build_job['id']}", check=False)

          # Parse the job result to extract nix_output
          nix_output = None
          if isinstance(job_data, dict):
              value = job_data.get("value", "")
              try:
                  job_result = json.loads(value) if isinstance(value, str) else value
                  data = job_result.get("result", {}).get("Success", {}).get("data", {})
                  nix_output = data.get("nix_output")
              except (json.JSONDecodeError, ValueError, AttributeError) as e:
                  node1.log(f"Failed to parse job result: {e}")

          assert nix_output is not None, \
              f"nix_output missing from job result — --print-out-paths not working"

          output_paths = nix_output.get("output_paths", [])
          assert len(output_paths) >= 1, \
              f"Expected at least one nix output path, got: {output_paths}"
          assert output_paths[0].startswith("/nix/store/"), \
              f"Expected nix store path, got: {output_paths[0]}"
          node1.log(f"Nix output path: {output_paths[0]}")

          blob_hash = nix_output.get("binary_blob_hash")
          assert blob_hash is not None, \
              f"binary_blob_hash missing — binary was not uploaded to blob store"
          binary_size = nix_output.get("binary_size", 0)
          assert binary_size > 1_000_000, \
              f"Binary too small ({binary_size} bytes) — a real aspen-node is 50+ MB"
          node1.log(f"Binary uploaded to blobs: hash={blob_hash} size={binary_size}")

      # ── fetch binary from blob store on node1 ───────────────────
      with subtest("fetch and run binary from blob store"):
          # blob get now streams large blobs (header + raw bytes) so
          # this works for the 86MB binary without hitting RPC size limits
          cli_text(f"blob get {blob_hash} -o /tmp/aspen-node-from-blob")
          node1.succeed("chmod +x /tmp/aspen-node-from-blob")

          fetched_size = int(node1.succeed("stat -c %s /tmp/aspen-node-from-blob").strip())
          node1.log(f"Fetched binary: {fetched_size} bytes")
          assert fetched_size == binary_size, \
              f"Size mismatch: got {fetched_size}, expected {binary_size}"

          version_output = node1.succeed("/tmp/aspen-node-from-blob --version").strip()
          node1.log(f"Version: {version_output}")
          assert "aspen" in version_output.lower(), \
              f"Version output missing 'aspen': {version_output}"

      # ── cross-node: download blob on client and run ──────────────
      with subtest("cross-node blob download and run"):
          # Get a blob ticket from node1 (contains node1's iroh endpoint)
          ticket_result = cli(f"blob ticket {blob_hash}")
          blob_ticket = ticket_result.get("ticket")
          assert blob_ticket, f"Failed to get blob ticket: {ticket_result}"
          node1.log(f"Blob ticket: {blob_ticket[:60]}...")

          # Client tells its own node to download from node1 via iroh-blobs P2P
          dl_result = client_cli(f"blob download {blob_ticket}")
          assert dl_result.get("is_success", False), \
              f"Blob download failed: {dl_result}"
          dl_size = dl_result.get("size_bytes", 0)
          node1.log(f"Client downloaded blob: {dl_size} bytes")
          assert dl_size == binary_size, \
              f"Size mismatch: downloaded {dl_size}, expected {binary_size}"

          # Export from client's local blob store to file (streaming get)
          client.succeed(
              f"aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) "
              f"blob get {blob_hash} -o /tmp/aspen-node-from-blob 2>/dev/null"
          )
          client.succeed("chmod +x /tmp/aspen-node-from-blob")

          # Verify the binary runs on the client node
          version_output = client.succeed("/tmp/aspen-node-from-blob --version").strip()
          node1.log(f"Client ran CI-built binary: {version_output}")
          assert "aspen" in version_output.lower(), \
              f"Version output missing 'aspen': {version_output}"

          node1.log("Cross-node verified: built on node1, downloaded to client via iroh-blobs, runs correctly")

      node1.log("FULL WORKSPACE DOGFOOD PASSED: Forge -> CI -> nix build -> blob store -> cross-node download -> run")
    '';
  }
