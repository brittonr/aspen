# NixOS VM integration test: Job orchestration primitives.
#
# Runs the `aspen-job-primitives-test` binary inside a VM, exercising:
#
#   1. Dependency tracker: diamond DAG ordering + upstream failure cascade
#   2. Scheduler: delayed and cron job scheduling, pause/resume
#   3. Dead letter queue: retry exhaustion → DLQ placement, redrive
#   4. Saga executor: LIFO compensation on mid-step failure, success path
#   5. Workflow engine: conditional transitions (success/failure paths)
#   6. Affinity: tag-based routing scores, no-match detection
#   7. Replay: deterministic execution self-consistency
#
# The test binary uses DeterministicKeyValueStore (in-memory) to exercise
# job primitives without needing a running Raft cluster.
#
# Run:
#   nix build .#checks.x86_64-linux.job-primitives-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.job-primitives-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  jobPrimitivesTestBin,
}:
pkgs.testers.nixosTest {
  name = "job-primitives";
  skipLint = true;

  nodes = {
    testvm = {
      environment.systemPackages = [jobPrimitivesTestBin];

      virtualisation.memorySize = 1024;
      virtualisation.cores = 2;
    };
  };

  testScript = ''
    start_all()
    testvm.wait_for_unit("multi-user.target")

    # ── Test 1: Dependency tracker ──
    with subtest("dependency: diamond DAG ordering and failure cascade"):
        output = testvm.succeed(
            "aspen-job-primitives-test dependency 2>&1"
        )
        testvm.log(f"dependency: {output[-200:]}")
        assert "PASS" in output, f"dependency test failed: {output}"

    # ── Test 2: Scheduler ──
    with subtest("scheduler: delayed and cron job scheduling"):
        output = testvm.succeed(
            "aspen-job-primitives-test scheduler 2>&1"
        )
        testvm.log(f"scheduler: {output[-200:]}")
        assert "PASS" in output, f"scheduler test failed: {output}"

    # ── Test 3: Dead letter queue ──
    with subtest("dlq: retry exhaustion and redrive"):
        output = testvm.succeed(
            "aspen-job-primitives-test dlq 2>&1"
        )
        testvm.log(f"dlq: {output[-200:]}")
        assert "PASS" in output, f"dlq test failed: {output}"

    # ── Test 4: Saga executor ──
    with subtest("saga: LIFO compensation on failure"):
        output = testvm.succeed(
            "aspen-job-primitives-test saga 2>&1"
        )
        testvm.log(f"saga: {output[-200:]}")
        assert "PASS" in output, f"saga test failed: {output}"

    # ── Test 5: Workflow engine ──
    with subtest("workflow: conditional transitions"):
        output = testvm.succeed(
            "aspen-job-primitives-test workflow 2>&1"
        )
        testvm.log(f"workflow: {output[-200:]}")
        assert "PASS" in output, f"workflow test failed: {output}"

    # ── Test 6: Affinity ──
    with subtest("affinity: tag-based routing scores"):
        output = testvm.succeed(
            "aspen-job-primitives-test affinity 2>&1"
        )
        testvm.log(f"affinity: {output[-200:]}")
        assert "PASS" in output, f"affinity test failed: {output}"

    # ── Test 7: Replay ──
    with subtest("replay: deterministic execution"):
        output = testvm.succeed(
            "aspen-job-primitives-test replay 2>&1"
        )
        testvm.log(f"replay: {output[-200:]}")
        assert "PASS" in output, f"replay test failed: {output}"

    testvm.log("=== All job primitives tests PASSED ===")
  '';
}
