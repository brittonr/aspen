## Phase 1: Reproduce and Classify

- [ ] [serial] Re-run or inspect the focused `microvm-cluster-test` failure and capture the exact build-closure mismatch without preserving secrets.
- [ ] [serial] Re-run or inspect the focused `multi-node-cluster-test` failure and capture the exact plugin/fixture API mismatch without preserving cluster tickets.
- [ ] [depends:reproduce] Identify whether each failure belongs to dependency packaging, external fixture source drift, Nix test script drift, or product cluster behavior.

## Phase 2: Repair Stock Rails

- [ ] [depends:microvm-classification] Fix the `microvm-cluster-test` build closure so the VM script runs far enough to exercise the 3-node cluster and AspenFs connection assertions.
- [ ] [depends:multi-node-classification] Fix or isolate the `multi-node-cluster-test` WASM forge/plugin fixture so core cluster formation is not blocked before boot by `ForgeRepoInfo`/`PluginInfo` drift.
- [ ] [parallel] Add or preserve an explicit core-cluster proof marker in the multi-node VM rail before optional plugin/forge subtests.
- [ ] [parallel] Add or preserve failure classification notes/docs so future reports distinguish build-input drift, VM host capability blockers, cluster failures, and optional plugin failures.

## Phase 3: Verify and Land

- [ ] [depends:microvm-fix] Run `nix build --impure .#checks.x86_64-linux.microvm-cluster-test --no-link -L` and record the pass marker or exact remaining boundary.
- [ ] [depends:multi-node-fix] Run `nix build --impure .#checks.x86_64-linux.multi-node-cluster-test --no-link -L` and record the pass marker or exact remaining boundary.
- [ ] [depends:verification] Run `openspec validate --all --strict --json` and `git diff --check`.
- [ ] [depends:verification] Commit the repair with focused evidence and no unredacted tickets/log secrets.
