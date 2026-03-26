## 1. Wire handle_federate_repository

- [x] 1.1 Pass `federation_cluster_identity` to `handle_federate_repository` from the executor
- [x] 1.2 Implement: parse repo_id hex → RepoId, verify repo exists via forge_node, construct FederatedId from cluster pubkey + repo hash, build FederationSettings, call forge_node.set_federation_settings, return fed_id

## 2. Extend federation sync to fetch ref state

- [x] 2.1 After `list_remote_resources`, call `get_remote_resource_state` for each resource and collect ref heads
- [x] 2.2 Add `ref_heads: Vec<(String, String)>` to `SyncPeerResourceInfo` (ref name, hex hash)
- [x] 2.3 Populate `ref_count` and `ref_names` from the actual state query (currently hardcoded to 0/empty)

## 3. Store synced refs in local KV

- [x] 3.1 After collecting ref heads, write each to local KV under `_fed:sync:<origin_short>:<local_id_short>:refs/<name>` with hex hash as value
- [x] 3.2 Pass `kv_store` through to the sync handler (executor already has access via forge_node)

## 4. Update NixOS federation test

- [x] 4.1 Alice creates repo, pushes a commit to create refs/heads/main
- [x] 4.2 Alice federates the repo via `federation federate <repo_id>`
- [x] 4.3 Alice verifies `federation list-federated` shows the repo
- [x] 4.4 Bob syncs from Alice with `federation sync --peer --addr`
- [x] 4.5 Bob asserts sync result has resources with ref_count > 0
- [x] 4.6 Bob verifies synced refs in local KV via `kv scan _fed:sync:`

## 5. Verify

- [x] 5.1 cargo clippy and rustfmt
- [x] 5.2 cargo nextest run -E 'test(/federation/)' --features full
