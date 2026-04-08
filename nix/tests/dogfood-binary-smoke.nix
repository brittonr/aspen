# Smoke test for the aspen-dogfood binary.
#
# Exercises the Rust orchestrator directly: start a single-node cluster,
# check status, attempt a push (which exercises Forge repo creation plus
# CiWatchRepo registration before git push), then stop. This is the only VM test that
# invokes aspen-dogfood instead of reimplementing its logic with
# aspen-cli calls.
#
# Run:
#   nix build .#checks.x86_64-linux.dogfood-binary-smoke-test --option sandbox false
{
  pkgs,
  aspenNodePackage,
  aspenDogfoodPackage,
  gitRemoteAspenPackage,
}:
pkgs.testers.nixosTest {
  name = "dogfood-binary-smoke";
  skipLint = true;
  skipTypeCheck = true;

  nodes.machine = {
    environment.systemPackages = [
      aspenDogfoodPackage
      aspenNodePackage
      gitRemoteAspenPackage
      pkgs.git
    ];

    networking.firewall.enable = false;

    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
  };

  testScript = ''
    import json

    DOGFOOD = "${aspenDogfoodPackage}/bin/aspen-dogfood"
    NODE_BIN = "${aspenNodePackage}/bin/aspen-node"
    GIT_REMOTE = "${gitRemoteAspenPackage}/bin/git-remote-aspen"
    CLUSTER_DIR = "/tmp/dogfood-smoke"
    PROJECT_DIR = "/tmp/dogfood-project"

    def dogfood(cmd, check=True):
        """Run aspen-dogfood with standard env."""
        full = (
            f"ASPEN_NODE_BIN={NODE_BIN} "
            f"GIT_REMOTE_ASPEN_BIN={GIT_REMOTE} "
            f"PROJECT_DIR={PROJECT_DIR} "
            f"{DOGFOOD} --cluster-dir {CLUSTER_DIR} {cmd}"
        )
        if check:
            return machine.succeed(f"{full} >/tmp/_df.log 2>&1")
        else:
            rc, _ = machine.execute(f"{full} >/tmp/_df.log 2>&1")
            return rc

    machine.start()
    machine.wait_for_unit("multi-user.target")

    # Prepare a minimal git repo so push has something to work with.
    machine.succeed(
        f"mkdir -p {PROJECT_DIR} && "
        f"cd {PROJECT_DIR} && "
        "git init --initial-branch=main && "
        "git config user.email 'test@test' && "
        "git config user.name 'Test' && "
        "echo 'hello' > README.md && "
        "git add -A && "
        "git commit -m 'init'"
    )

    # ── start ────────────────────────────────────────────────────
    with subtest("aspen-dogfood start creates a running cluster"):
        dogfood("start")

        # State file must exist with correct shape.
        machine.succeed(f"test -f {CLUSTER_DIR}/dogfood-state.json")
        state = json.loads(machine.succeed(f"cat {CLUSTER_DIR}/dogfood-state.json"))
        assert len(state["nodes"]) == 1, f"expected 1 node, got {len(state['nodes'])}"
        assert state["is_federation"] == False, "single-node should not be federation"
        assert state["nodes"][0]["ticket"] != "", "ticket must be non-empty"
        machine.log(f"State OK: 1 node, ticket present")

    # ── status ───────────────────────────────────────────────────
    with subtest("aspen-dogfood status reports reachable node"):
        dogfood("status")
        status_log = machine.succeed("cat /tmp/_df.log")
        status_log_lower = status_log.lower()
        assert "status=healthy" in status_log_lower, \
            f"status output should report a healthy node, got: {status_log!r}"
        assert "node_id=1" in status_log_lower, \
            f"status output should include the node id, got: {status_log!r}"
        machine.log(f"Status OK: {status_log.strip()}")

    # ── push ─────────────────────────────────────────────────────
    with subtest("aspen-dogfood push exercises repo creation and CiWatchRepo"):
        # Push may fail at the git-remote-aspen level, but cmd_push
        # must create the repo, register the CI watch, and only then
        # reach the git push path.
        rc = dogfood("push", check=False)
        push_log = machine.succeed("cat /tmp/_df.log")
        push_log_lower = push_log.lower()
        machine.log(f"Push rc={rc}, log: {push_log.strip()}")

        repo_created_idx = push_log_lower.find("repo created")
        watch_idx = push_log_lower.find("ci watch registered")
        git_push_idx = push_log_lower.find("git push")
        assert repo_created_idx != -1, \
            f"push should create the Forge repo first, got: {push_log!r}"
        assert watch_idx != -1, \
            f"push should register the CI watch before git push, got: {push_log!r}"
        assert git_push_idx != -1, \
            f"push should reach the git push path, got: {push_log!r}"
        assert repo_created_idx < watch_idx < git_push_idx, \
            f"expected repo creation -> watch registration -> git push ordering, got: {push_log!r}"

        if rc == 0:
            machine.log("Push succeeded end-to-end")
        else:
            machine.log("Push failed after ordered repo/watch/git-push steps")

    # ── stop ─────────────────────────────────────────────────────
    with subtest("aspen-dogfood stop cleans up"):
        dogfood("stop")
        machine.succeed(f"test ! -e {CLUSTER_DIR}")
        machine.log("Cluster stopped and cleaned up")
  '';
}
