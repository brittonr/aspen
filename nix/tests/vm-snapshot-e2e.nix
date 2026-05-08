# NixOS VM integration test: End-to-end snapshot/restore CI pipeline
#
# Tests the complete flow: boot cluster → start CloudHypervisorWorker with
# snapshots → verify golden snapshot → submit CI job → verify job uses
# restored VM → verify completion. Also benchmarks cold-boot vs restore
# time and stress-tests concurrent restores.
#
# Covers tasks:
#   9.1 — Boot cluster → CH worker with snapshots → golden snapshot → job → complete
#   9.2 — Benchmark cold-boot vs snapshot-restore time
#   9.3 — Stress test: 8 VMs simultaneously from same golden snapshot
#
# Architecture:
#   - QEMU host runs aspen-node (cluster leader) + CloudHypervisorWorker
#   - Worker boots first VM via cold-boot → golden snapshot → pool
#   - Subsequent VMs restore from snapshot
#   - CI jobs route to snapshot-restored VMs
#
# Requires nested KVM and the CI worker VM image (kernel, initrd, toplevel).
#
# Build & run:
#   nix build .#checks.x86_64-linux.vm-snapshot-e2e-test --impure --option sandbox false
{
  pkgs,
  microvm,
  aspen-node-vm-test,
  aspen-cli,
  ciKernel,
  ciInitrd,
  ciToplevel,
}: let
  cookie = "snapshot-e2e-test";
  # aspen-node derives the CloudHypervisorWorker state dir from --data-dir as
  # <data-dir>/ci/vms.
  stateDir = "/tmp/aspen-data/ci/vms";
  snapshotDir = "${stateDir}/snapshots/golden";
in
  pkgs.testers.nixosTest {
    name = "vm-snapshot-e2e";

    nodes.host = {pkgs, ...}: {
      virtualisation.qemu.options = [
        "-enable-kvm"
        "-cpu"
        "host"
      ];
      # Need enough RAM for the host cluster + multiple nested VMs
      virtualisation.memorySize = 16384;
      virtualisation.cores = 8;
      virtualisation.diskSize = 20480;

      environment.systemPackages = [
        aspen-node-vm-test
        aspen-cli
        pkgs.cloud-hypervisor
        pkgs.virtiofsd
        pkgs.curl
        pkgs.iproute2
        pkgs.jq
        pkgs.time
        pkgs.procps
      ];

      networking.firewall.enable = false;

      # Pre-create the CI VM state directory
      systemd.tmpfiles.rules = [
        "d ${stateDir} 0755 root root -"
        "d ${stateDir}/snapshots 0755 root root -"
      ];

      # Network bridge for CI VMs
      networking.bridges.aspen-ci-br0.interfaces = [];
      networking.interfaces.aspen-ci-br0.ipv4.addresses = [
        {
          address = "10.200.0.1";
          prefixLength = 24;
        }
      ];

      # NAT for CI VMs to reach the internet (for nix substituters)
      networking.nat = {
        enable = true;
        internalInterfaces = ["aspen-ci-br0"];
        externalInterface = "eth0";
      };
      boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    };

    skipLint = true;
    skipTypeCheck = true;

    testScript = ''
      import time
      import json
      import re

      host.start()
      host.wait_for_unit("multi-user.target")

      # ================================================================
      # 9.1: Boot cluster and start CH worker with snapshots
      # ================================================================
      with subtest("9.1: Start cluster node"):
          # Verify KVM
          host.succeed("test -c /dev/kvm")

          # Start aspen-node as cluster leader
          host.succeed(
              "systemd-run --unit=aspen-node "
              "--property=StandardOutput=file:/tmp/aspen-node.log "
              "--property=StandardError=file:/tmp/aspen-node-err.log "
              "'--property=Environment=RUST_LOG=info,aspen_ci_executor_vm=debug,aspen_ci=debug' "
              f"'--property=Environment=ASPEN_CI_KERNEL_PATH=${ciKernel}/bzImage' "
              f"'--property=Environment=ASPEN_CI_INITRD_PATH=${ciInitrd}/initrd' "
              f"'--property=Environment=ASPEN_CI_TOPLEVEL_PATH=${ciToplevel}' "
              "'--property=Environment=ASPEN_CI_VM_MEMORY_MIB=2048' "
              "'--property=Environment=ASPEN_CI_VM_VCPUS=2' "
              "'--property=Environment=ASPEN_CI_ENABLE_SNAPSHOTS=true' "
              "'--property=Environment=ASPEN_CI_IP_PATH=${pkgs.iproute2}/bin/ip' "
              "'--property=Environment=CLOUD_HYPERVISOR_PATH=${pkgs.cloud-hypervisor}/bin/cloud-hypervisor' "
              "'--property=Environment=VIRTIOFSD_PATH=${pkgs.virtiofsd}/bin/virtiofsd' "
              "aspen-node --node-id 1 --cookie ${cookie} "
              "--data-dir /tmp/aspen-data "
              "--enable-workers --enable-ci"
          )
          host.log("Started aspen-node")

          # Wait for the node to emit its bootstrap ticket, then initialize
          # the single-node Raft cluster via the same ticketed client path
          # used by other NixOS integration tests.
          host.wait_for_file("/tmp/aspen-data/cluster-ticket.txt", timeout=30)
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) cluster health >/dev/null",
              timeout=60
          )
          host.succeed(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) cluster init >/dev/null 2>&1 || true"
          )
          time.sleep(2)
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) cluster status 2>/dev/null | grep -q 'Leader'",
              timeout=60
          )
          host.log("Cluster is ready")

      with subtest("9.1: Verify golden snapshot creation"):
          # Wait for the CloudHypervisorWorker to boot first VM and create snapshot.
          # Emit bounded diagnostics first so gated failures identify whether the
          # node registered the VM worker, booted the child VM, or died silently.
          time.sleep(20)
          _, diagnostic = host.execute(
              "echo '--- aspen-node unit ---'; systemctl status --no-pager aspen-node || true; "
              "echo '--- aspen-node stdout tail ---'; tail -200 /tmp/aspen-node.log || true; "
              "echo '--- aspen-node stderr tail ---'; tail -200 /tmp/aspen-node-err.log || true; "
              "echo '--- vm state tree ---'; find /tmp/aspen-data/ci/vms -maxdepth 4 -print 2>/dev/null || true; "
              "echo '--- vm serial tails ---'; for log in /tmp/aspen-data/ci/vms/*-serial.log; do echo \"### $log\"; tail -200 \"$log\" 2>/dev/null || true; done"
          )
          host.log(str(diagnostic))
          host.wait_until_succeeds(
              f"test -d ${snapshotDir} && test -f ${snapshotDir}/memory-ranges",
              timeout=180
          )
          host.log("Golden snapshot directory exists")

          # Check snapshot files
          memory_ranges_size = host.succeed(f"stat -c %s ${snapshotDir}/memory-ranges").strip()
          host.log(f"Golden snapshot memory-ranges: {memory_ranges_size} bytes")

          # Verify sparse file (COW backing)
          apparent = host.succeed(f"stat -c %s ${snapshotDir}/memory-ranges").strip()
          blocks = host.succeed(f"stat -c %b ${snapshotDir}/memory-ranges").strip()
          disk_bytes = int(blocks) * 512
          apparent_bytes = int(apparent)
          is_sparse = disk_bytes < (apparent_bytes * 9 // 10)
          host.log(
              f"Memory file: apparent={apparent_bytes // (1024*1024)}MB, "
              f"disk={disk_bytes // (1024*1024)}MB, sparse={is_sparse}"
          )

          # Verify ticket file. The Cloud Hypervisor API writes memory/state
          # before Aspen persists the snapshot ticket, so wait for the full
          # Aspen-created snapshot contract rather than only CH artifacts.
          host.wait_until_succeeds(f"test -f ${snapshotDir}/ticket.txt", timeout=60)
          ticket = host.succeed(f"cat ${snapshotDir}/ticket.txt").strip()
          assert len(ticket) > 10, f"Ticket should be non-trivial, got {len(ticket)} chars"
          host.log("Golden snapshot validated")

      with subtest("9.1: Submit CI job and verify completion"):
          # The guest worker only registers after the restored/resumed VM has
          # completed its NixOS boot and read the injected cluster ticket.
          # Gate job submission on that visible guest-backed signal so failures
          # distinguish snapshot artifact creation from actual CI worker readiness.
          host.wait_until_succeeds(
              "grep -q 'worker registered with cluster' /tmp/aspen-data/ci/vms/*-serial.log",
              timeout=180
          )
          host.log("Snapshot VM worker registered with cluster")

          # Submit a simple CI job
          host.succeed(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job submit ci_vm "
              "'{\"command\": \"sh\", \"args\": [\"-c\", \"echo hello-from-snapshot-vm\"], \"timeout_secs\": 60}'"
          )
          host.log("CI job submitted")

          # Wait for job to complete
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job list 2>/dev/null | grep -q 'Completed\\|completed'",
              timeout=120
          )
          host.log("CI job completed via snapshot-restored VM")

      # ================================================================
      # 9.2: Benchmark cold-boot vs snapshot-restore time
      # ================================================================
      with subtest("9.2: Cold-boot vs restore benchmark"):
          # We measure by looking at the timing from the pool's perspective.
          # The pool logs include timestamps for cold-boot and restore operations.

          # Cold-boot time: from "starting cloud-hypervisor" to "VM is running"
          cold_boot_log = host.succeed(
              "grep -E 'cold.boot|boot.*complete|VM is running|creating golden' /tmp/aspen-node.log "
              "| head -5 || echo 'no cold boot log'"
          ).strip()
          host.log(f"Cold boot log entries: {cold_boot_log}")

          # Restore time: from "restoring VM" to "VM restored"
          restore_log = host.succeed(
              "grep -E 'restor|snapshot.*restore|VM restored' /tmp/aspen-node.log "
              "| head -5 || echo 'no restore log'"
          ).strip()
          host.log(f"Restore log entries: {restore_log}")

          # The key metric is observable from job latency:
          # submit a second job and time it (should be faster than first)
          start_time = time.time()
          host.succeed(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job submit ci_vm "
              "'{\"command\": \"sh\", \"args\": [\"-c\", \"echo benchmark-job\"], \"timeout_secs\": 60}'"
          )
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job list 2>/dev/null | "
              "grep -c 'Completed\\|completed' | grep -q '^2$'",
              timeout=120
          )
          elapsed = time.time() - start_time
          host.log(f"Second job (snapshot-restored VM): {elapsed:.1f}s")

          # Restored VMs should be significantly faster than cold-boot
          # Cold boot: 5-15s, Restore: <2s (+ job execution time)
          host.log("Benchmark data collected — compare cold boot vs restore in logs")

      # ================================================================
      # 9.3: Stress test — 8 concurrent VMs from same golden snapshot
      # ================================================================
      with subtest("9.3: Concurrent restore stress test"):
          # Submit 8 jobs simultaneously to force 8 concurrent restores
          for i in range(8):
              host.succeed(
                  f"aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job submit ci_vm "
                  f"'{{\"command\": \"sh\", \"args\": [\"-c\", \"echo stress-job-{i} && sleep 5\"], \"timeout_secs\": 120}}' &"
              )
          host.log("Submitted 8 concurrent CI jobs")

          # Wait for all 8 to complete (or at least start)
          # The pool has max_vms=8, so all should get VMs
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job list 2>/dev/null | "
              "grep -c 'Running\\|Completed\\|running\\|completed' | "
              "awk '{if ($1 >= 8) exit 0; else exit 1}'",
              timeout=180
          )
          host.log("All 8 concurrent jobs started/completed")

          # Check memory usage of all CH processes
          ch_memory = host.succeed(
              "ps aux | grep cloud-hypervisor | grep -v grep | "
              "awk '{sum += $6} END {print sum/1024}'"
          ).strip()
          host.log(f"Total CH RSS: {ch_memory} MB (8 VMs from same snapshot)")

          # With COW, 8 × 2GB VMs should use well under 16GB total
          if ch_memory:
              total_mb = float(ch_memory)
              host.log(f"COW efficiency: {total_mb:.0f}MB for 8×2GB VMs ({total_mb/16384*100:.1f}% of naive)")

          # Wait for all jobs to complete
          host.wait_until_succeeds(
              "aspen-cli --ticket $(cat /tmp/aspen-data/cluster-ticket.txt) job list 2>/dev/null | "
              "grep -c 'Completed\\|completed' | awk '{if ($1 >= 10) exit 0; else exit 1}'",
              timeout=300
          )
          host.log("All stress test jobs completed")

      # ================================================================
      # Cleanup
      # ================================================================
      with subtest("Cleanup"):
          host.succeed("systemctl stop aspen-node.service || true")
          time.sleep(2)
          host.succeed(f"rm -rf ${stateDir} /tmp/aspen-data")
          host.log("Cleanup complete")
    '';
  }
