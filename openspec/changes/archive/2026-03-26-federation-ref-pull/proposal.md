## Why

Federation sync currently discovers remote ref heads and persists them to local KV (`_fed:sync:*:refs/*`), but never fetches the actual git objects behind those refs. The ref hashes are useless without the pack data — you can see *what* changed on a remote cluster but can't read or use any of it locally. This is the missing link between "know about remote repos" and "have a local mirror."

## What Changes

- Add a `federation fetch` CLI command that takes a remote peer and fetches git pack data for synced refs
- The fetch uses the existing `SyncObjects` federation protocol to pull git objects (commits, trees, blobs) that the local cluster doesn't have
- Fetched objects are written into the local Forge repo's object store via iroh-blobs
- Local ref pointers are updated to match the remote heads after objects are verified
- The existing `federation sync` command gains an optional `--fetch` flag to do discovery + fetch in one shot

## Capabilities

### New Capabilities

- `federation-ref-fetch`: Fetching git objects for synced federation refs and updating local repo state

### Modified Capabilities

- `federation-sync-cli`: The sync command gains `--fetch` flag for combined discovery + object pull

## Impact

- `crates/aspen-cli/` — new `federation fetch` subcommand, `--fetch` flag on `sync`
- `crates/aspen-forge-handler/` — handler for the new fetch RPC
- `crates/aspen-forge/src/resolver.rs` — `sync_objects` impl currently returns empty; needs to return actual git objects
- `crates/aspen-client-api/` — new RPC message types for fetch request/response
- `crates/aspen-federation/src/sync/client.rs` — `sync_remote_objects` already exists but unused end-to-end
- NixOS VM test to validate cross-cluster ref fetch
