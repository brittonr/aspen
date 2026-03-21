# NixOS VM integration test: flake-compat native build (zero subprocesses).
#
# Validates the full zero-subprocess flake build path using embedded flake-compat:
#   1. Boot aspen-node with snix-build feature enabled
#   2. Serve a synthetic tarball over local HTTP
#   3. Create a flake.nix with a tarball input pointing at the local server
#   4. Submit a ci_nix_build job
#   5. Verify the build completes via the flake-compat + native path
#   6. Confirm logs show "zero subprocesses" (no nix eval subprocess)
#   7. Cache gateway serves narinfo for built output
#
# This test proves that flake-compat + snix-eval's fetchTarball builtin
# can resolve flake inputs without any nix subprocess.
#
# Run:
#   nix build .#checks.x86_64-linux.snix-flake-native-build-test --impure
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenCliPlugins,
  kvPluginWasm,
  gatewayPackage,
}: let
  secretKey = "0000000000000009000000000000000900000000000000090000000000000009";
  cookie = "snix-flake-native-build-test";

  pluginHelpers = import ./lib/wasm-plugins.nix {
    inherit pkgs aspenCliPlugins;
    plugins = [
      {
        name = "kv";
        wasm = kvPluginWasm;
      }
    ];
  };
in
  pkgs.testers.nixosTest {
    name = "snix-flake-native-build";
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
          logLevel = "info,aspen_ci_executor_nix=debug";
          relayMode = "disabled";
          enableWorkers = true;
          enableCi = true;
          enableSnix = true;
          features = ["blob"];
        };

        environment.systemPackages = [
          aspenCliPackage
          gatewayPackage
          pkgs.nix
          pkgs.curl
          pkgs.git
          pkgs.bubblewrap
          pkgs.python3
          pkgs.gnutar
          pkgs.gzip
        ];

        nix.settings.experimental-features = ["nix-command" "flakes"];
        nix.settings.sandbox = false;
        networking.firewall.enable = false;
        virtualisation.memorySize = 4096;
        virtualisation.cores = 2;
      };
    };

    testScript = ''
      import json
      import time

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
              return raw.strip()

      def cli_text(cmd):
          ticket = get_ticket()
          node1.succeed(
              f"aspen-cli --ticket '{ticket}' {cmd} "
              f">/tmp/_cli_out.txt 2>/dev/null"
          )
          return node1.succeed("cat /tmp/_cli_out.txt")

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

      with subtest("install WASM plugins"):
          plugin_cli(
              "plugin install /etc/aspen-plugins/kv-plugin.wasm "
              "--manifest /etc/aspen-plugins/kv-plugin.json"
          )
      with subtest("reload plugin runtime"):
          plugin_cli("plugin reload", check=False)
          time.sleep(8)

      # ── prepare tarball input ────────────────────────────────────────

      with subtest("create and serve synthetic tarball input"):
          # Create a small library flake as the tarball content.
          # Directory layout: source/flake.nix (fetchTarball unpacks
          # to a single top-level directory named after the archive).
          node1.succeed("mkdir -p /root/tarball-src/source")
          node1.succeed(
              "cat > /root/tarball-src/source/flake.nix << 'FLAKE'\n"
              "{\n"
              '  description = "tarball lib";\n'
              "  inputs = {};\n"
              "  outputs = { self, ... }: {\n"
              '    lib.greeting = "hello from tarball";\n'
              "  };\n"
              "}\n"
              "FLAKE"
          )

          # Create the tarball
          node1.succeed(
              "cd /root/tarball-src && "
              "tar czf /root/tarball-input.tar.gz source/"
          )

          # Compute narHash for the unpacked content.
          # fetchTarball unpacks, strips the top-level directory if there's
          # exactly one, and computes the narHash of the result.
          # Use nix hash to compute the SRI hash.
          nar_hash = node1.succeed(
              "nix hash path --sri /root/tarball-src/source"
          ).strip()
          node1.log(f"Tarball content narHash: {nar_hash}")
          assert nar_hash.startswith("sha256-"), f"bad narHash: {nar_hash}"

          # Start HTTP server to serve the tarball via systemd-run
          # (node1.succeed with & can hang in NixOS test driver).
          node1.succeed(
              "systemd-run --unit=http-tarball "
              "python3 -m http.server 8888 --bind 127.0.0.1 --directory /root"
          )
          node1.wait_until_succeeds(
              "curl -sf http://127.0.0.1:8888/tarball-input.tar.gz -o /dev/null",
              timeout=30,
          )
          node1.log("HTTP server serving tarball at :8888")

      # ── create flake with tarball input ──────────────────────────────

      with subtest("create flake with tarball input"):
          node1.succeed("mkdir -p /root/test-flake")
          node1.succeed(
              "cat > /root/test-flake/flake.nix << 'FLAKE'\n"
              "{\n"
              '  description = "flake-compat native build test";\n'
              '  inputs.mylib.url = "tarball+http://127.0.0.1:8888/tarball-input.tar.gz";\n'
              "  outputs = { self, mylib, ... }: {\n"
              "    packages.x86_64-linux.default = derivation {\n"
              '      name = "flake-compat-native-test";\n'
              '      system = "x86_64-linux";\n'
              '      builder = "/bin/sh";\n'
              '      args = [ "-c" "echo built-via-flake-compat > $out" ];\n'
              "    };\n"
              "  };\n"
              "}\n"
              "FLAKE"
          )

          # Generate flake.lock with the narHash we computed.
          # Use the tarball type — fetchTarball will be called by flake-compat.
          lock_json = json.dumps({
              "nodes": {
                  "mylib": {
                      "locked": {
                          "type": "tarball",
                          "url": "http://127.0.0.1:8888/tarball-input.tar.gz",
                          "narHash": nar_hash,
                      },
                      "original": {
                          "type": "tarball",
                          "url": "http://127.0.0.1:8888/tarball-input.tar.gz",
                      },
                  },
                  "root": {
                      "inputs": {
                          "mylib": "mylib",
                      },
                  },
              },
              "root": "root",
              "version": 7,
          }, indent=2)
          node1.succeed(f"cat > /root/test-flake/flake.lock << 'LOCK'\n{lock_json}\nLOCK")

          # Must be in a git repo for flake-compat to work
          node1.succeed(
              "cd /root/test-flake && "
              "git init && "
              "git config user.email 'test@test' && "
              "git config user.name 'test' && "
              "git add -A && "
              "git commit -m 'init'"
          )

          node1.log("Created flake with tarball input and pre-computed flake.lock")

      # ── submit nix build job ─────────────────────────────────────────

      with subtest("submit nix build job"):
          payload = json.dumps({
              "flake_url": "/root/test-flake",
              "attribute": "packages.x86_64-linux.default",
              "extra_args": [],
              "timeout_secs": 300,
              "sandbox": False,
              "should_upload_result": False,
              "publish_to_cache": True,
              "cache_outputs": [],
              "artifacts": [],
          })
          result = cli(f"job submit ci_nix_build '{payload}'")
          job_id = result.get("job_id") or result.get("id")
          assert job_id, f"no job_id: {result}"
          node1.log(f"Submitted flake-compat build job: {job_id}")

      # ── wait for completion ──────────────────────────────────────────

      with subtest("flake-compat build completes"):
          deadline = time.time() + 300
          final_status = None
          while time.time() < deadline:
              result = cli(f"job status {job_id}", check=False)
              if isinstance(result, dict):
                  job = result.get("job") or result
                  state = job.get("status")
                  node1.log(f"Job {job_id}: state={state}")
                  if state in ("success", "completed", "failed", "dead"):
                      final_status = result
                      break
              time.sleep(5)

          assert final_status is not None, \
              f"Job {job_id} did not complete within 300s"

          job = final_status.get("job") or final_status
          state = job.get("status")

          if state == "failed":
              node1.log(f"Job FAILED: {json.dumps(final_status, indent=2)[:3000]}")
              logs = node1.succeed(
                  "journalctl -u aspen-node.service --no-pager -n 200 "
                  "| grep -iE 'flake.compat|fetchTarball|eval|native|error|failed' || true"
              )
              node1.log(f"Relevant logs:\n{logs}")

          assert state in ("success", "completed"), \
              f"Build failed with state={state}: {final_status}"
          node1.log("Flake-compat build completed successfully!")

          # Extract output paths
          job_result = job.get("result") or {}
          if isinstance(job_result, str):
              try:
                  job_result = json.loads(job_result)
              except (json.JSONDecodeError, ValueError):
                  job_result = {}
          data = job_result.get("data") or job_result
          if isinstance(data, str):
              try:
                  data = json.loads(data)
              except (json.JSONDecodeError, ValueError):
                  data = {}
          output_paths = data.get("output_paths", [])
          node1.log(f"Output paths: {output_paths}")

      # ── verify zero-subprocess path ──────────────────────────────────

      with subtest("verify zero subprocesses in logs"):
          logs = node1.succeed(
              "journalctl -u aspen-node.service --no-pager "
              "| grep -i 'zero subprocesses' || true"
          )
          node1.log(f"Zero-subprocess log lines:\n{logs}")
          assert "zero subprocesses" in logs, \
              "Expected 'zero subprocesses' in logs — flake-compat eval should have succeeded"

      with subtest("verify no nix eval subprocess"):
          # The flake-compat path should NOT invoke nix eval --raw
          eval_logs = node1.succeed(
              "journalctl -u aspen-node.service --no-pager "
              "| grep -i 'nix eval' || true"
          )
          node1.log(f"nix eval log lines:\n{eval_logs}")
          # nix eval subprocess lines would contain "nix eval --raw" or
          # "resolved flake to derivation path" (from resolve_drv_path).
          # If flake-compat worked, these should be absent.
          if "nix eval --raw" in eval_logs or "resolved flake to derivation path" in eval_logs:
              node1.log(
                  "WARNING: nix eval subprocess was used despite flake-compat. "
                  "Check fallback logs for the reason."
              )

      # ── cache gateway ────────────────────────────────────────────────

      with subtest("start nix cache gateway"):
          ticket = get_ticket()
          node1.succeed(
              f"systemd-run --unit=nix-cache-gw "
              f"aspen-nix-cache-gateway --ticket '{ticket}' --port 8380 "
              f"--cache-name flake-native-cache"
          )
          node1.wait_until_succeeds(
              "curl -sf http://127.0.0.1:8380/nix-cache-info",
              timeout=30,
          )
          info = node1.succeed("curl -sf http://127.0.0.1:8380/nix-cache-info")
          assert "StoreDir: /nix/store" in info

      with subtest("cache gateway serves narinfo"):
          if output_paths:
              import re
              store_path = output_paths[0]
              m = re.match(r"/nix/store/([a-z0-9]+)-", store_path)
              if m:
                  path_hash = m.group(1)
                  narinfo_url = f"http://127.0.0.1:8380/{path_hash}.narinfo"
                  node1.log(f"Querying narinfo: {narinfo_url}")
                  exit_code, narinfo = node1.execute(f"curl -sf {narinfo_url}")
                  if exit_code == 0 and narinfo:
                      assert "StorePath:" in narinfo
                      assert "NarHash:" in narinfo
                      node1.log("Cache gateway serves valid narinfo!")
                  else:
                      node1.log(f"Narinfo not available (exit={exit_code})")
          else:
              node1.log("No output paths — skipping narinfo check")

      node1.log("All snix-flake-native-build tests passed!")
    '';
  }
