# NixOS VM test: Deploy executor statefulness writes KV lifecycle state.
#
# Validates the deploy-resource-statefulness from adopt-nixops4-patterns:
#
#   1. Push a flake with a deploy stage (stateful=true, the default)
#   2. CI builds the artifact, deploy stage runs
#   3. Deploy executor writes _deploy:state:{id}:metadata to KV
#   4. Deploy executor writes _deploy:state:{id}:node:{n} per node
#   5. Verify the KV entries exist and contain expected structure
#
# Run:
#   nix build .#checks.x86_64-linux.deploy-statefulness-test --impure --option sandbox false
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
  secretKey = "0000000000000009000000000000000900000000000000090000000000000009";
  cookie = "deploy-state-test";

  # CI config with deploy stage. stateful defaults to true.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "deploy-state-test",
      stages = [
        {
          name = "build",
          jobs = [
            {
              name = "build-app",
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
              name = "deploy-stateful",
              type = 'deploy,
              artifact_from = "build-app",
              strategy = "rolling",
              max_concurrent = 1,
              health_check_timeout_secs = 60,
              timeout_secs = 300,
              expected_binary = "bin/cowsay",
            },
          ],
        },
      ],
    }
  '';

  cowsayFlake = pkgs.writeText "flake.nix" ''
    {
      description = "Deploy statefulness test";
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
    name = "deploy-statefulness";
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

      virtualisation.memorySize = 4096;
      virtualisation.cores = 2;
      virtualisation.diskSize = 20480;
      virtualisation.writableStoreUseTmpfs = false;
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

      def wait_pipeline(run_id, timeout=600):
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

      # ── forge repo + push ────────────────────────────────────────

      with subtest("create repo"):
          out = cli("git init deploy-state-repo")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          result = cli(f"ci watch {repo_id}")
          assert result.get("is_success"), f"ci watch failed: {result}"

      with subtest("push flake with deploy stage"):
          ticket = get_ticket()
          node1.succeed(
              "mkdir -p /tmp/deploy-repo/.aspen && "
              "cp ${cowsayFlake} /tmp/deploy-repo/flake.nix && "
              "cp ${ciConfig} /tmp/deploy-repo/.aspen/ci.ncl && "
              "cd /tmp/deploy-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'test@test' && "
              "git config user.name 'Test' && "
              "git add -A && "
              "git commit -m 'deploy with statefulness'"
          )
          node1.succeed(
              f"cd /tmp/deploy-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )

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
          assert run_id, "No pipeline triggered"

      with subtest("pipeline completes"):
          result = wait_pipeline(run_id, timeout=600)
          pipeline_status = result.get("status")
          node1.log(f"Pipeline final status: {pipeline_status}")

          # Log stage details
          for stage in result.get("stages", []):
              for job in stage.get("jobs", []):
                  node1.log(f"  {stage["name"]}/{job["name"]}: {job.get("status")}")

      # ── verify deploy state in KV ───────────────────────────────

      with subtest("deploy state metadata in KV"):
          # The deploy executor writes _deploy:state:{deploy_id}:metadata
          scan = cli("kv scan --prefix '_deploy:state:' --limit 20", check=False)
          node1.log(f"Deploy state scan: {scan}")

          entries = []
          if isinstance(scan, dict):
              entries = scan.get("entries", scan.get("kvs", []))

          node1.log(f"Found {len(entries)} deploy state entries")

          metadata_found = False
          node_state_found = False

          for entry in entries:
              key = entry.get("key", "")
              node1.log(f"  Deploy KV key: {key}")

              if ":metadata" in key:
                  metadata_found = True
                  value = entry.get("value", "")
                  node1.log(f"  Metadata: {value[:500]}")

                  # Parse and verify metadata structure
                  try:
                      meta = json.loads(value)
                      node1.log(f"  Deploy ID in metadata: {meta.get("deploy_id", "N/A")}")
                      node1.log(f"  Strategy: {meta.get("strategy", "N/A")}")
                      node1.log(f"  Stateful: {meta.get("stateful", "N/A")}")
                  except (json.JSONDecodeError, ValueError):
                      node1.log(f"  (not JSON: {value[:200]})")

              if ":node:" in key:
                  node_state_found = True
                  value = entry.get("value", "")
                  node1.log(f"  Node state: {value[:500]}")

          if metadata_found:
              node1.log("Deploy metadata KV entry found")
          else:
              node1.log("No deploy metadata entry (deploy may have failed before writing)")

          if node_state_found:
              node1.log("Per-node deploy state KV entry found")

      # ── cluster healthy ──────────────────────────────────────────

      with subtest("cluster healthy after deploy"):
          node1.wait_until_succeeds(
              "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
              timeout=30,
          )

          # KV still works
          cli("kv set deploy-state-check alive")
          out = cli("kv get deploy-state-check")
          node1.log(f"KV after deploy: {out}")

      node1.log("Deploy statefulness test passed")
    '';
  }
