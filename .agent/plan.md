# Plan

- [x] `aspen-service-registry-handler` → deleted, `aspen-service-registry-plugin` is canonical
- [x] `aspen-automerge-handler` → deleted, `aspen-automerge-plugin` is canonical
- [x] `aspen-secrets-handler` → slimmed to PKI + NixCache only (native crypto), KV + Transit migrated to `aspen-secrets-plugin`
- [x] `aspen-coordination-handler` → deleted, `aspen-coordination-plugin` created (46 request types, 8 domains: locks, counters, sequences, rate limiters, barriers, semaphores, RW locks, queues)
- [ ] `aspen-nix-handler` → BLOCKED: depends on native snix-castore/snix-store (protobuf) and iroh-blobs, not WASM-compatible. Keep native.
- [ ] Add `sql_query` host function → migrate `aspen-query-handler` to plugin (DNS part is pure KV, SQL needs new host fn)
- [ ] Add `trigger_hook` host function → migrate `aspen-hooks-handler` to plugin (needs ctx.hook_service access)
- [ ] Add extended blob host functions (`blob_list`, `blob_delete`, `blob_protect`, `blob_unprotect`, `blob_replication_status`) → migrate `aspen-blob-handler` to plugin (heavy iroh-blobs dependency)

Progress: 4/8 steps completed (remaining 4 require host ABI extensions or are blocked on native deps)
