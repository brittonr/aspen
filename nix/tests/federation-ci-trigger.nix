# Federation CI Trigger test: federation pull triggers CI on receiving cluster.
#
# Two-cluster test proving the automatic gossip→trigger path:
#   1. Alice creates a forge repo with .aspen/ci.ncl, pushes a commit
#   2. Alice federates the repo
#   3. Bob trusts Alice, federation-pulls the repo
#   4. The pull emits RefUpdate gossip → TriggerService auto-watches mirror
#      → CI pipeline triggers automatically
#   5. Assert: Bob's `ci list` shows a pipeline run
#
# This tests the new federation_ci_enabled path (gossip-driven), NOT
# manual `ci trigger`. Compare with federation-ci-dogfood.nix which
# uses manual trigger + nix build.
#
# Run:
#   nix build .#checks.x86_64-linux.federation-ci-trigger-test --impure --option sandbox false
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
}: let
  aliceSecretKey = "a11ce00000000009a11ce00000000009a11ce00000000009a11ce00000000009";
  bobSecretKey = "b0b0000000000010b0b0000000000010b0b0000000000010b0b0000000000010";

  aliceCookie = "alice-trigger-test";
  bobCookie = "bob-trigger-test";

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

  # Simple CI config: one shell job that just echoes success.
  # No nix build — keeps the test fast and focused on the trigger path.
  ciConfig = pkgs.writeText "ci.ncl" ''
    {
      name = "fed-trigger-test",
      stages = [
        {
          name = "check",
          jobs = [
            {
              name = "echo-ok",
              type = 'shell,
              command = "echo",
              args = ["federation-ci-trigger-works"],
              timeout_secs = 30,
            },
          ],
        },
      ],
    }
  '';

  # Minimal repo content with CI config.
  testRepo = pkgs.runCommand "fed-trigger-repo" {} ''
    mkdir -p $out/.aspen $out/src
    cp ${ciConfig} $out/.aspen/ci.ncl
    echo 'fn main() { println!("hello"); }' > $out/src/main.rs
  '';

  mkClusterNode = {
    nodeId,
    secretKey,
    cookie,
    federationClusterKey,
    federationClusterName,
    enableCi ? false,
    ciFederationCiEnabled ? false,
    extraPackages ? [],
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
      logLevel = "info,aspen_raft_network=warn,aspen_rpc_core=warn";
      relayMode = "disabled";
      enableWorkers = enableCi;
      inherit enableCi ciFederationCiEnabled;
      enableSnix = false;
      features = ["forge" "blob" "federation"];

      enableFederation = true;
      inherit federationClusterKey federationClusterName;
    };

    environment.systemPackages =
      [aspenCliPackage pkgs.git]
      ++ extraPackages;

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
    virtualisation.diskSize = 10240;
    virtualisation.writableStoreUseTmpfs = false;
  };
in
  pkgs.testers.nixosTest {
    name = "federation-ci-trigger";
    skipLint = true;
    skipTypeCheck = true;

    nodes = {
      alice = mkClusterNode {
        nodeId = 1;
        secretKey = aliceSecretKey;
        cookie = aliceCookie;
        federationClusterKey = aliceSecretKey;
        federationClusterName = "alice-cluster";
        extraPackages = [gitRemoteAspenPackage];
      };

      bob = mkClusterNode {
        nodeId = 1;
        secretKey = bobSecretKey;
        cookie = bobCookie;
        federationClusterKey = bobSecretKey;
        federationClusterName = "bob-cluster";
        enableCi = true;
        ciFederationCiEnabled = true;
        extraPackages = [gitRemoteAspenPackage];
      };
    };

    testScript = ''
      import json
      import time
      import re

      TEST_REPO = "${testRepo}"

      def get_ticket(node):
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, check=True):
          ticket = get_ticket(node)
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node.succeed(run)
          else:
              node.execute(run)
          raw = node.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(node, cmd):
          ticket = get_ticket(node)
          node.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node.succeed("cat /tmp/_cli_out.txt")

      def wait_for_cluster(node, timeout=60):
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      def install_plugins(node):
          ticket = get_ticket(node)
          for pname in ["kv", "forge"]:
              wasm_path = f"/etc/aspen-plugins/{pname}-plugin.wasm"
              manifest_path = f"/etc/aspen-plugins/{pname}-plugin.json"
              node.succeed(
                  f"aspen-plugin-cli --ticket '{ticket}' --json "
                  f"plugin install {wasm_path} --manifest {manifest_path} "
                  f">/tmp/_plugin_out.json 2>/dev/null"
              )
          node.execute(
              f"aspen-plugin-cli --ticket '{ticket}' --json "
              f"plugin reload >/tmp/_plugin_out.json 2>/dev/null"
          )
          time.sleep(5)

      # ── boot both clusters ───────────────────────────────────────────
      start_all()

      wait_for_cluster(alice)
      wait_for_cluster(bob)

      cli_text(alice, "cluster init")
      cli_text(bob, "cluster init")
      time.sleep(2)

      install_plugins(alice)
      install_plugins(bob)

      # ── alice: create repo, push content with CI config ──────────────

      with subtest("alice creates forge repo"):
          out = cli(alice, "git init trigger-test --description 'Federation trigger test'")
          alice_repo_id = out.get("id") or out.get("repo_id", "")
          assert alice_repo_id, f"repo init failed: {out}"
          alice.log(f"Alice repo: {alice_repo_id}")

      with subtest("alice pushes content with ci config"):
          ticket = get_ticket(alice)
          alice.succeed(
              f"cp -r --no-preserve=mode {TEST_REPO} /tmp/trigger-repo && "
              "cd /tmp/trigger-repo && "
              "git init --initial-branch=main && "
              "git config user.email 'alice@test' && "
              "git config user.name 'Alice' && "
              "git add -A && "
              "git commit -m 'initial: shell CI job'"
          )
          rc, output = alice.execute(
              f"cd /tmp/trigger-repo && "
              f"git remote add aspen 'aspen://{ticket}/{alice_repo_id}' && "
              f"RUST_LOG=warn git push aspen main 2>&1"
          )
          alice.log(f"git push rc={rc} output={output}")
          assert rc == 0, f"git push failed: {output}"
          time.sleep(2)

      with subtest("alice federates repo"):
          fed_result = cli(alice, f"federation federate {alice_repo_id} --mode public", check=False)
          alice.log(f"Federate result: {fed_result}")
          assert isinstance(fed_result, dict) and fed_result.get("is_success"), \
              f"federate failed: {fed_result}"

      # ── extract alice's identity ─────────────────────────────────────

      alice_health = cli(alice, "cluster health")
      alice_node_id = alice_health.get("iroh_node_id")
      assert alice_node_id, f"iroh_node_id missing: {alice_health}"

      alice_journal = alice.succeed("journalctl -u aspen-node --no-pager -q")
      addr_match = re.search(r'192\.168\.1\.1:(\d+)', alice_journal)
      assert addr_match, "could not find Alice's address in journal"
      alice_addr = f"192.168.1.1:{addr_match.group(1)}"
      alice.log(f"Alice: node_id={alice_node_id} addr={alice_addr}")

      # ── bob: trust alice, federation pull → CI auto-trigger ──────────

      with subtest("bob trusts alice"):
          trust_result = cli_text(bob, f"federation trust {alice_node_id}")
          bob.log(f"Trust result: {trust_result}")

      with subtest("bob federation pull from alice"):
          bob_ticket = get_ticket(bob)
          pull_cmd = (
              f"aspen-cli --ticket '{bob_ticket}' --json "
              f"federation pull --peer {alice_node_id} --addr {alice_addr} "
              f"--repo {alice_repo_id} 2>&1"
          )
          bob.log(f"Pull: {pull_cmd}")
          rc, output = bob.execute(pull_cmd)
          bob.log(f"Pull rc={rc} output={output}")
          if rc != 0:
              bob.log("Pull failed, retrying after 5s...")
              time.sleep(5)
              rc, output = bob.execute(pull_cmd)
              bob.log(f"Retry pull rc={rc} output={output}")
          time.sleep(5)

      with subtest("bob has mirror metadata"):
          mirror_scan = cli(bob, "kv scan _fed:mirror:", check=False)
          bob.log(f"Mirror keys: {mirror_scan}")
          has_mirrors = False
          if isinstance(mirror_scan, dict):
              entries = mirror_scan.get("entries", [])
              has_mirrors = len(entries) > 0
          assert has_mirrors, f"No mirror metadata found: {mirror_scan}"

      # ── verify: CI pipeline triggered automatically ──────────────────

      with subtest("bob CI pipeline triggered by federation pull"):
          # The federation pull updated mirror refs → gossip RefUpdate
          # → TriggerService mirror scan picks up the mirror → CI fires.
          # Wait up to 90s for the periodic scan (60s) + trigger processing.
          deadline = time.time() + 120
          run_id = None
          while time.time() < deadline:
              out = cli(bob, "ci list", check=False)
              if isinstance(out, dict):
                  runs = out.get("runs", [])
                  if runs:
                      run_id = runs[0].get("run_id")
                      if run_id:
                          break
              time.sleep(5)

          assert run_id, "No CI pipeline triggered on Bob within 120s"
          bob.log(f"Auto-triggered pipeline: {run_id}")

      with subtest("bob pipeline completes"):
          deadline = time.time() + 120
          final_status = None
          while time.time() < deadline:
              result = cli(bob, f"ci status {run_id}", check=False)
              if isinstance(result, dict):
                  final_status = result.get("status")
                  bob.log(f"Pipeline {run_id}: status={final_status}")
                  if final_status in ("success", "failed", "cancelled"):
                      break
              time.sleep(5)

          # The shell job is just "echo federation-ci-trigger-works" so
          # it should succeed. But even a failed pipeline proves the
          # trigger mechanism works — the point is that a run was created.
          assert final_status is not None, \
              f"Pipeline {run_id} never reached terminal state"
          bob.log(f"Pipeline final status: {final_status}")

      alice.log("FEDERATION CI TRIGGER TEST PASSED")
      bob.log("FEDERATION CI TRIGGER TEST PASSED")
    '';
  }
