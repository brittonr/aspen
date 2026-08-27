## Phase 1: Profiles and metric core

- [x] [depends:add-world-commit-replication-and-retention] Record baseline content-store, Redb, snapshot, diff, replication, retention, and simulation tests. r[molten.world_bench.verification]
- [x] [serial] Define benchmark dataset, preparation, operation, metric, cohort, threshold, receipt, comparison, and extraction-decision DTOs. r[molten.world_bench.profile] r[molten.world_bench.receipt]
- [x] [depends:world-bench-dtos] Implement pure profile validation, metric completeness, comparison, threshold evaluation, freshness, and domain-separated BLAKE3 identities. r[molten.world_bench.profile] r[molten.world_bench.metrics]
- [x] [depends:world-bench-dtos] Implement the pure retain-current, optimize-in-place, or evaluate-shared-component decision over accepted receipts and policy. r[molten.world_bench.extraction_decision]
- [x] [parallel] Add typed Nickel profiles and checked projections with named limits and no unexplained numeric thresholds. r[molten.world_bench.profile]

## Phase 2: Instrumented adapters

- [x] [depends:world-bench-core] Add narrow dataset, preparation, operation, resource-observation, snapshot-observation, and receipt ports. r[molten.world_bench.metrics]
- [x] [depends:world-bench-ports] Instrument root-only branch creation, first mutation, repeated mutation, diff, merge planning, and capsule export. r[molten.world_bench.metrics]
- [x] [depends:world-bench-ports] Instrument content reuse, replicated bytes, pin evaluation, reachability traversal, candidate classification, and GC-plan size without executing deletion. r[molten.world_bench.retention]
- [x] [depends:portable-chaoscontrol-snapshot-descriptor] Add exact ChaosControl snapshot sharing metrics and preserve opaque-profile non-equivalence. r[molten.world_bench.snapshot_profiles]
- [x] [parallel] Add synthetic and downstream-shaped datasets with identified cold and declared warm preparation states. r[molten.world_bench.datasets]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive stable receipt, root-only branch, unchanged-object reuse, changed-byte accounting, replication reuse, safe retention plan, and repeated-run fixtures. r[molten.world_bench.verification]
- [x] [parallel] Add negative profile mixing, stale source, missing metric, unknown preparation, hidden prepopulation, unexplained threshold, unbound hardware, unsafe deletion candidate, timing-as-correctness, asymptotic-overclaim, and automatic-extraction fixtures. r[molten.world_bench.verification]
- [x] [serial] Document metric meaning, preparation, logical versus physical bytes, profile boundaries, retention non-authority, finite-run limits, and extraction gates. r[molten.world_bench.receipt]
- [x] [depends:world-bench-verification] Run focused tests, bounded benchmark cohorts, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_bench.verification]
