## Phase 1: Inventory and pure conformance core

- [ ] [depends:add-world-commit-replay-capsules] Record baseline capture, head, effect-release, reconciliation, replication, import, retention, and simulation tests. r[molten.world_faults.verification]
- [ ] [serial] Build a closed mutation-boundary inventory with owner, operation domain, pre-state, effects, linearization point, durable record, uncertain window, reconciliation entry, and required cases. r[molten.world_faults.inventory]
- [ ] [depends:world-mutation-inventory] Define typed fault phase, schedule, operation observation, durable read-back, expected decision, conformance result, and receipt DTOs. r[molten.world_faults.profile] r[molten.world_faults.receipt]
- [ ] [depends:world-fault-dtos] Implement pure inventory validation, schedule validation, observation comparison, unsupported-row handling, and domain-separated BLAKE3 profile identity. r[molten.world_faults.inventory] r[molten.world_faults.profile]
- [ ] [parallel] Add typed Nickel fault profiles with named limits and checked Rust projections. r[molten.world_faults.profile]

## Phase 2: Deterministic and durable harnesses

- [ ] [depends:world-fault-core] Add narrow fault-control, restart, durable-observation, concurrent-schedule, and receipt ports. r[molten.world_faults.shell_boundary]
- [ ] [depends:world-fault-ports] Implement deterministic in-memory phase interruption for every inventory row. r[molten.world_faults.interruption]
- [ ] [depends:world-fault-ports] Implement explicit concurrent schedules for head, promotion, witness, outbox, import, replication, retention, and GC operations. r[molten.world_faults.concurrency]
- [ ] [depends:world-fault-interruption] Add process-restart read-back tests for local durable adapters and Transactional Reconciliation Core decisions. r[molten.world_faults.recovery]
- [ ] [parallel] Add bounded conformance receipts that retain unsupported rows and physical-failure non-claims. r[molten.world_faults.receipt]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive already-complete, safe-retry, superseded, conflict-preserving, and manual-review recovery fixtures. r[molten.world_faults.verification]
- [ ] [parallel] Add negative torn record, lost response, duplicate submission, stale plan, missing object, corrupt record, generation race, effect uncertainty, rollback without witness, unsafe cleanup, contradictory observation, and fault-coverage-overclaim fixtures. r[molten.world_faults.verification]
- [ ] [serial] Document the inventory, semantic fault phases, schedule rules, recovery classes, physical-failure limits, and receipt interpretation. r[molten.world_faults.receipt]
- [ ] [depends:world-fault-verification] Run focused tests, deterministic matrix, restart harness, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_faults.verification]
