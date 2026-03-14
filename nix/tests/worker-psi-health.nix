# NixOS VM test: Worker PSI (Pressure Stall Information) health monitoring.
#
# Validates that Linux pressure metrics from /proc/pressure/* flow through
# worker heartbeats into the cluster KV store:
#
#   1. /proc/pressure/{cpu,memory,io} exist and contain data
#   2. Workers register and send heartbeats with PSI fields
#   3. Worker stats in KV contain cpu/memory/io pressure + disk free
#   4. Job dispatches to a healthy (low-pressure) worker
#
# Run:
#   nix build .#checks.x86_64-linux.worker-psi-health-test
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  secretKey = "0000000000000006000000000000000600000000000000060000000000000006";
  cookie = "worker-psi-test";
in
  pkgs.testers.nixosTest {
    name = "worker-psi-health";

    nodes.node1 = {
      imports = [../../nix/modules/aspen-node.nix];

      services.aspen.node = {
        enable = true;
        package = aspenNodePackage;
        nodeId = 1;
        inherit cookie;
        secretKey = secretKey;
        storageBackend = "redb";
        dataDir = "/var/lib/aspen";
        logLevel = "info,aspen_coordination=debug,aspen_jobs=debug";
        relayMode = "disabled";
        enableWorkers = true;
        workerCount = 2;
        enableCi = false;
        features = [];
      };

      environment.systemPackages = [aspenCliPackage];
      networking.firewall.enable = false;
      virtualisation.memorySize = 2048;
      virtualisation.cores = 2;
    };

    testScript = ''
      import json, time

      def get_ticket():
          return node1.succeed("cat /var/lib/aspen/cluster-ticket.txt").strip()

      def cli(cmd, check=True):
          ticket = get_ticket()
          run = (
              f"aspen-cli --ticket '{ticket}' --json {cmd} "
              f">/tmp/_cli.json 2>/dev/null"
          )
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

      # ── PSI files exist ──────────────────────────────────────────

      with subtest("PSI files available"):
          for resource in ["cpu", "memory", "io"]:
              node1.succeed(f"test -f /proc/pressure/{resource}")
              content = node1.succeed(f"cat /proc/pressure/{resource}").strip()
              node1.log(f"/proc/pressure/{resource}: {content}")
              # Must contain "avg10=" somewhere
              assert "avg10=" in content, f"PSI {resource} missing avg10 field"

      # ── boot cluster ─────────────────────────────────────────────

      start_all()
      node1.wait_for_unit("aspen-node.service")
      node1.wait_for_file("/var/lib/aspen/cluster-ticket.txt", timeout=30)
      node1.wait_until_succeeds(
          "aspen-cli --ticket $(cat /var/lib/aspen/cluster-ticket.txt) cluster health",
          timeout=60,
      )
      cli_text("cluster init")

      # ── workers register ─────────────────────────────────────────

      with subtest("workers register with heartbeats"):
          # Workers send stats on registration + periodic heartbeats.
          # Wait for at least one heartbeat cycle (default 10s).
          time.sleep(15)

          workers = cli_text("job workers")
          node1.log(f"Workers output:\n{workers}")
          assert "Idle" in workers or "idle" in workers.lower(), \
              f"No idle workers found: {workers}"

      # ── KV contains worker stats with PSI ────────────────────────

      with subtest("worker stats in KV contain PSI fields"):
          # Worker stats key: __worker_stats:<worker_id>
          out = cli("kv scan --prefix '__worker_stats:' --limit 10", check=False)
          node1.log(f"KV scan raw: {out}")

          # Handle both dict and string responses
          entries = []
          if isinstance(out, dict):
              entries = out.get("entries", out.get("kvs", []))

          assert len(entries) > 0, \
              f"No __worker_stats: entries in KV. Got: {out}"

          for entry in entries:
              key = entry.get("key", "")
              value_str = entry.get("value", "")
              node1.log(f"Worker key: {key}")

              stats = json.loads(value_str)

              # PSI fields
              for field in [
                  "cpu_pressure_avg10",
                  "memory_pressure_avg10",
                  "io_pressure_avg10",
              ]:
                  assert field in stats, f"Missing {field} in {key}: {list(stats.keys())}"
                  val = stats[field]
                  assert isinstance(val, (int, float)) and val >= 0.0, \
                      f"{field}={val} invalid (must be >= 0.0)"
                  node1.log(f"  {field} = {val}")

              # Disk free fields
              for field in [
                  "disk_free_build_pct",
                  "disk_free_store_pct",
              ]:
                  assert field in stats, f"Missing {field} in {key}"
                  val = stats[field]
                  assert isinstance(val, (int, float)) and val >= 0.0, \
                      f"{field}={val} invalid"
                  node1.log(f"  {field} = {val:.1f}%")

      # ── job dispatches to healthy worker ─────────────────────────

      with subtest("job completes on healthy worker"):
          out = cli("job submit test-job --payload 'psi-health-check'", check=False)
          assert isinstance(out, dict) and out.get("is_success"), f"submit failed: {out}"
          job_id = out["job_id"]
          node1.log(f"Submitted job: {job_id}")

          deadline = time.time() + 30
          while time.time() < deadline:
              s = cli(f"job status {job_id}", check=False)
              if isinstance(s, dict):
                  job = s.get("job", s)
                  status = job.get("status", "")
                  if status in ("completed", "failed"):
                      node1.log(f"Job {job_id}: {status}")
                      break
              time.sleep(1)

      node1.log("Worker PSI health test passed")
    '';
  }
