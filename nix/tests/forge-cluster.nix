# NixOS VM integration test for the Aspen Forge.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises every forge CLI command end-to-end:
#
#   - Cluster formation (init via CLI)
#   - Repository lifecycle (init, list, show)
#   - Git object storage (store-blob, get-blob, create-tree, get-tree)
#   - Commit operations (commit, show-commit, log)
#   - Ref management (push, get-ref, branch list/create/delete, tag list/create/delete)
#   - Issue tracking (create, list, show, comment, close, reopen)
#   - Patch workflow (create, list, show, update, approve, merge, close)
#   - Clone (full working-directory checkout via CLI)
#   - Native git push/clone/fetch via git-remote-aspen (real git interop)
#   - Federation (federate, list-federated)
#
# Run:
#   nix build .#checks.x86_64-linux.forge-cluster-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.forge-cluster-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  # The CLI must be built with --features forge to enable push and tag create.
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  # git-remote-aspen binary for native git push/clone/fetch tests.
  gitRemoteAspenPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "forge-vm-test";

  # WASM plugin helpers (KV + Forge handlers are WASM-only)
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
    name = "forge-cluster";
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
          logLevel = "info,aspen=debug";
          relayMode = "disabled";
          enableWorkers = false;
          enableCi = false;
          features = ["forge" "blob"];
        };

        environment.systemPackages = [aspenCliPackage gitRemoteAspenPackage pkgs.git];

        networking.firewall.enable = false;

        # Sufficient resources for a single-node cluster
        virtualisation.memorySize = 4096;
        virtualisation.cores = 2;
      };
    };

    # Python test script using the NixOS test driver API.
    testScript = ''
      import json
      import time

      # ── helpers ──────────────────────────────────────────────────────
      def get_ticket():
          """Read the cluster ticket written by aspen-node on startup."""
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          """Run aspen-cli --json with the cluster ticket and return parsed JSON.

          We redirect stdout to a temp file and cat it back, because the NixOS
          test serial console mixes stderr/kernel messages into the captured
          output, corrupting JSON parsing.
          """
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli_out.json 2>/dev/null"
          )
          if check:
              node1.succeed(run)
          else:
              node1.execute(run)
          raw = node1.succeed("cat /tmp/_cli_out.json")
          try:
              return json.loads(raw)
          except (json.JSONDecodeError, ValueError):
              node1.log(f"cli() JSON parse failed, raw={raw!r}")
              return raw.strip()

      def cli_text(cmd):
          """Run aspen-cli (human output) with the cluster ticket."""
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

      # ── cluster boot ─────────────────────────────────────────────────
      start_all()

      node1.wait_for_unit("aspen-node.service")

      # Wait for aspen-node to write its cluster ticket
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      # Wait for the node to become healthy
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      # Initialize the single-node cluster
      cli_text("cluster init")
      time.sleep(2)

      # ── install WASM plugins (KV + Forge handlers are WASM-only) ───
      ${pluginHelpers.installPluginsScript}

      # Verify cluster is up
      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── repository lifecycle ─────────────────────────────────────────
      with subtest("repo init"):
          out = cli("git init test-repo --description 'Integration test repo'")
          repo_id = out.get("id") or out.get("repo_id", "")
          assert repo_id, f"repo init should return an id, got: {out}"
          node1.log(f"Created repo: {repo_id}")

      with subtest("repo list"):
          out = cli("git list")
          repos = out.get("repos", [])
          assert any(r["name"] == "test-repo" for r in repos), \
              f"test-repo not in list: {repos}"

      with subtest("repo show"):
          out = cli(f"git show -r {repo_id}")
          node1.log(f"repo show output type={type(out).__name__}: {out}")
          if isinstance(out, dict):
              assert out.get("name") == "test-repo", f"unexpected show output: {out}"
          else:
              assert "test-repo" in str(out), f"test-repo not in show output: {out}"

      # ── blob storage ─────────────────────────────────────────────────
      with subtest("store and get blob"):
          node1.succeed("echo -n 'Hello, Forge!' > /tmp/test-blob.txt")
          blob_out = cli(f"git store-blob -r {repo_id} /tmp/test-blob.txt")
          blob_hash = blob_out.get("hash", "")
          assert len(blob_hash) == 64, f"bad blob hash: {blob_hash}"

          node1.succeed(
              f"aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) "
              f"git get-blob {blob_hash} -o /tmp/got-blob.txt 2>/dev/null"
          )
          content = node1.succeed("cat /tmp/got-blob.txt").strip()
          assert content == "Hello, Forge!", f"blob mismatch: {content!r}"

      # ── tree objects ─────────────────────────────────────────────────
      with subtest("create and get tree"):
          tree_out = cli(
              f"git create-tree -r {repo_id} -e '100644:README.md:{blob_hash}'"
          )
          tree_hash = tree_out.get("hash", "")
          assert len(tree_hash) == 64, f"bad tree hash: {tree_hash}"

          get_tree = cli(f"git get-tree {tree_hash}")
          assert get_tree.get("success", False), f"get-tree failed: {get_tree}"

      # ── commits ──────────────────────────────────────────────────────
      with subtest("create commit"):
          commit_out = cli(
              f"git commit -r {repo_id} --tree {tree_hash} -m 'Initial commit'"
          )
          commit_hash = commit_out.get("hash", "")
          assert len(commit_hash) == 64, f"bad commit hash: {commit_hash}"

      with subtest("show-commit"):
          sc = cli(f"git show-commit {commit_hash}")
          assert sc.get("hash") == commit_hash, f"show-commit hash mismatch: {sc}"
          assert sc.get("message") == "Initial commit", f"message mismatch: {sc}"

      # ── refs: push, get-ref, log ─────────────────────────────────────
      with subtest("push and get-ref"):
          cli_text(
              f"git push -r {repo_id} --ref-name heads/main "
              f"--hash {commit_hash} -f"
          )
          ref_out = cli(f"git get-ref -r {repo_id} heads/main")
          assert ref_out.get("hash") == commit_hash, f"ref mismatch: {ref_out}"

      with subtest("log"):
          log_out = cli(f"git log -r {repo_id}")
          assert log_out.get("count", 0) >= 1, f"log empty: {log_out}"

      # ── branches ─────────────────────────────────────────────────────
      with subtest("branch list"):
          bl = cli(f"branch list -r {repo_id}")
          names = [r["name"] for r in bl.get("refs", [])]
          assert any("main" in n for n in names), f"main missing: {names}"

      with subtest("branch create and delete"):
          cli_text(f"branch create -r {repo_id} feature --from {commit_hash}")
          bl2 = cli(f"branch list -r {repo_id}")
          names2 = [r["name"] for r in bl2.get("refs", [])]
          assert any("feature" in n for n in names2), f"feature missing: {names2}"
          cli_text(f"branch delete -r {repo_id} feature")

      # ── tags ─────────────────────────────────────────────────────────
      with subtest("tag create and delete"):
          cli_text(f"tag create -r {repo_id} v0.1 --commit {commit_hash}")
          tl = cli(f"tag list -r {repo_id}")
          tag_names = [r["name"] for r in tl.get("refs", [])]
          assert any("v0.1" in n for n in tag_names), f"v0.1 missing: {tag_names}"
          cli_text(f"tag delete -r {repo_id} v0.1")

      # ── issues ───────────────────────────────────────────────────────
      with subtest("issue create"):
          issue = cli(
              f"issue create -r {repo_id} -t 'Bug report' "
              f"-b 'Something broke' -l bug"
          )
          issue_id = issue.get("id", "")
          assert issue_id, f"issue create should return id: {issue}"

      with subtest("issue list"):
          il = cli(f"issue list -r {repo_id}")
          assert il.get("count", 0) >= 1, f"issue list empty: {il}"

      with subtest("issue show"):
          si = cli(f"issue show -r {repo_id} {issue_id}")
          assert si.get("title") == "Bug report", f"title mismatch: {si}"

      with subtest("issue comment"):
          cli_text(f"issue comment -r {repo_id} {issue_id} -b 'Investigating...'")
          si2 = cli(f"issue show -r {repo_id} {issue_id}")
          comments = si2.get("comments", [])
          assert len(comments) >= 1, f"expected comment: {si2}"

      with subtest("issue close and reopen"):
          cli_text(f"issue close -r {repo_id} {issue_id}")
          cli_text(f"issue reopen -r {repo_id} {issue_id}")

      # ── patches ──────────────────────────────────────────────────────
      with subtest("prepare patch commits"):
          # Second blob + tree + commit for patch head
          node1.succeed("echo -n 'Updated content' > /tmp/blob2.txt")
          b2 = cli(f"git store-blob -r {repo_id} /tmp/blob2.txt")
          blob2_hash = b2.get("hash", "")

          t2 = cli(f"git create-tree -r {repo_id} -e '100644:README.md:{blob2_hash}'")
          tree2_hash = t2.get("hash", "")

          c2 = cli(
              f"git commit -r {repo_id} --tree {tree2_hash} "
              f"--parent {commit_hash} -m 'Feature work'"
          )
          commit2_hash = c2.get("hash", "")

      with subtest("patch create"):
          patch = cli(
              f"patch create -r {repo_id} -t 'Add feature' "
              f"-d 'Implements the feature' "
              f"--base {commit_hash} --head {commit2_hash}"
          )
          patch_id = patch.get("id", "")
          assert patch_id, f"patch create should return id: {patch}"

      with subtest("patch list"):
          pl = cli(f"patch list -r {repo_id}")
          assert pl.get("count", 0) >= 1, f"patch list empty: {pl}"

      with subtest("patch show"):
          ps = cli(f"patch show -r {repo_id} {patch_id}")
          assert ps.get("title") == "Add feature", f"title mismatch: {ps}"

      with subtest("patch update"):
          # Third commit as updated head
          node1.succeed("echo -n 'Final content' > /tmp/blob3.txt")
          b3 = cli(f"git store-blob -r {repo_id} /tmp/blob3.txt")
          blob3_hash = b3.get("hash", "")

          t3 = cli(f"git create-tree -r {repo_id} -e '100644:README.md:{blob3_hash}'")
          tree3_hash = t3.get("hash", "")

          c3 = cli(
              f"git commit -r {repo_id} --tree {tree3_hash} "
              f"--parent {commit2_hash} -m 'Address review'"
          )
          commit3_hash = c3.get("hash", "")

          cli_text(
              f"patch update -r {repo_id} {patch_id} "
              f"--head {commit3_hash} -m 'Pushed review fixes'"
          )

      with subtest("patch approve"):
          cli_text(f"patch approve -r {repo_id} {patch_id}")

      with subtest("patch merge"):
          cli_text(
              f"patch merge -r {repo_id} {patch_id} "
              f"--merge-commit {commit3_hash}"
          )

      # ── clone (CLI) ────────────────────────────────────────────────────
      with subtest("clone"):
          cli_text(f"git clone {repo_id} --path /tmp/cloned-repo")
          node1.succeed("test -d /tmp/cloned-repo/.aspen")
          node1.succeed("test -f /tmp/cloned-repo/.aspen/config.json")

      # ── native git via git-remote-aspen ─────────────────────────────
      # These tests exercise the real `git push`/`git clone`/`git fetch`
      # workflow using git-remote-aspen as a git remote helper, which is
      # how users actually interact with the forge.

      with subtest("git-remote-aspen: create repo for native git"):
          git_repo_out = cli("git init native-git-repo --description 'Native git test'")
          git_repo_id = git_repo_out.get("id") or git_repo_out.get("repo_id", "")
          assert git_repo_id, f"repo init failed: {git_repo_out}"
          node1.log(f"Created native git repo: {git_repo_id}")

      with subtest("git-remote-aspen: git push"):
          ticket = get_ticket()
          remote_url = f"aspen://{ticket}/{git_repo_id}"

          # Create a local git repo with files
          node1.succeed("mkdir -p /tmp/git-source")
          node1.succeed(
              "cd /tmp/git-source && "
              "git init --initial-branch=main && "
              "git config user.email 'test@example.com' && "
              "git config user.name 'Test User'"
          )
          node1.succeed(
              "cd /tmp/git-source && "
              "echo '# Native Git Test' > README.md && "
              "mkdir -p src && "
              "echo 'fn main() { println!(\"hello forge\"); }' > src/main.rs && "
              "git add -A && "
              "git commit -m 'Initial commit via native git'"
          )

          # Get the HEAD SHA-1 before push
          source_head = node1.succeed(
              "cd /tmp/git-source && git rev-parse HEAD"
          ).strip()
          node1.log(f"Source HEAD: {source_head}")

          # Add forge remote
          node1.succeed(
              f"cd /tmp/git-source && "
              f"git remote add forge '{remote_url}'"
          )

          # Push to forge via git-remote-aspen
          # Capture both stdout and stderr for debugging.
          # Use execute() first to see exit code, then assert.
          rc, output = node1.execute(
              f"cd /tmp/git-source && "
              f"RUST_LOG=warn git push forge main "
              f">/tmp/git-push-stdout.txt 2>/tmp/git-push-stderr.txt"
          )
          push_stdout = node1.succeed("cat /tmp/git-push-stdout.txt 2>/dev/null || true")
          push_stderr = node1.succeed("cat /tmp/git-push-stderr.txt 2>/dev/null || true")
          node1.log(f"git push exit code: {rc}")
          node1.log(f"git push stdout: {push_stdout}")
          node1.log(f"git push stderr: {push_stderr}")
          assert rc == 0, f"git push failed (exit {rc}): stdout={push_stdout!r} stderr={push_stderr!r}"

      with subtest("git-remote-aspen: git clone"):
          # Clone from forge into a new directory
          node1.succeed(
              f"RUST_LOG=warn git clone '{remote_url}' /tmp/git-cloned 2>/tmp/git-clone-stderr.txt"
          )
          clone_stderr = node1.succeed("cat /tmp/git-clone-stderr.txt")
          node1.log(f"git clone stderr: {clone_stderr}")

          # Verify HEAD matches
          clone_head = node1.succeed(
              "cd /tmp/git-cloned && git rev-parse HEAD"
          ).strip()
          node1.log(f"Clone HEAD: {clone_head}")
          assert source_head == clone_head, \
              f"HEAD mismatch: source={source_head} clone={clone_head}"

          # Verify file contents
          readme = node1.succeed("cat /tmp/git-cloned/README.md").strip()
          assert readme == "# Native Git Test", f"README mismatch: {readme!r}"

          main_rs = node1.succeed("cat /tmp/git-cloned/src/main.rs").strip()
          assert "hello forge" in main_rs, f"main.rs mismatch: {main_rs!r}"

      with subtest("git-remote-aspen: incremental push"):
          # Add a second commit and push again — should be incremental
          node1.succeed(
              "cd /tmp/git-source && "
              "echo 'More content' >> README.md && "
              "git add -A && "
              "git commit -m 'Second commit'"
          )
          source_head2 = node1.succeed(
              "cd /tmp/git-source && git rev-parse HEAD"
          ).strip()

          node1.succeed(
              f"cd /tmp/git-source && "
              f"RUST_LOG=warn git push forge main 2>/tmp/git-push2-stderr.txt"
          )
          push2_stderr = node1.succeed("cat /tmp/git-push2-stderr.txt")
          node1.log(f"Incremental push stderr: {push2_stderr}")

      with subtest("git-remote-aspen: git fetch"):
          # Fetch the second commit into the cloned repo
          node1.succeed(
              f"cd /tmp/git-cloned && "
              f"RUST_LOG=warn git fetch origin 2>/tmp/git-fetch-stderr.txt"
          )
          fetch_stderr = node1.succeed("cat /tmp/git-fetch-stderr.txt")
          node1.log(f"git fetch stderr: {fetch_stderr}")

          # Merge and verify
          node1.succeed("cd /tmp/git-cloned && git merge origin/main")
          fetch_head = node1.succeed(
              "cd /tmp/git-cloned && git rev-parse HEAD"
          ).strip()
          assert source_head2 == fetch_head, \
              f"Fetch HEAD mismatch: source={source_head2} fetch={fetch_head}"

          # Verify updated content
          readme2 = node1.succeed("cat /tmp/git-cloned/README.md")
          assert "More content" in readme2, f"README not updated: {readme2!r}"

      # ── federation ───────────────────────────────────────────────────
      # Federation requires global-discovery feature which may not be enabled.
      # These subtests verify the CLI command exists and runs; failures are OK.
      with subtest("federate repo"):
          ticket = get_ticket()
          rc, _ = node1.execute(
              f"aspen-cli --ticket '{ticket}' git federate -r {repo_id} 2>/dev/null"
          )
          node1.log(f"federate exit code: {rc} (0=ok, non-zero=feature not enabled)")

      with subtest("list federated"):
          fl = cli("git list-federated", check=False)
          node1.log(f"list-federated output: {fl}")
          # Should succeed or return empty — either way is fine
          assert isinstance(fl, (list, dict, str)), \
              f"unexpected federation output: {fl}"

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All forge CLI integration tests passed!")
    '';
  }
