# Redb durable-log restore tasks

- [x] [serial] Read accepted durable-state and native-host contracts, current canonical profile validation, append budgets, Redb bounded table metadata, and journal callers before changing restore. r[molten.fabric_durability.redb_restore_profile_admission]
- [x] [serial] Capture failing-before real persisted Redb count and byte limit fixtures against the clean isolated owner. r[molten.fabric_durability.redb_restore_profile_admission]
- [x] [serial] Thread the validated current profile through Redb log restore; preflight count and borrowed per-record byte budgets before owning persisted values. r[molten.fabric_durability.redb_restore_profile_admission]
- [x] [serial] Land the same narrow fix and tests in the disjoint cluster composition; smoke the public API and native-host journal restart. r[molten.fabric_durability.redb_restore_profile_admission]
- [x] [serial] Update handoff and record focused owner/composition fmt, Clippy, scoped Octet and Cairn proof; do not invoke full strict Octet. r[molten.fabric_durability.redb_restore_profile_admission]
