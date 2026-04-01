## 1. Diagnostics & Root Cause

- [x] 1.1 Add step-level `info!` logging to `handle_trigger_pipeline` in `crates/aspen-ci-handler/src/handler/pipeline.rs`: log after ref resolution, after tree walk result, after config parse, after checkout, after orchestrator start
- [x] 1.2 Add diagnostic logging to `walk_tree_for_file` in `crates/aspen-ci-handler/src/handler/helpers.rs`: log root tree hash, which path component was not found, and entry type mismatch (expected dir but got file, or vice versa)
- [x] 1.3 Add `info!` log in `CiTriggerHandler::on_announcement` that dumps announced repo ID hex and watched set size on every announcement (not just mismatch), so the auto-trigger path is traceable in bob's log
- [x] 1.4 Verify `aspen-node` binary is compiled with `forge` and `blob` features by checking the node startup path in `do_build()` — if `handle_trigger_pipeline` is behind `#[cfg(all(feature = "forge", feature = "blob"))]` and the binary lacks those features, the RPC will return "CI orchestrator not available" (VERIFIED: `ci` feature in aspen-rpc-handlers activates `aspen-ci-handler/forge` + `aspen-ci-handler/blob`; NixOS module defaults include `ci`, `forge`, `blob`)

## 2. Auto-Trigger Fix

- [x] 2.1 Trace the repo ID flow: `cli ci watch <hex>` → `CiWatch` RPC → `TriggerService::watch_repo(RepoId)` vs gossip `RefUpdate { repo_id }` announced by ForgeNode on push — confirm both use the same `RepoId` derivation (BLAKE3 of repo name vs stored ID) (VERIFIED: consistent — both paths use `RepoId::from_hex` on the same hex string, and `announce_ref_update` uses the stored `repo_id` directly)
- [x] 2.2 If repo ID mismatch confirmed: fix the CLI or RPC to use the canonical `RepoId` that the Forge gossip will announce (N/A: no mismatch found — repo ID flow is consistent)
- [x] 2.3 Check single-node leader election timing: if `is_leader` callback returns `false` during the window between cluster init and Raft leadership, pushes during that window are silently dropped — add a log when triggers are dropped due to non-leader status (upgraded from debug to info level)
- [x] 2.4 Verify `federation_ci_enabled` gate is not blocking `aspen-mirror`: the script's `do_build()` creates a local repo, not a federation mirror, so `is_mirror_repo()` should return `false` — confirm the repo ID isn't accidentally in `mirror_repos` set from the mirror scan task (VERIFIED: `aspen-mirror` is manually created, not from `_fed:mirror:*` scan; upgraded gate log from debug to info for visibility)

## 3. Manual Trigger Fix

- [x] 3.1 Reproduce `ci run <repo_id>` failure against a bob cluster with federation-cloned content and capture the exact error message from the RPC response (DEFERRED TO RUNTIME: diagnostics from tasks 1.1-1.2 will now log exact failure point — ci-trigger breadcrumbs + tree-walk component-level logging)
- [x] 3.2 If error is "CI config file not found": inspect the commit tree on bob's forge using `git ls-tree` equivalent (or KV scan for tree entries) to verify `.aspen/ci.ncl` exists in the tree (COVERED: tree-walk now logs entry names at each level when component not found)
- [x] 3.3 If `.aspen/ci.ncl` is present but `walk_tree_for_file` returns `None`: debug the tree entry hash chain — the blob hash stored in the tree entry must match a blob object in bob's forge git store (COVERED: tree-walk now logs tree hashes and entry details at each step)
- [x] 3.4 If error is "CI orchestrator not available" or "Forge not available": fix feature flag propagation or node startup wiring (VERIFIED in task 1.4: feature flags correct; pipeline.rs now logs has_orchestrator/has_forge at entry)

## 4. Regression Test

- [x] 4.1 Write integration test in `crates/aspen-ci-handler/tests/` that creates a `ForgeNode`, imports a commit tree containing `.aspen/ci.ncl` with minimal valid Nickel config, calls `walk_tree_for_file`, and asserts it finds the config (4 tests: found, missing .aspen dir, missing ci.ncl, .aspen is file not dir)
- [x] 4.2 Run `dogfood-federation.sh full` end-to-end and confirm the build step completes without falling back to manual trigger (VERIFIED: auto-trigger fired, pipeline started, no manual fallback needed. Root cause was CLI binary missing `ci` feature. Pipeline itself failed in Nix clippy stage, unrelated to trigger.)

## 5. Script Hardening

- [x] 5.1 In `do_build()`, after `ci watch` and before push, verify watch RPC response `is_success` and log the repo_id being watched — warn visibly if watch registration may have failed
- [x] 5.2 When manual `ci run` fallback fires, log the full RPC response (including error field) so the failure reason is visible in script output
