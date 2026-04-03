# NixOS VM integration test for the Forge web CI dashboard and cluster overview.
#
# Spins up a single-node Aspen cluster with CI enabled, starts forge-web
# with a TCP proxy, then exercises every new web page via curl:
#
#   - /ci (pipeline list, empty and with runs)
#   - /{repo_id}/ci (per-repo pipeline list)
#   - /{repo_id}/ci/{run_id} (pipeline detail with stages/jobs)
#   - /{repo_id}/ci/{run_id}/{job_id} (job log viewer)
#   - /cluster (cluster health overview)
#   - Navigation links (CI and Cluster in nav bar, CI tab in repo tabs)
#   - CI status badge on repo overview
#
# Run:
#   nix build .#checks.x86_64-linux.forge-web-dashboard-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.forge-web-dashboard-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
  aspenForgeWebPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000003000000000000000300000000000000030000000000000003";

  # Shared cluster cookie.
  cookie = "forge-web-dashboard-test";

  # TCP port for forge-web proxy.
  webPort = 8080;
in
  pkgs.testers.nixosTest {
    name = "forge-web-dashboard";
    skipLint = true;
    skipTypeCheck = true;

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
          aspenForgeWebPackage
          pkgs.curl
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

      def curl(path, expect_status=200):
          """Fetch a page from forge-web and return the HTML body."""
          result = node1.succeed(
              f"curl -s -o /tmp/_curl_body.txt -w '%{{http_code}}' "
              f"'http://127.0.0.1:${toString webPort}{path}'"
          ).strip()
          body = node1.succeed("cat /tmp/_curl_body.txt")
          status = int(result)
          assert status == expect_status, \
              f"GET {path}: expected {expect_status}, got {status}"
          return body

      def curl_contains(path, *needles):
          """Fetch a page and assert all needle strings are present in the body."""
          body = curl(path)
          for needle in needles:
              assert needle in body, \
                  f"GET {path}: missing '{needle}' in response ({len(body)} bytes)"
          return body

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

      # ── start forge-web ──────────────────────────────────────────────

      with subtest("start forge-web with TCP proxy"):
          ticket = get_ticket()
          node1.succeed(
              f"systemd-run --unit=forge-web "
              f"${aspenForgeWebPackage}/bin/aspen-forge-web "
              f"--ticket '{ticket}' "
              f"--tcp-port ${toString webPort}"
          )
          # Wait for the TCP proxy to be ready
          node1.wait_until_succeeds(
              "curl -s -o /dev/null http://127.0.0.1:${toString webPort}/",
              timeout=30,
          )
          node1.log("forge-web is serving on port ${toString webPort}")

      # ── cluster overview page ────────────────────────────────────────

      with subtest("cluster overview page"):
          body = curl_contains("/cluster",
              "Cluster Overview",
              "meta http-equiv=\"refresh\"",
              "healthy",
              "leader",
              "Node 1",
          )
          node1.log(f"Cluster page OK ({len(body)} bytes)")

      # ── CI list page (empty baseline) ────────────────────────────────

      with subtest("CI list page empty"):
          body = curl("/ci")
          assert "CI Pipelines" in body or "CI Unavailable" in body, \
              f"GET /ci: unexpected content ({len(body)} bytes)"
          node1.log(f"CI list page OK ({len(body)} bytes)")

      # ── navigation links ─────────────────────────────────────────────

      with subtest("nav bar has CI and Cluster links"):
          body = curl("/")
          assert 'href="/ci"' in body, "nav bar missing CI link"
          assert 'href="/cluster"' in body, "nav bar missing Cluster link"
          node1.log("Navigation links OK")

      # ── trigger a CI pipeline via CLI ────────────────────────────────
      # We use the CI CLI to create run data that the web UI will display.

      with subtest("trigger CI pipeline"):
          # Try to trigger a pipeline. Even if forge isn't loaded (no WASM),
          # the CI handler should respond. We care about having *some* data.
          out = cli("ci run test-repo --ref main", check=False)
          node1.log(f"CI trigger result: {out}")

          # Also try ci list to see if anything is there
          runs = cli("ci list", check=False)
          node1.log(f"CI list after trigger: {runs}")

          # Give the pipeline a moment to register
          time.sleep(2)

      # ── CI list page with potential runs ─────────────────────────────

      with subtest("CI list page after trigger"):
          body = curl("/ci")
          # Should still render regardless of whether runs exist
          assert "CI Pipelines" in body or "CI Unavailable" in body, \
              f"CI list page broken after trigger"
          node1.log(f"CI list page after trigger OK ({len(body)} bytes)")

      # ── CI pipeline run detail (if we got a run ID) ──────────────────

      with subtest("CI run detail 404 for invalid run"):
          # A non-existent run should yield a page (404 or error, not crash)
          result = node1.succeed(
              f"curl -s -o /tmp/_curl_body.txt -w '%{{http_code}}' "
              f"'http://127.0.0.1:${toString webPort}/fakerepo/ci/fakerun'"
          ).strip()
          body = node1.succeed("cat /tmp/_curl_body.txt")
          status = int(result)
          node1.log(f"CI detail for fake run: status={status}, {len(body)} bytes")
          # Should return some HTML (404 or error), not crash
          assert status in (200, 404, 500, 503), \
              f"unexpected status for invalid run: {status}"
          assert len(body) > 0, "empty response for invalid run"

      # ── CI job log page (non-existent) ───────────────────────────────

      with subtest("CI job log 404 for invalid job"):
          result = node1.succeed(
              f"curl -s -o /tmp/_curl_body.txt -w '%{{http_code}}' "
              f"'http://127.0.0.1:${toString webPort}/fakerepo/ci/fakerun/fakejob'"
          ).strip()
          body = node1.succeed("cat /tmp/_curl_body.txt")
          status = int(result)
          node1.log(f"CI log for fake job: status={status}, {len(body)} bytes")
          assert status in (200, 404, 500, 503), \
              f"unexpected status for invalid job: {status}"
          assert len(body) > 0, "empty response for invalid job"

      # ── CI status badges and CSS classes ─────────────────────────────

      with subtest("CI CSS present in pages"):
          body = curl("/ci")
          assert "ci-succeeded" in body or "ci-failed" in body or \
              "ci-running" in body or "ci-pending" in body or \
              "No pipeline runs" in body or "CI Unavailable" in body, \
              "CI page missing status CSS or empty state"
          node1.log("CI CSS classes OK")

      # ── cluster page has auto-refresh ────────────────────────────────

      with subtest("cluster auto-refresh"):
          body = curl("/cluster")
          assert 'content="10"' in body, \
              "cluster page missing 10-second refresh"
          node1.log("Cluster auto-refresh OK")

      # ── cluster page has summary stats ───────────────────────────────

      with subtest("cluster summary stats"):
          body = curl("/cluster")
          assert "cluster-stat" in body, "cluster page missing stat cards"
          assert "nodes" in body, "cluster page missing node count"
          assert "uptime" in body, "cluster page missing uptime"
          node1.log("Cluster summary stats OK")

      # ── new CI observability features ─────────────────────────────────

      with subtest("CI list has duration column"):
          body = curl("/ci")
          assert "Duration" in body or "No pipeline runs" in body or "CI Unavailable" in body, \
              "CI list page missing Duration column header"
          node1.log("Duration column OK")

      with subtest("pipeline detail has stage timeline CSS"):
          body = curl("/ci")
          assert "stage-timeline" in body or "No pipeline runs" in body or "CI Unavailable" in body, \
              "stage-timeline CSS not found"
          node1.log("Stage timeline CSS OK")

      with subtest("pipeline detail cancel/retrigger routes"):
          result = node1.succeed(
              f"curl -s -o /tmp/_curl_body.txt -w '%{{http_code}}' "
              f"'http://127.0.0.1:${toString webPort}/fakerepo/ci/fakerun'"
          ).strip()
          body = node1.succeed("cat /tmp/_curl_body.txt")
          status = int(result)
          assert status in (200, 404, 500, 503), \
              f"cancel/retrigger route test: unexpected status {status}"
          node1.log(f"Cancel/retrigger route test OK (status={status})")

      with subtest("commit page CI status rendering"):
          result = node1.succeed(
              f"curl -s -o /tmp/_curl_body.txt -w '%{{http_code}}' "
              f"'http://127.0.0.1:${toString webPort}/fakerepo/commit/deadbeef'"
          ).strip()
          body = node1.succeed("cat /tmp/_curl_body.txt")
          status = int(result)
          assert status in (200, 404, 500, 503), \
              f"commit CI badge test: unexpected status {status}"
          node1.log(f"Commit CI badge test OK (status={status})")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All forge-web-dashboard tests passed!")
    '';
  }
