## Why

`handle_git_bridge_push` in `git_bridge.rs` never calls `forge_node.announce_ref_update()` after updating refs. This means git pushes via `git-remote-aspen` never fire `RefUpdate` gossip announcements, so the CI `TriggerService` (registered via `ci watch`) never receives push events. Auto-trigger is dead code for regular pushes.

Both `dogfood-local.sh` and `dogfood-federation.sh` always fall back to manual `ci run` because of this. The federation dogfood specifically fails because the script waits for auto-trigger, times out, then the manual fallback either succeeds or fails depending on timing.

The `announce_ref_update` path exists and works — federation mirror updates in `federation.rs` call it and correctly fire CI triggers. The git push path just never wired it up.

## What Changes

Add `announce_ref_update` calls to `handle_git_bridge_push` after successful ref updates. Also enable `federation_ci_enabled` in the federation dogfood script so mirror repos can auto-trigger CI.

## Capabilities

### New Capabilities

- `git-push-ci-trigger`: Emit RefUpdate gossip announcements from git push handler so CI auto-trigger fires without manual fallback

### Modified Capabilities

## Impact

- `crates/aspen-forge-handler/src/handler/handlers/git_bridge.rs`: Add announce calls after ref updates
- `scripts/dogfood-federation.sh`: Set `ASPEN_CI_FEDERATION_CI_ENABLED=true` for bob
- Both dogfood scripts can remove manual trigger fallback dependency (though keeping it as safety net is fine)
