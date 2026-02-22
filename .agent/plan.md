# Plan

- [x] `aspen-service-registry-handler` → deleted, `aspen-service-registry-plugin` is canonical
- [x] `aspen-automerge-handler` → deleted, `aspen-automerge-plugin` is canonical
- [x] `aspen-secrets-handler` → slimmed to PKI + NixCache only (native crypto), KV + Transit migrated to `aspen-secrets-plugin`
- [x] `aspen-coordination-handler` → deleted, `aspen-coordination-plugin` created (46 request types, 8 domains: locks, counters, sequences, rate limiters, barriers, semaphores, RW locks, queues)
- [ ] `aspen-nix-handler` → new `aspen-nix-plugin`. Already uses only `blob_store` + `kv_store` + `node_id`, all in host ABI today.
- [ ] Add `sql_query` host function → migrate `aspen-query-handler` to plugin
- [ ] Add `trigger_hook` host function → migrate `aspen-hooks-handler` to plugin
- [ ] Add extended blob host functions (`blob_list`, `blob_delete`, `blob_protect`, `blob_unprotect`, `blob_replication_status`) → migrate `aspen-blob-handler` to plugin

Progress: 4/8 steps completed
