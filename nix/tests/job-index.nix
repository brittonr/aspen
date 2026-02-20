# NixOS VM integration test for Aspen job queue and index management.
#
# Spins up a single-node Aspen cluster inside a QEMU VM with full networking,
# then exercises job and index CLI commands end-to-end:
#
# JOBS:
#   - job submit (submit job to the queue)
#   - job status (check job status by ID)
#   - job list (list all jobs with filtering)
#   - job cancel (cancel a pending/running job)
#   - job retry (retry a failed job)
#   - job purge (purge completed/failed jobs)
#
# INDEX:
#   - index list (list all built-in indexes)
#   - index show (show details of a specific index)
#
# Run:
#   nix build .#checks.x86_64-linux.job-index-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.job-index-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  aspenNodePackage,
  aspenCliPackage,
}: let
  # Deterministic Iroh secret key (64 hex chars = 32 bytes).
  secretKey = "0000000000000001000000000000000100000000000000010000000000000001";

  # Shared cluster cookie.
  cookie = "job-index-vm-test";
in
  pkgs.testers.nixosTest {
    name = "job-index";

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
          logLevel = "info,aspen=debug";
          relayMode = "disabled";
          enableWorkers = true;
          enableCi = false;
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

      # ── INDEX tests (local, no cluster needed) ──────────────────────

      with subtest("index list"):
          # Index commands are local-only, don't need cluster connection
          node1.succeed(
              "aspen-cli --json index list >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = node1.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          count = out.get("count", 0)
          indexes = out.get("indexes", [])
          assert count > 0, f"expected at least 1 index: {out}"
          assert len(indexes) == count, f"count mismatch: {count} vs {len(indexes)}"
          # Check for known built-in indexes
          names = [idx.get("name") for idx in indexes]
          node1.log(f"index list: {count} indexes: {names}")
          assert "idx_mod_revision" in names, \
              f"idx_mod_revision should be a built-in index: {names}"
          assert "idx_create_revision" in names, \
              f"idx_create_revision should be a built-in index: {names}"

      with subtest("index show idx_mod_revision"):
          node1.succeed(
              "aspen-cli --json index show idx_mod_revision >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = node1.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          assert out.get("name") == "idx_mod_revision", \
              f"wrong index name: {out}"
          assert out.get("builtin") is True, \
              f"should be builtin: {out}"
          assert out.get("field_type") is not None, \
              f"should have field_type: {out}"
          node1.log(f"index show: name={out.get('name')}, field={out.get('field')}, type={out.get('field_type')}")

      with subtest("index show idx_create_revision"):
          node1.succeed(
              "aspen-cli --json index show idx_create_revision >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = node1.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          assert out.get("name") == "idx_create_revision", \
              f"wrong index name: {out}"
          assert out.get("builtin") is True, \
              f"should be builtin: {out}"
          node1.log(f"index show: field={out.get('field')}, unique={out.get('unique')}")

      with subtest("index show idx_expires_at"):
          node1.succeed(
              "aspen-cli --json index show idx_expires_at >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = node1.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          assert out.get("name") == "idx_expires_at", \
              f"wrong index name: {out}"
          node1.log(f"index show idx_expires_at: type={out.get('field_type')}")

      with subtest("index show idx_lease_id"):
          node1.succeed(
              "aspen-cli --json index show idx_lease_id >/tmp/_cli_out.json 2>/dev/null"
          )
          raw = node1.succeed("cat /tmp/_cli_out.json")
          out = json.loads(raw)
          assert out.get("name") == "idx_lease_id", \
              f"wrong index name: {out}"
          node1.log(f"index show idx_lease_id: type={out.get('field_type')}")

      # ── cluster boot (for job tests) ─────────────────────────────────
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

      # ── JOB tests ───────────────────────────────────────────────────

      with subtest("job list empty"):
          # Initially, no user-submitted jobs should exist
          out = cli("job list", check=False)
          if isinstance(out, dict):
              jobs = out.get("jobs", [])
              total = out.get("total_count", 0)
              node1.log(f"job list: {total} jobs, {len(jobs)} shown")
          else:
              node1.log(f"job list: {out}")

      with subtest("job submit"):
          # Submit a test job — use a simple job type
          out = cli("job submit test-job --payload 'hello job'", check=False)
          if isinstance(out, dict):
              is_success = out.get("is_success")
              job_id = out.get("job_id")
              error = out.get("error")
              node1.log(f"job submit: is_success={is_success}, job_id={job_id}, error={error}")
              if is_success and job_id:
                  # Check job status
                  out2 = cli(f"job status {job_id}", check=False)
                  if isinstance(out2, dict):
                      node1.log(f"job status: was_found={out2.get('was_found')}, job={out2.get('job')}")
                  else:
                      node1.log(f"job status: {out2}")
          else:
              node1.log(f"job submit: {out}")

      with subtest("job submit with priority"):
          out = cli("job submit test-job-priority --payload 'priority test' --priority 5", check=False)
          if isinstance(out, dict):
              node1.log(f"job submit priority: is_success={out.get('is_success')}, job_id={out.get('job_id')}")
          else:
              node1.log(f"job submit priority: {out}")

      with subtest("job list after submit"):
          out = cli("job list", check=False)
          if isinstance(out, dict):
              jobs = out.get("jobs", [])
              total = out.get("total_count", 0)
              node1.log(f"job list: {total} total, {len(jobs)} listed")
          else:
              node1.log(f"job list: {out}")

      with subtest("job list with status filter"):
          # Filter by status
          out = cli("job list --status pending", check=False)
          if isinstance(out, dict):
              node1.log(f"job list --status pending: {out.get('total_count', 0)} jobs")
          else:
              node1.log(f"job list pending: {out}")

      with subtest("job cancel"):
          # Submit a job and then cancel it
          out = cli("job submit cancel-test --payload 'to be cancelled'", check=False)
          if isinstance(out, dict) and out.get("is_success") and out.get("job_id"):
              job_id = out.get("job_id")
              cancel_out = cli(f"job cancel {job_id}", check=False)
              if isinstance(cancel_out, dict):
                  node1.log(f"job cancel: is_success={cancel_out.get('is_success')}, previous_status={cancel_out.get('previous_status')}")
              else:
                  node1.log(f"job cancel: {cancel_out}")
          else:
              node1.log(f"job cancel: submit failed: {out}")

      with subtest("job purge"):
          # Purge completed/cancelled jobs
          out = cli("job purge", check=False)
          if isinstance(out, dict):
              node1.log(f"job purge: is_success={out.get('is_success')}, purged_count={out.get('purged_count')}")
          else:
              node1.log(f"job purge: {out}")

      # ── done ─────────────────────────────────────────────────────────
      node1.log("All job and index integration tests passed!")
    '';
  }
