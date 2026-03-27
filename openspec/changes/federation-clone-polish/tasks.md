## 1. Incremental sync dedup

- [ ] 1.1 In `export_git_objects` (resolver.rs), build a content→envelope hash map from the exported objects, then convert `have_set` content hashes to envelope hashes before passing to `export_dag_blake3` as `known_blake3`
- [ ] 1.2 Fix the incremental unit test to assert exact count (3 new objects, not "at least 3")

## 2. Multi-branch ref translation

- [ ] 2.1 Rewrite `translate_ref_hashes` to match each ref to its commit via SHA1: compute SHA1 from each commit SyncObject's raw content, build SHA1→content_hash map, look up local BLAKE3 via `content_to_local_blake3`
- [ ] 2.2 Unit test: two refs pointing to different commits produce different local BLAKE3 hashes

## 3. Clean up logging

- [ ] 3.1 Downgrade per-object KV read/write info logs to debug in exporter.rs and importer.rs
- [ ] 3.2 Remove or downgrade `export_dag_blake3 returned` and `sync_objects phase 3` diagnostic logs to debug
- [ ] 3.3 Keep summary-level info logs (federation git sync complete, translated ref)
