## Phase 1: Reproduce and Classify

- [x] [serial] Re-run or inspect the focused `microvm-cluster-test` failure and capture the exact build-closure mismatch without preserving secrets.
  - Classified initial failures as vendored Iroh/portmapper + vendored iroh-gossip packaging drift before the VM script could prove clustering.
- [x] [serial] Re-run or inspect the focused `multi-node-cluster-test` failure and capture the exact plugin/fixture API mismatch without preserving cluster tickets.
  - Classified initial failure as optional plugin/forge build closure drift (`ForgeRepoInfo`/`PluginInfo`, then stale plugin vendor closure), not core Raft clustering behavior.
- [x] [depends:reproduce] Identify whether each failure belongs to dependency packaging, external fixture source drift, Nix test script drift, or product cluster behavior.

## Phase 2: Repair Stock Rails

- [x] [depends:microvm-classification] Fix the `microvm-cluster-test` build closure so the VM script runs far enough to exercise the 3-node cluster and AspenFs connection assertions.
- [x] [depends:multi-node-classification] Fix or isolate the `multi-node-cluster-test` WASM forge/plugin fixture so core cluster formation is not blocked before boot by `ForgeRepoInfo`/`PluginInfo` drift.
- [x] [parallel] Add or preserve an explicit core-cluster proof marker in the multi-node VM rail before optional plugin/forge subtests.
- [x] [parallel] Add or preserve failure classification notes/docs so future reports distinguish build-input drift, VM host capability blockers, cluster failures, and optional plugin failures.

## Phase 3: Verify and Land

- [x] [depends:microvm-fix] Run `nix build --impure .#checks.x86_64-linux.microvm-cluster-test --no-link -L` and record the pass marker or exact remaining boundary.
  - Passed with `=== ALL PHASES PASSED ===`; phases covered 3-node Raft cluster formation and AspenFs VirtioFS daemon connection.
- [x] [depends:multi-node-fix] Run `nix build --impure .#checks.x86_64-linux.multi-node-cluster-test --no-link -L` and record the pass marker or exact remaining boundary.
  - Passed after isolating optional plugin fixture closure; core checks covered cluster bootstrap, 3 voters, cross-node health/leader agreement, leader failover, and rejoin.
- [x] [depends:verification] Run `openspec validate --all --strict --json` and `git diff --check`.
- [x] [depends:verification] Commit the repair with focused evidence and no unredacted tickets/log secrets.
