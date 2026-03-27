## Why

Federation sync updates refs on the local cluster but never emits gossip announcements. The CI `TriggerService` only fires on gossip `RefUpdate` events, so federation pull/push/bidi-sync silently updates mirror repos without triggering builds.

## What Changes

- Federation handlers emit `Announcement::RefUpdate` via gossip after updating mirror refs
- `TriggerService` auto-watches federation mirror repos via periodic KV scan
- Opt-in `federation_ci_enabled` config flag (default `false`)
- NixOS VM integration test for cross-cluster federation→CI flow

## Capabilities

### New Capabilities

- `federation-ci-trigger`: Gossip emission from federation sync, auto-watch mirrors, opt-in CI triggering

### Modified Capabilities

- `forge-ci-trigger`: Startup wires mirror scanning, local and federated announcements treated equally

## Impact

- `crates/aspen-forge/src/node.rs` — `announce_ref_update()` method
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — gossip in `update_mirror_refs`
- `crates/aspen-ci/src/trigger/service.rs` — mirror scanning, federation CI gating
