# Plan

- [ ] `aspen-service-registry-handler` → delete, `aspen-service-registry-plugin` becomes canonical
- [ ] `aspen-automerge-handler` → delete, `aspen-automerge-plugin` becomes canonical
- [ ] `aspen-secrets-handler` → delete, `aspen-secrets-plugin` becomes canonical (SOPS/PKI stay native in `aspen-secrets` crate — only the RPC handler moves)
- [ ] `aspen-coordination-handler` → new `aspen-coordination-plugin` (2323 lines of pure KV logic — biggest win). This is the most valuable migration: distributed locks, counters, queues, semaphores, barriers, rate limiters, RW locks — all purely KV state machines. Hot-reloadable coordination primitives.
- [ ] `aspen-nix-handler` → new `aspen-nix-plugin`. Already uses only `blob_store` + `kv_store` + `node_id`, all in host ABI today.
- [ ] Add `sql_query` host function → migrate `aspen-query-handler` to plugin
- [ ] Add `trigger_hook` host function → migrate `aspen-hooks-handler` to plugin
- [ ] Add extended blob host functions (`blob_list`, `blob_delete`, `blob_protect`, `blob_unprotect`, `blob_replication_status`) → migrate `aspen-blob-handler` to plugin

Progress: 0/8 steps completed
