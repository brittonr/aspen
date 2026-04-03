# NixOS VM integration test: Durable workflow execution engine.
#
# Runs the `aspen-durable-workflow-test` binary inside a VM, exercising:
#
#   1. Basic workflow lifecycle (start → 3 activities → complete)
#   2. Activity memoization on crash recovery (replay returns cached results)
#   3. Durable timer scheduling and KV persistence
#   4. Saga compensation with event recording (LIFO order)
#   5. Concurrent workflow recovery (5 workflows, all resume after crash)
#   6. Leader failover simulation (on_lose_leadership → on_become_leader)
#
# The test binary uses DeterministicKeyValueStore (in-memory) to exercise
# the DurableWorkflowExecutor without needing a running Raft cluster.
# This validates the executor logic and event sourcing in an isolated VM.
#
# Run:
#   nix build .#checks.x86_64-linux.durable-workflow-failover-test
#
# Interactive debugging:
#   nix build .#checks.x86_64-linux.durable-workflow-failover-test.driverInteractive
#   ./result/bin/nixos-test-driver
{
  pkgs,
  durableWorkflowTestBin,
}:
pkgs.testers.nixosTest {
  name = "durable-workflow-failover";
  skipLint = true;

  nodes = {
    testvm = {
      environment.systemPackages = [durableWorkflowTestBin];

      virtualisation.memorySize = 1024;
      virtualisation.cores = 2;
    };
  };

  testScript = ''
    import time

    start_all()
    testvm.wait_for_unit("multi-user.target")

    # ── Test 1: Basic workflow lifecycle ──
    with subtest("basic workflow: start → 3 activities → complete"):
        output = testvm.succeed(
            "aspen-durable-workflow-test basic 2>&1"
        )
        testvm.log(f"basic: {output[-200:]}")
        assert "PASS" in output, f"basic test failed: {output}"

    # ── Test 2: Activity memoization ──
    with subtest("memoization: recovery returns cached results"):
        output = testvm.succeed(
            "aspen-durable-workflow-test memoization 2>&1"
        )
        testvm.log(f"memoization: {output[-200:]}")
        assert "PASS" in output, f"memoization test failed: {output}"

    # ── Test 3: Durable timer ──
    with subtest("timer: durable sleep with KV persistence"):
        output = testvm.succeed(
            "aspen-durable-workflow-test timer 2>&1"
        )
        testvm.log(f"timer: {output[-200:]}")
        assert "PASS" in output, f"timer test failed: {output}"

    # ── Test 4: Saga compensation ──
    with subtest("saga: compensation events in LIFO order"):
        output = testvm.succeed(
            "aspen-durable-workflow-test saga 2>&1"
        )
        testvm.log(f"saga: {output[-200:]}")
        assert "PASS" in output, f"saga test failed: {output}"

    # ── Test 5: Concurrent workflows ──
    with subtest("concurrent: 5 workflows survive crash recovery"):
        output = testvm.succeed(
            "aspen-durable-workflow-test concurrent 2>&1"
        )
        testvm.log(f"concurrent: {output[-200:]}")
        assert "PASS" in output, f"concurrent test failed: {output}"

    # ── Test 6: Failover simulation ──
    with subtest("failover: leader transition with memoization"):
        output = testvm.succeed(
            "aspen-durable-workflow-test failover 2>&1"
        )
        testvm.log(f"failover: {output[-200:]}")
        assert "PASS" in output, f"failover test failed: {output}"

    testvm.log("=== All durable workflow tests PASSED ===")
  '';
}
