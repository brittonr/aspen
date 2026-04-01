# NixOS VM integration test for Aspen Federated Git Clone.
#
# Two-cluster test: Alice creates a repo, pushes a commit, federates it.
# Bob runs `git clone aspen://<bob-ticket>/fed:<alice-key>:<repo-id>` against
# his own cluster and verifies the working tree matches Alice's content.
#
# Run:
#   nix build .#checks.x86_64-linux.federation-git-clone-test
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  forgePluginWasm,
  gitRemoteAspenPackage,
}: let
  aliceSecretKey = "a11ce00000000005a11ce00000000005a11ce00000000005a11ce00000000005";
  bobSecretKey = "b0b0000000000006b0b0000000000006b0b0000000000006b0b0000000000006";

  aliceCookie = "alice-git-clone-test";
  bobCookie = "bob-git-clone-test";

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

  mkClusterNode = {
    nodeId,
    secretKey,
    cookie,
    federationClusterKey,
    federationClusterName,
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
      logLevel = "info";
      relayMode = "disabled";
      enableWorkers = false;
      enableCi = false;
      features = ["forge" "blob" "federation"];

      enableFederation = true;
      inherit federationClusterKey federationClusterName;
    };

    environment.systemPackages =
      [aspenCliPackage pkgs.git]
      ++ extraPackages;

    networking.firewall.enable = false;

    virtualisation.memorySize = 4096;
    virtualisation.cores = 2;
  };
in
  pkgs.testers.nixosTest {
    name = "federation-git-clone";
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
        extraPackages = [gitRemoteAspenPackage];
      };
    };

    testScript = ''
      import json
      import time
      import re

      # ── helpers ──────────────────────────────────────────────────────

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

      def install_plugins(node, node_name):
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

      install_plugins(alice, "alice")
      install_plugins(bob, "bob")

      # ── alice creates a repo and pushes a commit via CLI ─────────────

      with subtest("alice creates repo"):
          out = cli(alice, "git init fedclone-test --description 'Federated clone test'")
          alice_repo_id = out.get("id") or out.get("repo_id", "")
          assert alice_repo_id, f"repo init failed: {out}"
          alice.log(f"Alice repo: {alice_repo_id}")

      with subtest("alice pushes initial commit via git-remote-aspen"):
          ticket = get_ticket(alice)
          alice.succeed(
              "mkdir -p /tmp/fedclone && cd /tmp/fedclone && "
              "git init -b main && "
              "git config user.email 'alice@test' && "
              "git config user.name 'Alice' && "
              f"git remote add origin 'aspen://{ticket}/{alice_repo_id}' && "
              "echo 'hello from federated clone' > README.md && "
              "echo 'second file content' > data.txt && "
              "git add . && "
              "git commit -m 'initial commit for federated clone'"
          )
          rc, output = alice.execute(
              "cd /tmp/fedclone && git push origin main 2>&1"
          )
          alice.log(f"git push rc={rc} output={output}")
          assert rc == 0, f"git push failed: {output}"
          time.sleep(2)

      with subtest("alice federates repo"):
          fed_result = cli(alice, f"federation federate {alice_repo_id} --mode public", check=False)
          alice.log(f"Federate result: {fed_result}")
          assert isinstance(fed_result, dict) and fed_result.get("is_success"), \
              f"federate failed: {fed_result}"

      # ── extract alice's iroh node ID and address ─────────────────────

      alice_health = cli(alice, "cluster health")
      alice_node_id = alice_health.get("iroh_node_id")
      assert alice_node_id, f"iroh_node_id missing: {alice_health}"

      alice_journal = alice.succeed("journalctl -u aspen-node --no-pager -q")
      addr_match = re.search(r'192\.168\.1\.1:(\d+)', alice_journal)
      assert addr_match, "could not find Alice's address in journal"
      alice_addr = f"192.168.1.1:{addr_match.group(1)}"
      alice.log(f"Alice: node_id={alice_node_id} addr={alice_addr}")

      # ── bob clones via federated URL ─────────────────────────────────

      with subtest("bob clones alice's repo via federated URL"):
          bob_ticket = get_ticket(bob)

          # Build the federated clone URL:
          #   aspen://<bob-ticket>/fed:<alice-node-id>:<alice-repo-id>
          fed_url = f"aspen://{bob_ticket}/fed:{alice_node_id}:{alice_repo_id}"
          bob.log(f"Clone URL: {fed_url}")

          # Set ASPEN_ORIGIN_ADDR so git-remote-aspen can find Alice directly
          clone_cmd = (
              f"ASPEN_ORIGIN_ADDR='{alice_addr}' "
              f"git clone '{fed_url}' /tmp/bob-clone 2>&1"
          )
          bob.log(f"Running: {clone_cmd}")
          rc, output = bob.execute(clone_cmd)
          bob.log(f"git clone rc={rc} output={output}")

          # Give it time — first clone does a federation sync
          if rc != 0:
              bob.log("Clone failed, retrying after 10s...")
              time.sleep(10)
              bob.execute(f"rm -rf /tmp/bob-clone")
              rc, output = bob.execute(clone_cmd)
              bob.log(f"Retry git clone rc={rc} output={output}")

          assert rc == 0, f"git clone failed with rc={rc}: {output}"

      with subtest("verify cloned content matches"):
          readme = bob.succeed("cat /tmp/bob-clone/README.md").strip()
          assert readme == "hello from federated clone", \
              f"README.md mismatch: {readme!r}"

          data = bob.succeed("cat /tmp/bob-clone/data.txt").strip()
          assert data == "second file content", \
              f"data.txt mismatch: {data!r}"

          bob.log("Cloned content verified!")

      # ── large repo: 120+ files in nested dirs ───────────────────────

      with subtest("alice creates large repo with nested dirs"):
          out = cli(alice, "git init large-repo --description 'Large nested repo'")
          large_repo_id = out.get("id") or out.get("repo_id", "")
          assert large_repo_id, f"large repo init failed: {out}"
          alice.log(f"Large repo: {large_repo_id}")

      with subtest("alice pushes 120+ files in nested structure"):
          ticket = get_ticket(alice)
          # Create 120+ files: src/{a..z}/{1..4}.txt + docs/{a..j}.md + lib/{a..f}/{x,y}.rs
          alice.succeed(
              "mkdir -p /tmp/largerepo && cd /tmp/largerepo && "
              "git init -b main && "
              "git config user.email 'alice@test' && "
              "git config user.name 'Alice' && "
              f"git remote add origin 'aspen://{ticket}/{large_repo_id}' && "
              "echo '# Large Repo' > README.md"
          )
          # src/{a..z}/{1..4}.txt = 104 files
          for c in "abcdefghijklmnopqrstuvwxyz":
              alice.succeed(f"mkdir -p /tmp/largerepo/src/{c}")
              for n in range(1, 5):
                  alice.succeed(
                      f"echo 'content of src/{c}/{n}.txt — line 1' > /tmp/largerepo/src/{c}/{n}.txt && "
                      f"echo 'unique id: {c}{n}' >> /tmp/largerepo/src/{c}/{n}.txt"
                  )
          # docs/{a..j}.md = 10 files
          alice.succeed("mkdir -p /tmp/largerepo/docs")
          for c in "abcdefghij":
              alice.succeed(
                  f"echo '# Doc {c}' > /tmp/largerepo/docs/{c}.md && "
                  f"echo 'Documentation content for {c}' >> /tmp/largerepo/docs/{c}.md"
              )
          # lib/{a..f}/{x,y}.rs = 12 files
          for c in "abcdef":
              alice.succeed(f"mkdir -p /tmp/largerepo/lib/{c}")
              for name in ["x", "y"]:
                  alice.succeed(
                      f"echo '// lib/{c}/{name}.rs' > /tmp/largerepo/lib/{c}/{name}.rs && "
                      f"echo 'pub fn {name}_{c}() -> u32 {{ 42 }}' >> /tmp/largerepo/lib/{c}/{name}.rs"
                  )
          # Total: 1 README + 104 src + 10 docs + 12 lib = 127 files
          alice.succeed(
              "cd /tmp/largerepo && git add . && "
              "git commit -m 'initial: 127 files in nested dirs'"
          )
          rc, output = alice.execute(
              "cd /tmp/largerepo && git push origin main 2>&1"
          )
          alice.log(f"large repo push rc={rc} output={output}")
          assert rc == 0, f"large repo push failed: {output}"
          time.sleep(3)

      with subtest("alice federates large repo"):
          fed_result = cli(alice, f"federation federate {large_repo_id} --mode public", check=False)
          assert isinstance(fed_result, dict) and fed_result.get("is_success"), \
              f"federate large repo failed: {fed_result}"

      with subtest("bob clones large repo via federation"):
          bob_ticket = get_ticket(bob)
          fed_url = f"aspen://{bob_ticket}/fed:{alice_node_id}:{large_repo_id}"
          bob.log(f"Large repo clone URL: {fed_url}")

          clone_cmd = (
              f"ASPEN_ORIGIN_ADDR='{alice_addr}' "
              f"git clone '{fed_url}' /tmp/bob-large-clone 2>&1"
          )
          rc, output = bob.execute(clone_cmd)
          bob.log(f"large repo clone rc={rc} output={output}")

          if rc != 0:
              bob.log("Large clone failed, retrying after 15s...")
              time.sleep(15)
              bob.execute("rm -rf /tmp/bob-large-clone")
              rc, output = bob.execute(clone_cmd)
              bob.log(f"Retry large clone rc={rc} output={output}")

          assert rc == 0, f"large repo clone failed: {output}"

      with subtest("verify large repo file count and contents"):
          # Count files (excluding .git)
          file_count = int(bob.succeed(
              "find /tmp/bob-large-clone -not -path '*/.git/*' -type f | wc -l"
          ).strip())
          bob.log(f"Large repo file count: {file_count}")
          assert file_count >= 127, f"Expected 127+ files, got {file_count}"

          # Spot-check specific files
          content = bob.succeed("cat /tmp/bob-large-clone/src/a/1.txt").strip()
          assert "unique id: a1" in content, f"src/a/1.txt mismatch: {content!r}"

          content = bob.succeed("cat /tmp/bob-large-clone/src/z/4.txt").strip()
          assert "unique id: z4" in content, f"src/z/4.txt mismatch: {content!r}"

          content = bob.succeed("cat /tmp/bob-large-clone/docs/e.md").strip()
          assert "Documentation content for e" in content, f"docs/e.md mismatch: {content!r}"

          content = bob.succeed("cat /tmp/bob-large-clone/lib/c/y.rs").strip()
          assert "y_c" in content, f"lib/c/y.rs mismatch: {content!r}"

          bob.log("Large repo content verified!")

      # ── done ─────────────────────────────────────────────────────────
      alice.log("Federation git clone test passed!")
      bob.log("Federation git clone test passed!")
    '';
  }
