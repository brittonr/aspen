## Why

Federation push lets cluster A send refs and git objects to cluster B. The missing half is pull: cluster A requests refs and objects *from* cluster B by specifying a remote repo. The current `federation pull` CLI command exists but only works on repos already mirrored via `sync --fetch` — there's no way to pull a repo cold from a remote cluster by name. This blocks the workflow where you discover a repo on a peer and want a local mirror without the origin pushing to you.

## What Changes

- **`aspen-cli federation pull --peer <id> --repo <hex> [--addr <hint>]`**: Pull a remote repo by peer + repo ID, creating a local mirror if none exists. Incremental on subsequent calls — sends `have_hashes` of locally stored objects.
- **`aspen-cli federation pull --repo <mirror-id>`** (existing): Still works, reads stored mirror metadata for origin info.
- **Incremental fetch in forge handler**: The `handle_federation_fetch_refs` path already paginates git objects with `have_hashes`. Wire this properly so repeated pulls transfer only deltas.
- **Mirror metadata enrichment**: Store the origin peer's node ID and optional address hint in mirror metadata so future `pull --repo <mirror-id>` can reconnect without re-specifying `--peer`.
- **Integration test**: Two-cluster test that creates a repo on cluster A, pulls it from cluster B, verifies content, pushes a new commit on A, pulls again, verifies incremental.

## Capabilities

### New Capabilities

- `federation-pull`: Pull refs and git objects from a remote cluster into a local mirror repo, with incremental sync on subsequent pulls.

### Modified Capabilities

## Impact

- `crates/aspen-cli/`: Extend `Pull` args with `--peer` and `--addr` flags
- `crates/aspen-client-api/`: Extend `FederationPull` request with optional peer/addr fields
- `crates/aspen-forge-handler/`: Handle cold-pull (no existing mirror) and enrich mirror metadata
- `crates/aspen-federation/`: No wire protocol changes needed — uses existing `SyncObjects` + `GetResourceState`
- `tests/`: New integration test for pull round-trip
