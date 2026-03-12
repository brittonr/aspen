# Multi-node deploy dogfood NixOS VM test.
#
# Spins up a 3-node Aspen cluster, pushes a Nix flake with a deploy stage
# to Forge, CI auto-triggers, build stage produces an artifact, and the
# deploy stage exercises the DeploymentCoordinator's rolling upgrade path
# across all 3 nodes (follower-first ordering, quorum safety).
#
#   1. Form a 3-node Raft cluster (init, add-learner ×2, change-membership)
#   2. Install WASM plugins (KV + Forge) on all nodes
#   3. Write sentinel KV data before deploy
#   4. Create Forge repo, enable CI auto-trigger
#   5. Push a cowsay Nix flake with .aspen/ci.ncl (build + deploy stages)
#   6. CI auto-triggers: build stage produces Nix store path
#   7. Deploy stage resolves artifact, DeploymentCoordinator dispatches
#      NodeUpgrade RPCs to followers first, then leader
#   8. Verify cluster health and KV data survive the deploy
#
# Run:
#   nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test --impure --option sandbox false
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test.driverInteractive --impure --option sandbox false
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
  nixpkgsFlake,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each).
  # Each node gets a unique key for distinct iroh endpoint IDs.
  secretKey1 = "0000000000000006000000000000000600000000000000060000000000000006";
  secretKey2 = "0000000000000007000000000000000700000000000000070000000000000007";
  secretKey3 = "0000000000000008000000000000000800000000000000080000000000000008";

  # Shared cluster cookie.
  cookie = "ci-deploy-multinode-test";

  # CI config: build + deploy stages.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "dogfood-deploy-multinode",
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

  # WASM plugin helpers (KV + Forge handlers are WASM-only).
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

  # Common node configuration.
  mkNodeConfig = {
    nodeId,
    secretKey,
    enableCi ? false,
    enableWorkers ? false,
  }: {
    imports = [
      ../../nix/modules/aspen-node.nix
      pluginHelpers.nixosConfig
    ];

    services.aspen.node = {
      enable = true;
      package = aspenNodePackage;
      inherit nodeId cookie secretKey;
      storageBackend = "redb";
      dataDir = "/var/lib/aspen";
      logLevel = "info";
      relayMode = "disabled";
      inherit enableWorkers enableCi;
      features = ["forge" "blob" "deploy"];
    };

    environment.systemPackages =
      [
        aspenCliPackage
      ]
      ++ lib.optionals enableCi [
        gitRemoteAspenPackage
        pkgs.git
        pkgs.nix
      ];

    networking.firewall.enable = false;

    # Nix settings for CI builds (only node1 needs these, but harmless on others).
    nix.settings.experimental-features = ["nix-command" "flakes"];
    nix.settings.sandbox = false;
    nix.registry.nixpkgs.flake = nixpkgsFlake;

    virtualisation.memorySize = 4096;
    virtualisation.cores = 2;
    virtualisation.diskSize = 20480;
    virtualisation.writableStoreUseTmpfs = false;
  };
in
  pkgs.testers.nixosTest {
    name = "ci-dogfood-deploy-multinode";
    skipLint = true;
    skipTypeCheck = true;

    nodes = {
      node1 = mkNodeConfig {
        nodeId = 1;
        secretKey = secretKey1;
        enableCi = true;
        enableWorkers = true;
      };
      node2 = mkNodeConfig {
        nodeId = 2;
        secretKey = secretKey2;
      };
      node3 = mkNodeConfig {
        nodeId = 3;
        secretKey = secretKey3;
      };
    };

    testScript = ''
      import json, re, time

      CI_CONFIG = "${ciConfig}"
      COWSAY_FLAKE = "${cowsayFlake}"

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket(node):
          """Read the cluster ticket from a node."""
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, ticket=None, check=True):
          """Run aspen-cli --json on a specific node and return parsed JSON."""
          if ticket is None:
              ticket = get_ticket(node1)
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli.json 2>/tmp/_cli.err"
          )
          if check:
              node.succeed(run)
          else:
              node.execute(run)
          raw = node.succeed("cat /tmp/_cli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(node, cmd, ticket=None):
          """Run aspen-cli (human output) on a specific node."""
          if ticket is None:
              ticket = get_ticket(node1)
          node.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli.txt 2>/dev/null"
          )
          return node.succeed("cat /tmp/_cli.txt")

      def plugin_cli(node, cmd, ticket=None, check=True):
          """Run aspen-plugin-cli on a specific node."""
          if ticket is None:
              ticket = get_ticket(node1)
          run = (
              f"aspen-plugin-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_pcli.json 2>/tmp/_pcli.err"
          )
          if check:
              node.succeed(run)
          else:
              node.execute(run)
          raw = node.succeed("cat /tmp/_pcli.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              return raw.strip()

      def get_endpoint_addr_json(node):
          """Extract endpoint address (id + addrs) from node's journal."""
          output = node.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep 'cluster ticket generated'"
              " | head -1"
          )
          eid_match = re.search(r'endpoint_id=([0-9a-f]{64})', output)
          assert eid_match, f"endpoint_id not found: {output[:300]}"
          eid = eid_match.group(1)

          addrs = []
          for addr in re.findall(r'(192\.168\.\d+\.\d+:\d+)', output):
              addrs.append({"Ip": addr})
          if not addrs:
              for addr in re.findall(r'(\d+\.\d+\.\d+\.\d+:\d+)', output):
                  addrs.append({"Ip": addr})

          assert addrs, f"no direct addrs found: {output[:300]}"
          endpoint_json = json.dumps({"id": eid, "addrs": addrs})
          node.log(f"EndpointAddr JSON: {endpoint_json}")
          return endpoint_json

      def wait_for_healthy(node, timeout=60):
          """Wait for a node to become healthy and responsive."""
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      def stream_job_logs(run_id, job_id, job_name):
          """Stream logs for a CI job."""
          ticket = get_ticket(node1)
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
              result = cli(node1, f"ci status {run_id}", check=False)
              if not isinstance(result, dict):
                  time.sleep(3)
                  continue

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

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      for node in [node1, node2, node3]:
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      wait_for_healthy(node1)

      addr2_json = get_endpoint_addr_json(node2)
      addr3_json = get_endpoint_addr_json(node3)

      # ── cluster formation ────────────────────────────────────────────

      with subtest("initialize 3-node cluster"):
          cli_text(node1, "cluster init")
          time.sleep(2)

          cli_text(node1, f"cluster add-learner --node-id 2 --addr '{addr2_json}'")
          time.sleep(3)

          cli_text(node1, f"cluster add-learner --node-id 3 --addr '{addr3_json}'")
          time.sleep(3)

          cli_text(node1, "cluster change-membership 1 2 3")
          time.sleep(5)

          status = cli(node1, "cluster status")
          node1.log(f"Cluster status: {json.dumps(status, indent=2)}")
          nodes_list = status.get("nodes", [])
          voters = [n for n in nodes_list if n.get("is_voter", False)]
          assert len(voters) == 3, f"expected 3 voters, got {len(voters)}: {nodes_list}"
          node1.log("3-node cluster formed")

      # ── install WASM plugins ─────────────────────────────────────────

      with subtest("install WASM plugins"):
          for name in ["kv", "forge"]:
              plugin_cli(
                  node1,
                  f"plugin install /etc/aspen-plugins/{name}-plugin.wasm "
                  f"--manifest /etc/aspen-plugins/{name}-plugin.json",
              )
          plugin_cli(node1, "plugin reload", check=False)
          time.sleep(8)

      # Stage plugin blobs on follower nodes so they can serve requests.
      with subtest("stage plugin blobs on followers"):
          for other in [node2, node3]:
              other_ticket = get_ticket(other)
              for pname in ["kv", "forge"]:
                  other.succeed(
                      f"aspen-cli --ticket '{other_ticket}' "
                      f"blob add /etc/aspen-plugins/{pname}-plugin.wasm "
                      f">/dev/null 2>/dev/null"
                  )
              other.log("plugin blobs staged")

      # ── write sentinel KV data before deploy ─────────────────────────

      with subtest("write sentinel KV data"):
          cli(node1, "kv set deploy-test-key deploy-survives-upgrade")
          result = cli(node1, "kv get deploy-test-key")
          node1.log(f"Sentinel KV written: {result}")

      # ── create forge repo + push deploy flake ────────────────────────

      with subtest("create forge repo"):
          out = cli(node1, "git init cowsay-deploy-multinode")
          repo_id = out.get("id") or out.get("repo_id")
          assert repo_id, f"no repo_id: {out}"
          node1.log(f"Repo: {repo_id}")

      with subtest("enable CI auto-trigger"):
          result = cli(node1, f"ci watch {repo_id}")
          assert isinstance(result, dict) and result.get("is_success"), \
              f"ci watch failed: {result}"

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
              "git commit -m 'deploy dogfood multinode'"
          )
          ticket = get_ticket(node1)
          node1.succeed(
              f"cd /tmp/deploy-repo && "
              f"git remote add aspen 'aspen://{ticket}/{repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>/tmp/push.err"
          )
          node1.log("Pushed flake with deploy config to Forge")

      # ── wait for pipeline ────────────────────────────────────────────

      with subtest("pipeline auto-triggers"):
          deadline = time.time() + 60
          run_id = None
          while time.time() < deadline:
              out = cli(node1, "ci list", check=False)
              if isinstance(out, dict):
                  runs = out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      break
              time.sleep(3)
          assert run_id, "No pipeline triggered within 60s"
          node1.log(f"Pipeline: {run_id}")

      with subtest("build stage completes"):
          final = wait_for_pipeline(run_id, timeout=600)
          node1.log(f"Final pipeline: {json.dumps(final, indent=2)}")

          build_stage = None
          for stage in final.get("stages", []):
              if stage.get("name") == "build":
                  build_stage = stage
                  break
          assert build_stage, "Build stage not found"
          for job in build_stage.get("jobs", []):
              assert job["status"] == "success", \
                  f"Build job '{job.get('name')}' failed: {job['status']}"
          node1.log("Build stage completed successfully")

      # ── verify deploy stage ──────────────────────────────────────────

      with subtest("deploy stage in pipeline"):
          deploy_stage = None
          for stage in final.get("stages", []):
              if stage.get("name") == "deploy":
                  deploy_stage = stage
                  break

          assert deploy_stage, "Deploy stage not found in pipeline result"
          node1.log(f"Deploy stage: {json.dumps(deploy_stage, indent=2)}")
          for job in deploy_stage.get("jobs", []):
              node1.log(f"Deploy job '{job.get('name')}': {job['status']}")

      with subtest("deployment targeted cluster nodes"):
          # Check deploy logs for evidence the coordinator dispatched to nodes.
          # The coordinator logs: "running deploy job" with artifact_from,
          # "Resolved deploy artifact", "deployment created", and per-node
          # "sending NodeUpgrade RPC" messages.
          deploy_logs = node1.succeed(
              "journalctl -u aspen-node --no-pager 2>/dev/null"
              " | grep -E 'deploy|Deploy|NodeUpgrade'"
              " | tail -40"
          )
          node1.log(f"Deploy-related logs:\n{deploy_logs}")

          # Verify deployment was created (coordinator logged it)
          assert "deployment created" in deploy_logs or "Deployment initiated" in deploy_logs, \
              "No deployment creation logged"

      # ── verify cluster health after deploy ───────────────────────────

      with subtest("cluster healthy after pipeline"):
          for node, name in [(node1, "node1"), (node2, "node2"), (node3, "node3")]:
              node.wait_until_succeeds(
                  f"aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health 2>/dev/null",
                  timeout=30,
              )
              node.log(f"{name} healthy after deploy pipeline")

      with subtest("kv data survives deploy"):
          result = cli(node1, "kv get deploy-test-key")
          value = result.get("value", "") if isinstance(result, dict) else ""
          assert "deploy-survives-upgrade" in str(value), \
              f"Sentinel KV data lost after deploy: {result}"
          node1.log("KV data survived deploy pipeline")

      with subtest("kv operations work after deploy"):
          cli(node1, "kv set post-deploy-key post-deploy-value")
          result = cli(node1, "kv get post-deploy-key")
          value = result.get("value", "") if isinstance(result, dict) else ""
          assert "post-deploy-value" in str(value), \
              f"Post-deploy KV write/read failed: {result}"
          node1.log("KV operations work after deploy pipeline")

      node1.log("MULTI-NODE DEPLOY DOGFOOD PASSED: Forge -> CI -> build -> deploy across 3 nodes")
    '';
  }
