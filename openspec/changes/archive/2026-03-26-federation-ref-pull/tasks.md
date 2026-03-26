## 1. ForgeResourceResolver returns ref SyncObjects

- [x] 1.1 Define `RefEntry` struct (ref_name, head_hash) with postcard serialization in `aspen-federation/src/sync/types.rs`
- [x] 1.2 Implement `ForgeResourceResolver::sync_objects` to scan `forge:refs:{repo_id}:*`, build `SyncObject` entries with `object_type: "ref"`, BLAKE3 hash, and postcard-encoded data
- [x] 1.3 Filter out objects whose BLAKE3 hash appears in `have_hashes`
- [x] 1.4 Add unit tests for the resolver: returns refs, filters by have_hashes, respects limit, handles empty repo

## 2. FederationFetchRefs RPC plumbing

- [x] 2.1 Add `FederationFetchRefs { peer_node_id, peer_addr, fed_id }` to `ClientRpcRequest` in `aspen-client-api`
- [x] 2.2 Add `FederationFetchRefsResult { is_success, fetched, already_present, errors, error }` to `ClientRpcResponse`
- [x] 2.3 Wire the new request through the RPC handler dispatch in `aspen-forge-handler` (route to a new `handle_federation_fetch_refs` function)

## 3. Fetch handler implementation

- [x] 3.1 Implement `handle_federation_fetch_refs` in `aspen-forge-handler/src/handler/handlers/federation.rs`: connect to remote peer, call `sync_remote_objects` with `want_types: ["refs"]`, collect `have_hashes` from existing `_fed:mirror:*` keys
- [x] 3.2 Parse received `SyncObject` data as `RefEntry`, write each to `_fed:mirror:{origin_short}:{local_id_short}:refs/{name}` in local KV
- [x] 3.3 Return fetch stats (fetched count, already_present count, errors)

## 4. CLI commands

- [x] 4.1 Add `federation fetch` subcommand with `--peer`, `--addr`, `--fed-id` args in `aspen-cli`
- [x] 4.2 Add `--fetch` flag to `federation sync` that calls fetch for each discovered resource after sync completes
- [x] 4.3 Display per-resource fetch results in human and JSON output formats

## 5. NixOS VM test

- [x] 5.1 Create `nix/tests/federation-ref-fetch-test.nix`: two-node test where Alice creates a repo, federates it, pushes a commit; Bob syncs then fetches, verifying `_fed:mirror:*` keys appear in KV
