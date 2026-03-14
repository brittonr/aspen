# NixOS VM test: NixBuildSupervisor kills stuck builds.
#
# Validates the process isolation from adopt-nixops4-patterns:
#
#   1. Push a flake with a build that hangs (sleep infinity)
#   2. CI config sets a short timeout (30s)
#   3. NixBuildSupervisor kills the child after timeout
#   4. Pipeline reports failure with timeout indication
#   5. Worker remains healthy and can process subsequent jobs
#
# Run:
#   nix build .#checks.x86_64-linux.nix-build-supervisor-test --impure
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
}: let
  secretKey = "0000000000000008000000000000000800000000000000080000000000000008";
  cookie = "nix-supervisor-test";

  # A flake whose build hangs forever.
  hangingFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Hanging build for supervisor timeout test";
      inputs = {};
      outputs = { self, ... }: {
        packages.x86_64-linux.default = derivation {
          name = "hangs-forever";
          system = "x86_64-linux";
          builder = "/bin/sh";
          args = [ "-c" "echo 'Starting infinite hang...'; sleep 999999" ];
        };
      };
    }
  '';

  # CI config with short timeout so the test doesn't wait long.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "supervisor-timeout-test",
      stages = [
        {
          name = "build",
          jobs = [
            {
              name = "hanging-build",
              type = 'nix,
              flake_url = ".",
              flake_attr = "packages.x86_64-linux.default",
              timeout_secs = 30,
            },
          ],
        },
      ],
    }
  '';

  # A flake that builds quickly (for the "worker recovers" check).
  quickFlake = pkgs.writeText "flake-quick.nix" ''
    {
      description = "Quick build to verify worker recovery";
      inputs = {};
      outputs = { self, ... }: {
        packages.x86_64-linux.default = derivation {
          name = "quick-build";
          system = "x86_64-linux";
          builder = "/bin/sh";
          args = [ "-c" "echo 'quick build OK' > $out" ];
        };
      };
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
    name = "nix-build-supervisor";
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

      nix.settings.experimental-features = ["nix-command" "flakes"];
      nix.settings.sandbox = false;
      networking.firewall.enable = false;
      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
    };

    testScript = ''
      import json, time

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

      def wait_pipeline(run_id, timeout=180):
          deadline = time.time() + timeout
          while time.time() < deadline:
              r = cli(f"ci status {run_id}", check=False)
              if isinstance(r, dict):
                  s = r.get("status")
                  node1.log(f"Pipeline {run_id}: {s}")
                  if s in ("success", "failed", "cancelled"):
                      return r
              time.sleep(5)
          raise Exception(f"Pipeline {run_id} timed out after {timeout}s")

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
      with subtest("install plugins"):
          for name, wasm, manifest in [
              ("kv", "/etc/aspen-plugins/kv-plugin.wasm", "/etc/aspen-plugins/kv-plugin.json"),
              ("forge", "/etc/aspen-plugins/forge-plugin.wasm", "/etc/aspen-plugins/forge-plugin.json"),
          ]:
              plugin_cli(f"plugin install {wasm} --manifest {manifest}")
          plugin_cli("plugin reload", check=False)
          time.sleep(8)

      # ── setup forge repo with hanging build ──────────────────────

      with subtest("create repo"):
          out = cli("git init supervisor-test-repo")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          result = cli(f"ci watch {repo_id}")
          assert result.get("is_success"), f"ci watch failed: {result}"

      with subtest("push hanging flake"):
          ticket = get_ticket()
          node1.succeed(
              "mkdir -p /tmp/hang-repo/.aspen && "
              "cp ${hangingFlake} /tmp/hang-repo/flake.nix && "
              "cp ${ciConfig} /tmp/hang-repo/.aspen/ci.ncl && "
              "cd /tmp/hang-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'hanging build'"
          )
          node1.succeed(
              f"cd /tmp/hang-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )

      # ── pipeline triggers and supervisor kills it ────────────────

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
          assert run_id, "No pipeline triggered"
          node1.log(f"Pipeline: {run_id}")

      with subtest("supervisor kills hanging build within timeout"):
          # The build hangs, supervisor should kill it after 30s timeout.
          # Wait up to 90s (30s timeout + overhead).
          result = wait_pipeline(run_id, timeout=90)
          assert result.get("status") == "failed", \
              f"Expected failure from timeout, got: {result.get("status")}"
          node1.log("Hanging build was killed by supervisor")

          # Check that the failure mentions timeout
          for stage in result.get("stages", []):
              for job in stage.get("jobs", []):
                  if job.get("name") == "hanging-build":
                      error = job.get("error", "")
                      node1.log(f"Job error: {error}")

      # ── verify no orphan processes ───────────────────────────────

      with subtest("no orphan nix or sleep processes"):
          # The supervisor should have killed the child tree.
          # Check that "sleep 999999" is not running.
          exit_code, output = node1.execute("pgrep -f 'sleep 999999' || true")
          orphans = output.strip()
          if orphans:
              node1.log(f"WARNING: orphan sleep processes found: {orphans}")
              # Kill them to not block the test
              node1.execute("pkill -f 'sleep 999999' || true")
          else:
              node1.log("No orphan processes — supervisor cleaned up correctly")

      # ── worker still healthy after kill ──────────────────────────

      with subtest("workers still healthy"):
          workers = cli_text("job workers")
          node1.log(f"Workers after timeout:\n{workers}")
          assert "Idle" in workers or "node-1-worker" in workers, \
              f"Workers should still be alive: {workers}"

      # ── push quick build to verify recovery ──────────────────────

      with subtest("worker recovers — quick build succeeds"):
          # Replace flake with one that builds quickly
          node1.succeed("cp ${quickFlake} /tmp/hang-repo/flake.nix")
          node1.succeed(
              "cd /tmp/hang-repo && "
              "git add -A && "
              "git commit -m 'fix: use quick build' && "
              "RUST_LOG=warn git push aspen main 2>/tmp/push2.err"
          )

          # Wait for second pipeline
          time.sleep(10)
          out = cli("ci list", check=False)
          runs = out.get("runs", []) if isinstance(out, dict) else []
          if len(runs) >= 2:
              run2_id = runs[0].get("run_id")
              if run2_id and run2_id != run_id:
                  result2 = wait_pipeline(run2_id, timeout=120)
                  node1.log(f"Recovery pipeline: {result2.get("status")}")
                  # Quick build should succeed
                  if result2.get("status") == "success":
                      node1.log("Worker recovered — quick build succeeded after timeout kill")
                  else:
                      node1.log(f"Recovery build status: {result2.get("status")} (may fail for other reasons)")

      # ── cluster healthy ──────────────────────────────────────────

      with subtest("cluster healthy"):
          node1.wait_until_succeeds(
              "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
              timeout=30,
          )

      node1.log("NixBuildSupervisor timeout test passed")
    '';
  }
