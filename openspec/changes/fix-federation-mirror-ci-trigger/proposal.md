## Why

During the federation clone completeness validation, the full dogfood-federation roundtrip confirmed that all 34,645 git objects survive federation. But CI on the mirror cluster (bob) failed to trigger — both the auto-trigger path (gossip-based `TriggerService`) and the manual fallback (`cli ci run <repo_id>`). The federation clone fix is validated; this is a separate CI-side bug blocking the end-to-end federation dogfood pipeline.

## What Changes

- Diagnose and fix why the auto-trigger path (`TriggerService` gossip handler → `CiTriggerHandler`) doesn't fire for the `aspen-mirror` repo that bob creates and pushes federated content into. Likely causes: `ci watch` registered too late (replay buffer expired), or the pushed repo isn't in `watched_repos` because the script's `ci watch` targets a different repo ID than the one gossip announces.
- Diagnose and fix why the manual `ci run <repo_id>` fallback fails. The handler (`handle_trigger_pipeline`) walks the commit tree to find `.aspen/ci.ncl`. After a federation clone → local push roundtrip, BLAKE3 tree hashes may differ from the original import, or the `.aspen/` directory entry may be dropped/mangled during the clone-to-local-push path.
- Add diagnostic logging and a regression test that exercises the full `ci run` path on a repo populated via federation clone.

## Capabilities

### New Capabilities

- `federation-mirror-ci-trigger`: Covers CI triggering (both auto and manual) for repos populated via federation clone content, including tree-walk `.aspen/ci.ncl` resolution and the `TriggerService` auto-watch/replay path for mirror repos.

### Modified Capabilities

## Impact

- `crates/aspen-ci/src/trigger/service.rs`: Auto-trigger gossip handler, mirror scan, replay buffer timing
- `crates/aspen-ci-handler/src/handler/pipeline.rs`: Manual trigger handler (`handle_trigger_pipeline`), tree walk for `.aspen/ci.ncl`
- `crates/aspen-ci-handler/src/handler/helpers.rs`: `walk_tree_for_file` on federation-imported trees
- `scripts/dogfood-federation.sh`: `do_build()` — repo creation, watch, push, trigger sequencing
- `crates/aspen-cluster/src/config/ci.rs`: `federation_ci_enabled` flag propagation
