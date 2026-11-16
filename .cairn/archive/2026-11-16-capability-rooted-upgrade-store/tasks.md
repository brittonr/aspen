## Phase 1: Implementation

- [x] [serial] Add the upgrade store kind and typed root, atomic `publish`/`publish_new` on `StorageRoot`, and stage-free listings. r[molten.upgrades.capability_rooted_store]
- [x] [serial] Move every upgrades read, scan and write onto the typed root, publish content-addressed members without replacement, replace pointers and status atomically, store pointer receipts first, and remove the ambient `fs` shim; open the root once in the CLI. r[molten.upgrades.capability_rooted_store]
- [x] [serial] Test immutable conflicts, whole pointer replacement, invisible stages and refused links; document the guarantee; run the upgrades, node-host and CLI upgrade suites, formatting, Clippy and source/lifecycle gates before review, sync and archive. r[molten.upgrades.capability_rooted_store]
