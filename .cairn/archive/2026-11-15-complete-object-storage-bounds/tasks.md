# Tasks

- [x] [serial] Implement local capacity/reopen and shell local-hit fixes; author consumer-visible boundary tests (execution remains below). r[molten.content_store_adapter.verify_before_available]
- [x] [serial] Freeze endpoint/credential policy and enforce remote request bounds; preserve fault taxonomy and author parity receipt producers. r[molten.content_store_adapter.streaming_bounds]
- [x] [serial] Gate deletion with canonical retention and author remote-presence-loss tests. r[molten.content_store_adapter.retention_boundary]
- [x] [serial] Add pinned endpoint, direct-dependency guard and actual-device qualification inputs, with exact parent commands. r[molten.content_store_adapter.streaming_bounds] r[molten.content_store_adapter.retention_boundary]
- [x] [serial] Parent exact-pins the existing optional reqwest 0.13.4 / sha2 0.10.9 / hmac 0.12.1 / time 0.3.47 cohort and matching observed registry checksums; no new client dependency or version update. Root locks/build plans and source-policy integration remain parent-owned. r[molten.content_store_adapter.streaming_bounds]
- [x] [serial] Retain baseline/current lock counts or inverse dependency roots and run the direct-dependency guard after parent regeneration. r[molten.content_store_adapter.streaming_bounds]
- [x] [serial] Parent runs focused core/object/simulation tests and retains canonical receipts; do not mark unexecuted evidence complete. r[molten.content_store_adapter.verify_before_available] r[molten.content_store_adapter.streaming_bounds] r[molten.content_store_adapter.retention_boundary]

## Parent execution (2026-11-13)

The optional object-feature controls passed 32 tests (six separately selected
rails ignored). After the bounded HEAD repair, the actual local S3 command and
cache roundtrip controls each passed. The actual NVMe-path capacity/reopen
control also passed using `/home/brittonr/.cache/aspen-nvme-qualification-20261113`
and `/sys/class/block/nvme1n1p3`; the task-owned tmpfs was not its storage claim.
The direct-dependency guard passed. Canonical object-conformance and NVMe-path
receipts were retained by the parent. These observations do not qualify cloud
providers, physical power loss, machine loss or measured recovery time.

