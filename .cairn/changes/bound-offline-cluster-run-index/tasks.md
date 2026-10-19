# Bound offline cluster run-index intake

## Implemented and observed

- [x] [serial] Identify F628/F629, the real `cluster harness-verify` local CLI reader, and the existing 4,096-artifact core assessment limit. r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound]
- [x] [serial] Reproduce over-count and over-byte parser admission before the fix; test exact and maximum-plus-one boundaries afterward. r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound]
- [x] [serial] Guard the file read and parser append without changing normal artifact or verification-companion checks. r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound]
- [x] [serial] Exercise the real positive two-node harness/verify CLI and negative over-count and over-byte verify CLI; retain the source-only owner until disjoint composition evidence. r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound]
- [x] [serial] Record scoped Clippy, rustfmt, warning-only Octet, and Cairn structural/advisory evidence without claiming full strict-ci or lifecycle acceptance. r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound]
