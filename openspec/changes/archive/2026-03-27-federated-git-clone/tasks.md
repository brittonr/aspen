## 1. URL Parsing & Routing

- [x] 1.1 Add `FederatedTarget` variant to `AspenUrl` in `src/bin/git-remote-aspen/url.rs` — parse `fed:<origin-base32>:<repo-hex>` from the repo_id portion, extract origin public key + upstream repo ID
- [x] 1.2 Unit tests for federated URL parsing: valid, malformed, non-federated passthrough
- [x] 1.3 Route federated URLs in `RemoteHelper::handle_list` and `handle_fetch_batch` — send `FederationGitListRefs`/`FederationGitFetch` instead of direct Forge RPCs

## 2. Client API — New RPC Variants

- [x] 2.1 Add `FederationGitListRefs { origin_key: String, repo_id: String, origin_addr_hint: Option<String> }` and `FederationGitFetch { origin_key: String, repo_id: String, want: Vec<String>, have: Vec<String>, origin_addr_hint: Option<String> }` to `ClientRpcRequest` in `crates/aspen-client-api/`
- [x] 2.2 Add response variants `FederationGitListRefs(GitBridgeListRefsResponse)` and `FederationGitFetch(GitBridgeFetchResponse)` to `ClientRpcResponse`
- [x] 2.3 Serialization round-trip test for new variants

## 3. Federation Git Handler

- [x] 3.1 Create handler for `FederationGitListRefs` — derive mirror repo ID (`blake3(origin_key || repo_id)`), check if mirror exists, create if not, do federation sync if stale (>30s), return refs from mirror
- [x] 3.2 Create handler for `FederationGitFetch` — serve objects from mirror repo, trigger federation sync if mirror lacks requested objects
- [x] 3.3 Wire handler into RPC dispatch (registry or handler chain)
- [x] 3.4 Integration test: deterministic mirror ID derivation (same inputs → same mirror, different inputs → different mirror)

## 4. Federation Sync Glue

- [x] 4.1 Add helper function to perform a federation sync pull for a specific repo — connect to origin via iroh, call `sync_objects` for refs + git objects, import into mirror repo via `GitImporter`
- [x] 4.2 Staleness check: read `MirrorConfig.last_sync_ms`, skip sync if within 30s
- [x] 4.3 Update `MirrorConfig.last_sync_ms` after successful sync

## 5. NixOS VM Integration Test

- [x] 5.1 Create `nix/tests/federation-git-clone.nix` — two-cluster test: Alice creates repo + pushes commit + federates; Bob runs `git clone aspen://<bob-ticket>/fed:<alice-key>:<repo-id>` and verifies file contents
- [x] 5.2 Wire test into flake checks

## 6. Polish

- [x] 6.1 Add `--origin-addr` option to git-remote-aspen for passing address hints in federated URLs (avoids discovery latency)
- [x] 6.2 stderr progress messages during federation sync ("syncing from origin cluster...", "imported N objects")
