## Phase 1: Repository and provenance

- [x] [serial] r[vm_cohort.source_inventory] Record exact SmolVM and ChaosControl source artifacts, licenses, contributor ownership, semantic adaptations, and retained consumer authority.
- [x] [serial] r[vm_cohort.extraction_boundary] Classify the reviewed ChaosControl snapshot, block-overlay, dirty-log, and KVM restore source into shared and consumer-owned scopes.
- [x] [serial] r[vm_cohort.chaoscontrol_migration.parity] Run and retain the focused ChaosControl snapshot, block-overlay, divergence, and negative-test baseline before extraction.
- [x] [serial] r[vm_cohort.source_revision] Create the Rust workspace, pinned Nix inputs, typed Nickel profiles, and strict Octet gate.
- [x] [serial] r[vm_cohort.functional_core] Establish the `no_std + alloc` core and imperative-shell dependency boundary.
- [x] [serial] r[vm_cohort.extraction_boundary.leak] Add dependency checks that reject ChaosControl fault, scheduler, assertion, coverage, exploration, and replay types from shared crates.

## Phase 2: Checkpoint and clone model

- [x] [serial] r[vm_cohort.checkpoint_admission] Implement complete golden-checkpoint admission and domain-separated identity.
- [x] [serial] r[vm_cohort.checkpoint_retention] Implement retained checkpoint state and repeated cohort-use transitions.
- [x] [serial] r[vm_cohort.effective_bases] Implement admission that seals initialized memory and disk views into immutable effective bases.
- [x] [serial] r[vm_cohort.clone_isolation] Implement independent clone plans for every mutable memory, disk, device, endpoint, output, and lifecycle surface.
- [x] [serial] r[vm_cohort.effective_bases.shared] Implement shared memory backing and disk-base mechanics with private per-clone overlays in `vm-cohort-kvm`.
- [x] [parallel] r[vm_cohort.conformance.negative] Add incompatible checkpoint, duplicate identity, shared mutable surface, full-copy adapter, base mutation, and consumed-base fixtures.

## Phase 3: Cohort lifecycle

- [x] [serial] r[vm_cohort.resource_admission] Implement checked resource arithmetic, full-cohort reservation, and bounded in-flight admission.
- [x] [serial] r[vm_cohort.transactional_creation] Implement prepared, activated, failed, and rollback-required cohort transitions.
- [x] [serial] r[vm_cohort.retry_policy] Implement explicit transient failure classes, bounded attempts, delay plans, and cancellation checks.
- [x] [serial] r[vm_cohort.cleanup] Implement per-resource cleanup obligations, exact observations, and uncertain terminal outcomes.
- [x] [parallel] r[vm_cohort.transactional_creation.partial_failure] Add failure injection for every creation and activation boundary.
- [x] [parallel] r[vm_cohort.cleanup.uncertain] Add cancellation, failed cleanup, stale observation, and wrong-cohort fixtures.

## Phase 4: Shell and evidence

- [x] [serial] r[vm_cohort.kvm_adapter] Adapt reviewed common vCPU, in-kernel device, dirty-log, memory-mapping, and restore mechanics into the Linux/KVM shell.
- [x] [serial] r[vm_cohort.extraction_boundary.shell] Keep file reads, mappings, KVM descriptors, dirty logs, device state, and page mutation outside `vm-cohort-core`.
- [x] [serial] r[vm_cohort.receipts] Implement bounded BLAKE3 plan, transition, cohort, clone, retry, and cleanup receipts.
- [x] [serial] r[vm_cohort.configuration] Implement typed Nickel profiles and checked generated runtime values.
- [x] [parallel] r[vm_cohort.kvm_adapter.negative] Add KVM incompatibility, resource exhaustion, endpoint collision, and teardown failure tests.

## Phase 5: Consumers and release evidence

- [x] [serial] r[vm_cohort.conformance] Publish shared positive and negative adapter conformance suites.
- [x] [serial] r[vm_cohort.chaoscontrol_migration] Implement a pinned ChaosControl adapter that composes deterministic policy over VM Cohort mechanics.
- [x] [serial] r[vm_cohort.chaoscontrol_migration.parity] Run the baseline and VM Cohort-backed paths over the same bounded migration corpus and compare typed results.
- [x] [serial] r[vm_cohort.chaoscontrol_migration.authority] Verify that ChaosControl retains fault, scheduler, assertion, coverage, exploration, replay, and evidence ownership.
- [x] [serial] r[vm_cohort.chaoscontrol_migration.legacy] Select the shared implementation after parity and retain any legacy path only as named diagnostic rollback code.
- [x] [serial] r[vm_cohort.consumer_pilots] Integrate ChaosControl and one independent fixture or runtime adapter without domain-type leakage.
- [x] [serial] r[vm_cohort.receipts.non_claims] Verify receipts do not claim guest trust, authorization, sandbox correctness, determinism, or release eligibility.
- [x] [serial] r[vm_cohort.source_revision] Run workspace tests, Clippy, Octet, Nickel, Cairn, consumer pilots, migration parity, KVM smoke checks, and `nix flake check -L`.
