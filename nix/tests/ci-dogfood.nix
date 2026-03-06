# Dogfood NixOS VM test: push Aspen's own source to its Forge, build it.
#
# This is the self-hosting litmus test. It:
#   1. Creates a Forge repository
#   2. Pushes Aspen's ACTUAL source tree (80 crates, ~23MB) via git-remote-aspen
#   3. CI auto-triggers and validates the source tree structure
#   4. CI compiles aspen-constants (2,602 lines, zero-dep Rust crate) with rustc
#   5. Verifies pipeline completion with build output
#
# This proves Aspen can host its own source code AND compile its own
# Rust code through its own CI infrastructure.
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
}: let
  # Deterministic Iroh secret key.
  secretKey = "0000000000000005000000000000000500000000000000050000000000000005";

  cookie = "ci-dogfood-test";

  # Override CI config for the dogfood test.
  # Two stages:
  #   1. Validate source tree structure (quick sanity check)
  #   2. Actually compile aspen-constants (zero-dep crate, 2,602 lines of Rust)
  #
  # Stage 2 proves Aspen can build its own code through its own CI.
  # aspen-constants has no external dependencies, so cargo only needs rustc
  # (no vendoring/network). The Rust nightly toolchain is pre-staged in the
  # VM's nix store.
  dogfoodCiConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "aspen-dogfood-build",
      description = "Build Aspen from its own Forge via its own CI",
      stages = [
        {
          name = "validate",
          jobs = [
            {
              name = "check-source-tree",
              type = 'shell,
              command = "sh -c 'test -f Cargo.toml && grep -q \"name = .aspen.\" Cargo.toml && test -d crates && CRATE_COUNT=$(ls -d crates/*/ 2>/dev/null | wc -l) && echo \"Crate count: $CRATE_COUNT\" && test \"$CRATE_COUNT\" -ge 70 && echo \"Source tree OK\"'",
              timeout_secs = 60,
            },
          ],
        },
        {
          name = "build",
          depends_on = ["validate"],
          jobs = [
            {
              name = "compile-aspen-constants",
              type = 'shell,
              command = "sh -c 'export PATH=${rustToolChain}/bin:$PATH && export CARGO_HOME=/tmp/cargo-home && mkdir -p $CARGO_HOME && echo \"rustc version: $(rustc --version)\" && echo \"cargo version: $(cargo --version)\" && echo \"Compiling aspen-constants (2,602 lines, zero deps)...\" && cp -r crates/aspen-constants /tmp/aspen-constants-build && cd /tmp/aspen-constants-build && cargo build 2>&1 && echo \"Build output:\" && ls -la target/debug/libaspen_constants.rlib && echo \"Aspen built its own code through its own CI.\"'",
              timeout_secs = 120,
            },
          ],
        },
      ],
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
          rustToolChain
        ];

        networking.firewall.enable = false;

        # More memory for handling large git push (23MB source)
        virtualisation.memorySize = 4096;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

      SOURCE_TAR = "${aspenSourceTar}"
      DOGFOOD_CI = "${dogfoodCiConfig}"

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

      # ── phase 5: wait for pipeline completion ───────────────────────

      with subtest("dogfood pipeline completes successfully"):
          # Two stages: validate (quick) + build (rustc compile ~30s)
          final_status = wait_for_pipeline(run_id, timeout=180)
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

      # ── phase 6: verify CI list shows success ───────────────────────

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
      node1.log("Aspen built itself. Self-hosting dogfood test passed!")
    '';
  }
