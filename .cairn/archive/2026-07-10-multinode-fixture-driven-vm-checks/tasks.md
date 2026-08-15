# Tasks: multinode-fixture-driven-vm-checks

## Phase 1: Fixture contract and export

- [x] [parallel] r[molten.testing.multinode.fixture_driven_vm_plan] Extend the Nickel fixture contract or exports to cover VM shard id, command surface, expected artifact kinds, required receipts, variance, unavailable policy, diagnostic logs, and caveats.
- [x] [parallel] r[molten.testing.multinode.fixture_export_validation_gate] Add pure validation for fixture export schema, source language, required refs, and evidence-only caveats.

## Phase 2: VM plan wiring

- [x] [serial] r[molten.testing.multinode.fixture_driven_vm_plan] Wire VM shard planning or validation to consume checked fixture metadata instead of duplicating scenario shape in handwritten Nix logic.
- [x] [serial] r[molten.testing.multinode.fixture_export_validation_gate] Deny VM pass evidence when observed topology, command surface, artifact kinds, child refs, unavailable policy, or caveats diverge from the fixture.

## Phase 3: Fixtures, docs, validation

- [x] [parallel] r[molten.testing.multinode.fixture_driven_vm_plan] Add valid fixtures for smoke, live-control, service/job, restart, fault, three-node quorum, and aggregate scenarios.
- [x] [parallel] r[molten.testing.multinode.fixture_export_validation_gate] Add negative fixtures for wrong topology, wrong command surface, missing artifact kind, missing variance, unsupported pass claim, and log-only success.
- [x] [serial] r[molten.testing.multinode.fixture_driven_vm_plan] Document fixture authoring and run focused fixture/VM validation tests.
