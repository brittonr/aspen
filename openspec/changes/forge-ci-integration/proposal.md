## Why

Aspen's self-hosting goal requires three pillars working together: Forge (Git hosting), CI/CD (build/test), and Nix Cache (artifact distribution). Each pillar exists individually — Forge handles `git push aspen://`, CI has orchestrator/triggers/executors, and the Nix cache has snix backend + gateway. But they aren't wired together. A push to Forge doesn't trigger a CI pipeline, and CI builds don't publish to the distributed cache. Until these are connected end-to-end, Aspen can't host itself.

The pieces are closer than they appear. `TriggerService` already subscribes to Forge `Announcement::RefUpdate` gossip. `PipelineOrchestrator` already converts configs to jobs. `NixBuildWorker` already collects artifacts and uploads to blob store. The gap is the glue: a `ConfigFetcher` that reads `.aspen/ci.ncl` from Forge objects, a `PipelineStarter` that drives the orchestrator, and a cache publish step in the Nix executor that registers store paths in the distributed binary cache.

## What Changes

- **Wire Forge → CI trigger path**: Implement `ConfigFetcher` that reads `.aspen/ci.ncl` from Forge git objects at a specific commit, and `PipelineStarter` that calls `PipelineOrchestrator::start_run()`
- **Wire CI → Nix cache publish**: After `NixBuildWorker` completes a build, automatically register output store paths in the distributed Nix binary cache via the snix backend
- **Add `forge.push_completed` hook event**: New `HookEventType` variant emitted after successful push, carrying repo_id + ref + commit hash — gives the trigger service a second integration path beyond gossip
- **Add CI status reporting**: Pipeline run status written to KV with ref-indexed lookups so `aspen-cli ci status <repo> <ref>` shows build results
- **Node startup wiring**: Connect `TriggerService` + `CiTriggerHandler` + `PipelineOrchestrator` in the node startup sequence when both `forge` and `ci` features are enabled
- **End-to-end integration test**: NixOS VM test that does `git push aspen://` → verifies CI pipeline triggers → build completes → Nix store path is available via `nix path-info` against the cache gateway

## Capabilities

### New Capabilities

- `forge-ci-trigger`: Forge push events trigger CI pipeline execution via gossip announcements and hook events
- `ci-cache-publish`: CI Nix builds automatically publish output store paths to the distributed binary cache

### Modified Capabilities

- `ci`: Add pipeline status reporting with ref-indexed KV lookups and CLI integration
- `forge`: Add `push_completed` hook event emission after successful ref updates

## Impact

- **Crates modified**: `aspen-ci` (ConfigFetcher + PipelineStarter impls), `aspen-ci-executor-nix` (cache publish), `aspen-hooks-types` (new event variant), `aspen-forge-handler` (hook emission), `aspen-rpc-handlers` (startup wiring), `aspen-cli` (ci status command), `aspen` (node binary feature gating)
- **New dependencies**: `aspen-ci` gains dependency on `aspen-forge` (for reading git objects via `ForgeNode`), `aspen-ci-executor-nix` gains dependency on `aspen-snix` (for cache registration)
- **Feature flags**: New `ci-forge` compound feature that activates when both `ci` and `forge` are enabled
- **No breaking changes**: All additions are behind feature flags; existing APIs unchanged
