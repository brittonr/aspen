## 1. Wire GitExporter into federation resolver

- [x] 1.1 Auto-create GitExporter in `ForgeResourceResolver::new()` using InMemoryBlobStore + KV-backed reads (no changes to `src/node/mod.rs` needed)
- [x] 1.2 `new()` now always creates an exporter when git-bridge feature is enabled — no separate wiring step
- [x] 1.3 Verified compilation with `--features full`, all 17 resolver/exporter tests pass

## 2. Content hash fix

- [x] 2.1 Fix SyncObject hash: use blake3(content) not envelope hash (client was rejecting all objects)

## 3. Mirror ref hash alignment

- [x] 3.1 After federation_import_objects, map Alice's ref hashes to Bob's imported commit hashes and update mirror refs accordingly

## 4. Verify

- [x] 4.1 Run the `federation-git-clone` NixOS VM test — assert `README.md` content matches ✅
