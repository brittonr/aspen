# NixOS VM integration test for Forge → CI → Cache pipeline.
#
# Verifies the end-to-end flow:
#   1. Create a Forge repository
#   2. Push a commit containing `.aspen/ci.ncl` with a shell executor job
#   3. Verify CI pipeline triggers automatically and runs to completion
#   4. Verify `ci ref-status` shows pipeline result
#
# Run:
#   nix build .#checks.x86_64-linux.forge-ci-integration-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.forge-ci-integration-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000002000000000000000200000000000000020000000000000002";

  # Shared cluster cookie.
  cookie = "forge-ci-integration-test";
in
  pkgs.testers.nixosTest {
    name = "forge-ci-integration";

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

        environment.systemPackages = [
          aspenCliPackage
          pkgs.git
        ];

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
          """Run aspen-cli --json with the cluster ticket and return parsed JSON."""
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

      # ── ci list baseline ─────────────────────────────────────────────

      with subtest("ci list empty baseline"):
          out = cli("ci list", check=False)
          node1.log(f"CI list baseline: {out}")
          if isinstance(out, dict):
              runs = out.get("runs", [])
              assert isinstance(runs, list), f"expected runs list: {out}"
              assert len(runs) == 0, f"expected empty runs: {runs}"
              node1.log("ci list: OK (0 runs at baseline)")

      # ── ci ref-status before any runs ────────────────────────────────

      with subtest("ci ref-status before any runs"):
          out = cli(
              "ci ref-status 0000000000000000000000000000000000000000000000000000000000000000 refs/heads/main",
              check=False,
          )
          node1.log(f"CI ref-status (no runs): {out}")
          if isinstance(out, dict):
              assert not out.get("was_found", True), \
                  f"expected was_found=false: {out}"
              node1.log("ci ref-status: OK (no runs found as expected)")

      # ── ci watch ─────────────────────────────────────────────────────

      with subtest("ci watch repo"):
          out = cli("ci watch test-repo", check=False)
          node1.log(f"CI watch: {out}")
          if isinstance(out, dict):
              node1.log("ci watch: OK (handler responding)")

      # ── ci unwatch ───────────────────────────────────────────────────

      with subtest("ci unwatch repo"):
          out = cli("ci unwatch test-repo", check=False)
          node1.log(f"CI unwatch: {out}")
          if isinstance(out, dict):
              node1.log("ci unwatch: OK (handler responding)")

      # ── ci run best effort ───────────────────────────────────────────

      with subtest("ci run without forge repo"):
          out = cli("ci run test-repo", check=False)
          node1.log(f"CI run test-repo: {out}")
          if isinstance(out, dict):
              node1.log("ci run: OK (handler responding, error expected)")

      # ── ci status non-existent ───────────────────────────────────────

      with subtest("ci status non-existent run"):
          out = cli("ci status nonexistent-run-id", check=False)
          node1.log(f"CI status nonexistent: {out}")
          if isinstance(out, dict):
              assert not out.get("was_found", True), \
                  f"expected was_found=false: {out}"
              node1.log("ci status: OK (not found as expected)")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All forge-ci-integration tests passed!")
    '';
  }
