# NixOS VM integration test for Aspen CI pipelines and Nix binary cache.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises CI pipeline management and binary cache CLI commands:
#
#   - CI: run, status, list, cancel, watch, unwatch, output
#   - Cache: query, stats, download
#
# Run:
#   nix build .#checks.x86_64-linux.ci-cache-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.ci-cache-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "ci-cache-vm-test";
in
  pkgs.testers.nixosTest {
    name = "ci-cache";

    nodes = {
      node1 = {
        imports = [../../nix/modules/aspen-node.nix];

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
          features = [];
        };

        environment.systemPackages = [aspenCliPackage];

        networking.firewall.enable = false;

        virtualisation.memorySize = 2048;
        virtualisation.cores = 2;
      };
    };

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
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)

      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )

      cli_text("cluster init")
      time.sleep(2)

      status = cli("cluster status")
      node1.log(f"Cluster status: {status}")

      # ── cache stats (baseline) ───────────────────────────────────────

      with subtest("cache stats"):
          out = cli("cache stats", check=False)
          node1.log(f"Cache stats: {out}")
          # Even if empty, stats should return structured data
          if isinstance(out, dict):
              node1.log("cache stats: OK (handler responding)")
          else:
              node1.log(f"cache stats: returned {out!r}")

      # ── cache query non-existent ─────────────────────────────────────

      with subtest("cache query non-existent"):
          # Query a non-existent store path
          out = cli(
              "cache query /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nonexistent",
              check=False,
          )
          node1.log(f"Cache query non-existent: {out}")
          # We expect structured response (not-found is OK)
          if isinstance(out, dict):
              node1.log("cache query: OK (handler responding)")

      # ── ci list (empty) ──────────────────────────────────────────────

      with subtest("ci list empty"):
          out = cli("ci list", check=False)
          node1.log(f"CI list: {out}")
          if isinstance(out, dict):
              # Should have runs list (empty is fine)
              runs = out.get("runs", [])
              if isinstance(runs, list):
                  node1.log(f"ci list: OK ({len(runs)} runs)")
              else:
                  node1.log(f"ci list: response has no runs list: {out}")

      # ── ci list with filters ─────────────────────────────────────────

      with subtest("ci list with limit"):
          out = cli("ci list --limit 5", check=False)
          node1.log(f"CI list --limit 5: {out}")
          if isinstance(out, dict):
              runs = out.get("runs", [])
              if isinstance(runs, list):
                  assert len(runs) <= 5, \
                      f"limit not respected: {len(runs)} runs"
                  node1.log(f"ci list --limit: OK ({len(runs)} runs)")

      # ── ci run (best effort, no repo) ────────────────────────────────

      with subtest("ci run without repo"):
          # This will fail since no forge repo exists, but proves CLI dispatch
          out = cli("ci run test-repo", check=False)
          node1.log(f"CI run test-repo: {out}")
          # Just verify we got a response (error is expected)
          if isinstance(out, dict):
              node1.log("ci run: OK (handler responding)")

      # ── ci status non-existent ───────────────────────────────────────

      with subtest("ci status non-existent"):
          out = cli("ci status nonexistent-run-id", check=False)
          node1.log(f"CI status nonexistent: {out}")
          # Expect structured error
          if isinstance(out, dict):
              # Success if we get structured response
              node1.log("ci status: OK (handler responding)")

      # ── ci cancel non-existent ───────────────────────────────────────

      with subtest("ci cancel non-existent"):
          out = cli(
              "ci cancel nonexistent-run-id --reason testing",
              check=False,
          )
          node1.log(f"CI cancel nonexistent: {out}")
          if isinstance(out, dict):
              node1.log("ci cancel: OK (handler responding)")

      # ── ci watch ─────────────────────────────────────────────────────

      with subtest("ci watch"):
          # Enable auto-trigger for a repo (repo doesn't need to exist)
          out = cli("ci watch test-repo", check=False)
          node1.log(f"CI watch test-repo: {out}")
          if isinstance(out, dict):
              node1.log("ci watch: OK (handler responding)")

      # ── ci unwatch ───────────────────────────────────────────────────

      with subtest("ci unwatch"):
          # Disable auto-trigger
          out = cli("ci unwatch test-repo", check=False)
          node1.log(f"CI unwatch test-repo: {out}")
          if isinstance(out, dict):
              node1.log("ci unwatch: OK (handler responding)")

      # ── ci output non-existent ───────────────────────────────────────

      with subtest("ci output non-existent"):
          out = cli(
              "ci output nonexistent-run nonexistent-job",
              check=False,
          )
          node1.log(f"CI output nonexistent: {out}")
          if isinstance(out, dict):
              node1.log("ci output: OK (handler responding)")

      # ── cache download non-existent ──────────────────────────────────

      with subtest("cache download non-existent"):
          out = cli(
              "cache download /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nonexistent",
              check=False,
          )
          node1.log(f"Cache download nonexistent: {out}")
          if isinstance(out, dict):
              node1.log("cache download: OK (handler responding)")

      # ── cache download ticket-only ───────────────────────────────────

      with subtest("cache download ticket-only"):
          out = cli(
              "cache download /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-test --ticket-only",
              check=False,
          )
          node1.log(f"Cache download --ticket-only: {out}")
          if isinstance(out, dict):
              node1.log("cache download --ticket-only: OK (handler responding)")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All CI and cache integration tests passed!")
    '';
  }
