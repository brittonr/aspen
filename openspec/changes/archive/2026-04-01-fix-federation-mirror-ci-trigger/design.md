## Context

The dogfood-federation script (`scripts/dogfood-federation.sh`) runs this sequence on bob's cluster:

1. `git init aspen-mirror` — creates a local Forge repo
2. `ci watch <bob_repo_id>` — tells TriggerService to watch it
3. Federated clone from alice → `/tmp/aspen-fed-clone-$$`
4. `git push bob-forge main` — pushes clone content into bob's local repo
5. Polls `ci list` for 120s waiting for auto-triggered pipeline
6. Falls back to `ci run <bob_repo_id>` if nothing fires

Both step 5 (auto-trigger) and step 6 (manual trigger) fail. Three distinct bugs are likely at play:

**Auto-trigger failure**: The `CiTriggerHandler` receives gossip `RefUpdate` announcements, but the `aspen-mirror` repo is a plain local repo, not a federation mirror. The `scan_and_watch_mirrors` task only auto-watches repos found under `_fed:mirror:*` KV keys. The script does call `ci watch` manually, so the repo should be in `watched_repos`. The likely failure is that `ci watch` RPC resolves to a different repo ID than the one gossip announces (the push goes to bob's local repo ID, which should match — but this needs verification), or the `is_leader` check is rejecting the trigger on single-node clusters if leadership isn't established yet.

**Manual trigger failure**: `handle_trigger_pipeline` calls `walk_tree_for_file` to find `.aspen/ci.ncl` in the commit tree. After a federation clone → `git push` roundtrip, the git objects in bob's forge are re-imported from a working copy checkout. The tree structure should be intact, but if the federation clone produced a shallow or partial checkout (e.g., missing `.aspen/` directory), the file won't be found. The handler returns a non-success response with error "CI config file (.aspen/ci.ncl) not found in repository" — the script treats this as a failure and exits.

**Feature flag gap**: The `handle_trigger_pipeline` function is behind `#[cfg(all(feature = "forge", feature = "blob"))]`. The node binary must be compiled with both features. The dogfood script starts bob with `--enable-workers --enable-ci --ci-auto-trigger`, but these are runtime flags. The compile-time features (`forge`, `blob`) must be in the binary's Cargo build — if the CI handler was compiled without them, the RPC dispatch would fail silently or return "CI orchestrator not available".

## Goals / Non-Goals

**Goals:**

- Auto-trigger fires reliably when content is pushed to a watched repo on bob's cluster
- Manual `ci run` succeeds when `.aspen/ci.ncl` exists in the pushed commit tree
- Diagnostic logging pinpoints which step fails (watch mismatch, tree walk, config parse, orchestrator)
- Regression test covering the `ci run` → tree walk → config parse → pipeline start path for federation-sourced repos

**Non-Goals:**

- Changing the federation clone or import machinery (already fixed and validated)
- Modifying the auto-trigger replay buffer TTL or architecture
- Adding new CI features beyond fixing the existing trigger path

## Decisions

### 1. Instrument `handle_trigger_pipeline` with step-level logging

Each step (ref resolution, tree walk, config parse, checkout, orchestrator start) already returns specific errors, but only the final error surfaces in the CLI output. Add `info!`-level breadcrumbs at each step so `bob/node1.log` reveals exactly where the path dies.

**Rationale**: The dogfood script already captures bob's log. Breadcrumbs cost nothing and prevent future debugging sessions from starting blind.

### 2. Verify repo ID consistency between `ci watch` and gossip

The `ci watch` RPC takes a repo ID string, and the gossip `RefUpdate` carries a `RepoId`. If the CLI sends a hex-encoded ID that gets parsed differently than what the Forge gossip announces, the watched set won't match. Add an assertion/log in `CiTriggerHandler::on_announcement` that dumps both the announced repo ID and the watched set contents on mismatch.

**Rationale**: Repo ID mismatch is silent — the handler just buffers to the replay queue and moves on. Making this visible is the fastest path to confirming or ruling out this hypothesis.

### 3. Add `walk_tree_for_file` diagnostic mode

When tree walk returns `None`, log which step failed: root tree not found, `.aspen` entry missing from root tree, `ci.ncl` entry missing from `.aspen` subtree, or entry found but not a file. Currently all four cases collapse into a single `None`.

**Rationale**: The tree walk is the most likely failure point for federation-sourced content, and the current return type discards the reason.

### 4. Add integration test: `ci run` on federation-cloned repo

Create a test that populates a Forge repo with known content (including `.aspen/ci.ncl`), triggers `CiTriggerPipeline` via the handler, and asserts the pipeline starts. This doesn't need federation — it just needs a repo with a valid tree containing the config file.

**Rationale**: Regression coverage. The existing trigger tests mock `ConfigFetcher` and never exercise the real tree walk path.

## Risks / Trade-offs

- **[Risk]** The root cause turns out to be a compile-time feature flag issue in the dogfood binary → **Mitigation**: Check `aspen-node` binary features in `Cargo.toml` and the Nix build first, before touching Rust code.
- **[Risk]** Fixing auto-trigger for local repos could accidentally fire CI on repos that shouldn't trigger → **Mitigation**: Only change logging and diagnostics, not trigger logic. The `federation_ci_enabled` gate stays for mirror-scanned repos.
- **[Risk]** Tree walk works fine in unit tests but fails on federation content due to hash format differences → **Mitigation**: The integration test should use the real `ForgeNode` import path, not synthetic objects.
