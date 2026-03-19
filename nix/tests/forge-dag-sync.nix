# NixOS VM integration test for Forge DAG sync across nodes.
#
# Exercises the streaming DAG sync protocol that replicates Git objects
# between forge nodes over QUIC. The test flow:
#
#   1. Bootstrap a 2-node cluster with forge + blob features
#   2. Git push to node1 via git-remote-aspen (objects stored locally on node1)
#   3. Verify refs are visible on node2 (Raft-replicated)
#   4. Git clone from node2 via git-remote-aspen (requires DAG sync or
#      on-demand blob fetch to make objects available on node2)
#   5. Verify clone contents match the original push
#   6. Incremental push: add a second commit, push, verify node2 sees it
#
# This tests the full pipeline: git-remote-aspen → forge blob store →
# gossip announcement → DAG sync worker → cross-node blob availability.
#
# Run:
#   nix build .#checks.x86_64-linux.forge-dag-sync-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.forge-dag-sync-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
}: let
  # Deterministic Iroh secret keys (64 hex chars = 32 bytes each).
  secretKey1 = "1111111111111111111111111111111111111111111111111111111111111111";
  secretKey2 = "2222222222222222222222222222222222222222222222222222222222222222";

  # Shared cluster cookie.
  cookie = "dag-sync-vm-test";

  # WASM plugin helpers
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

  mkNodeConfig = {
    nodeId,
    secretKey,
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
      logLevel = "info,aspen=debug";
      relayMode = "disabled";
      enableWorkers = false;
      enableCi = false;
      features = ["forge" "blob"];
    };

    environment.systemPackages = [aspenCliPackage gitRemoteAspenPackage pkgs.git];

    networking.firewall.enable = false;

    virtualisation.memorySize = 4096;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "forge-dag-sync";
    skipLint = true;
    skipTypeCheck = true;

    nodes = {
      node1 = mkNodeConfig {
        nodeId = 1;
        secretKey = secretKey1;
      };
      node2 = mkNodeConfig {
        nodeId = 2;
        secretKey = secretKey2;
      };
    };

    testScript = ''
      import json
      import re
      import time

      # ── helpers ──────────────────────────────────────────────────────

      def get_ticket(node):
          """Read the cluster ticket from a node."""
          return node.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(node, cmd, check=True):
          """Run aspen-cli --json with node1's ticket."""
          ticket = get_ticket(node1)
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
          """Run aspen-cli (human output) with node1's ticket."""
          ticket = get_ticket(node1)
          node.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node.succeed("cat /tmp/_cli_out.txt")

      def get_endpoint_addr_json(node):
          """Extract EndpointAddr JSON from node's journal."""
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
          return json.dumps({"id": eid, "addrs": addrs})

      def wait_for_healthy(node, timeout=60):
          """Wait for a node to become healthy."""
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
          ticket = get_ticket(node)
          node.wait_until_succeeds(
              f"aspen-cli --ticket '{ticket}' cluster health 2>/dev/null",
              timeout=timeout,
          )

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      for node in [node1, node2]:
          node.wait_for_unit("aspen-node.service")
          node.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      wait_for_healthy(node1)

      addr2_json = get_endpoint_addr_json(node2)

      # ── cluster formation (2-node) ──────────────────────────────────

      with subtest("initialize 2-node cluster"):
          cli_text(node1, "cluster init")
          time.sleep(2)

          cli_text(node1, f"cluster add-learner --node-id 2 --addr '{addr2_json}'")
          time.sleep(3)

          cli_text(node1, "cluster change-membership 1 2")
          time.sleep(3)

          status = cli(node1, "cluster status")
          voters = [n for n in status.get("nodes", []) if n.get("is_voter")]
          assert len(voters) == 2, f"expected 2 voters, got {len(voters)}"
          node1.log("2-node cluster formed")

      # ── install WASM plugins ─────────────────────────────────────────
      ${pluginHelpers.installPluginsScript}

      if not _plugins_loaded:
          node1.log("WASM plugins unavailable — skipping DAG sync tests")

      if _plugins_loaded:
        # Stage plugin blobs on node2
        with subtest("stage plugins on node2"):
            node2_ticket = get_ticket(node2)
            for pname in ["kv", "forge"]:
                node2.succeed(
                    f"aspen-cli --ticket '{node2_ticket}' "
                    f"blob add /etc/aspen-plugins/{pname}-plugin.wasm "
                    f">/dev/null 2>/dev/null"
                )
            node2.log("plugin blobs staged on node2")

        # ── create forge repo ────────────────────────────────────────────

        with subtest("create repo for DAG sync test"):
            out = cli(node1, "git init dag-sync-repo --description 'DAG sync test'")
            repo_id = out.get("id") or out.get("repo_id", "")
            assert repo_id, f"repo init failed: {out}"
            node1.log(f"Created repo: {repo_id}")

            time.sleep(2)

            # Verify node2 sees the repo (Raft-replicated metadata)
            repos = cli(node2, "git list")
            repo_names = [r["name"] for r in repos.get("repos", [])]
            assert "dag-sync-repo" in repo_names, \
                f"repo not visible on node2: {repo_names}"

        # ── git push to node1 ────────────────────────────────────────────

        with subtest("git push to node1 via git-remote-aspen"):
            ticket = get_ticket(node1)
            remote_url = f"aspen://{ticket}/{repo_id}"

            # Create a local git repo with multiple files
            node1.succeed(
                "mkdir -p /tmp/dag-source && cd /tmp/dag-source && "
                "git init --initial-branch=main && "
                "git config user.email 'test@test.com' && "
                "git config user.name 'Tester'"
            )
            node1.succeed(
                "cd /tmp/dag-source && "
                "echo '# DAG Sync Test' > README.md && "
                "mkdir -p src && "
                "echo 'fn main() {}' > src/main.rs && "
                "echo '[package]' > Cargo.toml && "
                "echo 'name = \"dag-test\"' >> Cargo.toml && "
                "git add -A && "
                "git commit -m 'Initial commit'"
            )

            source_head = node1.succeed(
                "cd /tmp/dag-source && git rev-parse HEAD"
            ).strip()
            node1.log(f"Source HEAD: {source_head}")

            node1.succeed(
                f"cd /tmp/dag-source && git remote add forge '{remote_url}'"
            )

            rc, _ = node1.execute(
                f"cd /tmp/dag-source && "
                f"RUST_LOG=warn git push forge main "
                f">/tmp/push-stdout.txt 2>/tmp/push-stderr.txt"
            )
            push_stderr = node1.succeed("cat /tmp/push-stderr.txt 2>/dev/null || true")
            node1.log(f"git push exit={rc}, stderr={push_stderr}")
            assert rc == 0, f"git push failed: {push_stderr}"

        # ── verify refs visible on node2 (Raft) ─────────────────────────

        with subtest("refs replicated to node2 via Raft"):
            time.sleep(3)

            ref_out = cli(node2, f"git get-ref -r {repo_id} heads/main")
            node2.log(f"node2 ref: {ref_out}")
            ref_hash = ref_out.get("hash", "")
            assert len(ref_hash) > 0, f"ref not found on node2: {ref_out}"
            node2.log(f"node2 sees heads/main = {ref_hash}")

        # ── verify git objects reachable from node2 ──────────────────────

        with subtest("commit log visible from node2"):
            log_out = cli(node2, f"git log -r {repo_id}")
            count = log_out.get("count", 0)
            assert count >= 1, f"log empty on node2: {log_out}"
            node2.log(f"node2 sees {count} commit(s)")

        # ── git clone from node2 (exercises DAG sync) ────────────────────

        with subtest("git clone from node2"):
            # Use node2's ticket to clone — forces blob reads through node2
            ticket2 = get_ticket(node2)
            remote_url2 = f"aspen://{ticket2}/{repo_id}"

            rc, _ = node2.execute(
                f"RUST_LOG=warn git clone '{remote_url2}' /tmp/dag-cloned "
                f">/tmp/clone-stdout.txt 2>/tmp/clone-stderr.txt"
            )
            clone_stderr = node2.succeed("cat /tmp/clone-stderr.txt 2>/dev/null || true")
            node2.log(f"git clone exit={rc}, stderr={clone_stderr}")
            assert rc == 0, f"git clone from node2 failed: {clone_stderr}"

            # Verify HEAD matches
            clone_head = node2.succeed(
                "cd /tmp/dag-cloned && git rev-parse HEAD"
            ).strip()
            assert source_head == clone_head, \
                f"HEAD mismatch: source={source_head} clone={clone_head}"
            node2.log(f"Clone HEAD matches: {clone_head}")

        with subtest("cloned content matches original"):
            readme = node2.succeed("cat /tmp/dag-cloned/README.md").strip()
            assert readme == "# DAG Sync Test", f"README mismatch: {readme!r}"

            main_rs = node2.succeed("cat /tmp/dag-cloned/src/main.rs").strip()
            assert main_rs == "fn main() {}", f"main.rs mismatch: {main_rs!r}"

            cargo = node2.succeed("cat /tmp/dag-cloned/Cargo.toml")
            assert "dag-test" in cargo, f"Cargo.toml mismatch: {cargo!r}"
            node2.log("All cloned files match original content")

        # ── incremental push + sync ──────────────────────────────────────

        with subtest("incremental push and cross-node sync"):
            node1.succeed(
                "cd /tmp/dag-source && "
                "echo 'pub fn add(a: i32, b: i32) -> i32 { a + b }' > src/lib.rs && "
                "git add -A && "
                "git commit -m 'Add lib module'"
            )
            source_head2 = node1.succeed(
                "cd /tmp/dag-source && git rev-parse HEAD"
            ).strip()
            node1.log(f"Source HEAD after second commit: {source_head2}")

            node1.succeed(
                f"cd /tmp/dag-source && "
                f"RUST_LOG=warn git push forge main 2>/tmp/push2-stderr.txt"
            )

            time.sleep(5)

            # Verify node2 sees the updated ref
            ref_out2 = cli(node2, f"git get-ref -r {repo_id} heads/main")
            node2.log(f"node2 ref after push2: {ref_out2}")

            # Verify commit log has 2 commits
            log_out2 = cli(node2, f"git log -r {repo_id}")
            count2 = log_out2.get("count", 0)
            assert count2 >= 2, f"expected >= 2 commits on node2, got {count2}"
            node2.log(f"node2 sees {count2} commits after incremental push")

        with subtest("fetch incremental changes on node2"):
            # Fetch from node2 into the cloned repo
            ticket2 = get_ticket(node2)
            remote_url2 = f"aspen://{ticket2}/{repo_id}"

            node2.succeed(
                f"cd /tmp/dag-cloned && git remote set-url origin '{remote_url2}'"
            )
            rc, _ = node2.execute(
                f"cd /tmp/dag-cloned && "
                f"RUST_LOG=warn git fetch origin 2>/tmp/fetch-stderr.txt"
            )
            fetch_stderr = node2.succeed("cat /tmp/fetch-stderr.txt 2>/dev/null || true")
            node2.log(f"git fetch exit={rc}, stderr={fetch_stderr}")
            assert rc == 0, f"git fetch failed: {fetch_stderr}"

            node2.succeed("cd /tmp/dag-cloned && git merge origin/main")

            fetch_head = node2.succeed(
                "cd /tmp/dag-cloned && git rev-parse HEAD"
            ).strip()
            assert source_head2 == fetch_head, \
                f"Fetch HEAD mismatch: {source_head2} vs {fetch_head}"

            lib_rs = node2.succeed("cat /tmp/dag-cloned/src/lib.rs").strip()
            assert "add" in lib_rs, f"lib.rs not updated: {lib_rs!r}"
            node2.log("Incremental fetch + merge successful")

        # ── done ─────────────────────────────────────────────────────────
        node1.log("All DAG sync integration tests passed!")
    '';
  }
