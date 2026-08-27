## Phase 1: Profile and compatibility core

- [x] [depends:introduce-world-commit-core] Record baseline logical task restore, scheduler, time, entropy, effect-log, VM evidence, and ChaosControl snapshot checks. r[molten.world_snapshot.verification]
- [x] [serial] Define execution-profile, cohort, completeness inventory, compatibility, ownership, synchronization, restore, clone, and diagnostic DTOs. r[molten.world_snapshot.profiles] r[molten.world_snapshot.cohort]
- [x] [depends:world-snapshot-dtos] Implement pure logical and opaque profile validation, completeness checks, cohort comparison, and default denial for unknown profiles. r[molten.world_snapshot.logical] r[molten.world_snapshot.opaque] r[molten.world_snapshot.cohort]
- [x] [parallel] Add canonical Preserves descriptors, inventories, compatibility reports, restore plans, clone plans, and bounded receipts. r[molten.world_snapshot.profiles]

## Phase 2: Restore and clone adapters

- [x] [depends:world-snapshot-core] Add narrow logical-root, ChaosControl descriptor, host-handle, current-authority, restore, and observation ports. r[molten.world_snapshot.restore]
- [x] [depends:world-snapshot-ports] Implement logical restore ordering and current runtime admission over Molten-owned roots. r[molten.world_snapshot.logical] r[molten.world_snapshot.restore]
- [x] [depends:chaoscontrol-exact-snapshot-cohort] Add an exact ChaosControl snapshot descriptor and restore adapter with no compatibility fallback. r[molten.world_snapshot.opaque] r[molten.world_snapshot.restore]
- [x] [depends:build-vm-cohort] [depends:vm-cohort-chaoscontrol-pilot] Pin a reviewed VM Cohort revision and add parent-bound isolated copy-on-write clone planning and realization. r[molten.world_snapshot.cow]
- [x] [depends:world-snapshot-restore-adapters] Add operator snapshot-inspect, compatibility, restore-plan, restore, clone-plan, and clone commands with safe receipts. r[molten.world_snapshot.restore]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive complete logical restore, exact opaque restore, repeated compatibility, isolated child overlay, and current-handle recreation fixtures. r[molten.world_snapshot.verification]
- [x] [parallel] Add negative missing component, duplicate device, CPU inventory mismatch, topology mismatch, stale runtime, unsupported schema, live-handle capture, stale authority, opaque merge, mixed ownership conflict, overlay collision, partial clone, cross-architecture fallback, and snapshot-as-correctness fixtures. r[molten.world_snapshot.verification]
- [x] [serial] Document profile classes, cohort fields, completeness rules, handle and authority recreation, VM Cohort boundary, and portability non-claims. r[molten.world_snapshot.profiles] r[molten.world_snapshot.restore]
- [x] [depends:world-snapshot-verification] Run focused tests, ChaosControl and VM Cohort compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_snapshot.verification]
