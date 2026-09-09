# Tasks: Repair partial replicas without a complete donor

All tasks are proposed work. This package grants no implementation permission.

- [ ] [serial] Record the current repair planner inputs, per-replica verification facts, completeness handling, and the existing replication test baseline. r[molten.closure_repair.donor_eligibility]
- [ ] [serial] Change donor eligibility to per-object granularity over admitted verification facts, with bounded inventory admission. r[molten.closure_repair.donor_eligibility]
- [ ] [serial] Implement per-object verification on assembly with identity-mismatch rejection that isolates failures to the failing object. r[molten.closure_repair.per_object]
- [ ] [serial] Add the typed incomplete-repair outcome that names unrecoverable identities and preserves progress on repaired objects. r[molten.closure_repair.incomplete_outcome]
- [ ] [parallel] Add the multi-partial-donor closure scenario to normal repository tests with named identities and bounded inventories. r[molten.closure_repair.per_object]
- [ ] [parallel] Add negative fixtures: zero-intact-copy objects, assembly identity mismatch, donor flapping between plan and transfer, unbounded inventory, and malformed verification records. r[molten.closure_repair.validation]
- [ ] [serial] Update replication docs and operator status with the completeness field and the bytes-not-authority non-claim. r[molten.closure_repair.boundary]
- [ ] [serial] Keep the F04 stale-success regression passing unchanged, then run focused Octet, Clippy, workspace, Nix, and required Cairn gates. r[molten.closure_repair.validation]
