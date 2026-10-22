# Tasks: Repair partial replicas without a complete donor

All tasks are proposed work. This package grants no implementation permission.

- [x] [serial] Record the current repair planner inputs, per-replica verification facts, completeness handling, and the existing replication test baseline. r[molten.closure_repair.donor_eligibility]
- [x] [serial] Change donor eligibility to per-object granularity over admitted verification facts, with bounded inventory admission. r[molten.closure_repair.donor_eligibility]
- [x] [serial] Implement per-object verification on assembly with identity-mismatch rejection that isolates failures to the failing object. r[molten.closure_repair.per_object]
- [x] [serial] Add the typed incomplete-repair outcome that names unrecoverable identities and preserves progress on repaired objects. r[molten.closure_repair.incomplete_outcome]
- [x] [parallel] Add the multi-partial-donor closure scenario to normal repository tests with named identities and bounded inventories. r[molten.closure_repair.per_object]
- [x] [parallel] Add negative fixtures: zero-intact-copy objects, assembly identity mismatch, donor flapping between plan and transfer, unbounded inventory, and malformed verification records. r[molten.closure_repair.validation]
- [x] [serial] Update replication docs and operator status with the completeness field and the bytes-not-authority non-claim. r[molten.closure_repair.boundary]
      The shell now runs the closure repair planner over the observed inventory, the operator status carries the
      repair completeness, the unrecoverable identities, and the repair non-claims, and the canonical operator status
      record refuses a view without the bytes-not-authority claim, with an unknown completeness, or with an
      incomplete repair that names nothing. The record moves to `content-replication-operator-status-v2`.
- [x] [serial] Keep the F04 stale-success regression passing unchanged, then run focused Octet, Clippy, workspace, Nix, and required Cairn gates. r[molten.closure_repair.validation]
