## Phase 1: Provider contract and pure core

- [x] [serial] Record the governed release-channel ownership decision and pin the exact provider contract, source revision, license, and non-claims. r[molten.world_witness.provider_boundary]
      Dedicated `release-channel` owner selected by the user on 2026-10-29 (root workspace governance commit
      `87e08c9`, local, unpublished). Pinned `OnixResearch/release-channel` `fa389e1`, contract
      `release-channel-index-v1`, MIT in `Cargo.toml`/`Cargo.lock`/`flake.nix`/`crate-hashes.json`,
      `world_witness::provider::contract`, `THIRD_PARTY_LICENSES.md` and `docs/world-branch-heads.md`.
      See `evidence/2026-10-30-provider-decision.md`.
- [x] [depends:world-head-protocol] Record baseline head-claim, generation, authority, reconciliation, and artifact-auth tests. r[molten.world_witness.verification]
      Pre-change baseline `cargo test -p molten-core --lib world_` (world-head and world-witness cores): 147 passed.
      See `evidence/2026-10-30-verification.md`.
- [x] [serial] Define local and independent witness profiles plus normalized append, inclusion, consistency, checkpoint, quorum, fork, unavailable, currentness, and diagnostic DTOs. r[molten.world_witness.profiles] r[molten.world_witness.currentness]
      `molten_core::world_witness` `AssuranceProfile`, `WitnessPolicy`, `AppendObservation`, `InclusionObservation`,
      `ConsistencyObservation`, `SignedCheckpoint`, `QuorumObservation`, `CurrentHeadObservation`,
      `WitnessObservation::{Available, Unavailable, Fork}`, `WitnessIssue` (`domain.rs`).
- [x] [depends:world-witness-dtos] Implement pure provider, branch, claim, checkpoint-chain, quorum, freshness, and highest-admitted-state validation. r[molten.world_witness.currentness] r[molten.world_witness.provider_boundary]
      `validate_currentness`, `validate_committed_currentness` and bounds (`validation.rs`, `bounds.rs`); tests
      `tests/{currentness,retained,bounds}.rs`.
- [x] [depends:world-witness-dtos] Implement pure stage, witness-request, finalization, and uncertain reconciliation planning. r[molten.world_witness.finalization] r[molten.world_witness.reconciliation]
      `plan_stage`, `plan_finalization` (`planning.rs`) and `reconcile` over Transactional Reconciliation Core
      (`reconciliation.rs`); tests `tests/{planning,recovery}.rs`.

## Phase 2: Shell and persistence

- [x] [depends:world-witness-core] Add narrow witness-provider, staged-claim, durable-currentness, local-head-transaction, clock-observation, and reconciliation ports. r[molten.world_witness.provider_boundary] r[molten.world_witness.finalization]
      `provider::{Transport, Ports}`, `WallClock`, `FreshAdmission`, `WitnessRecheck`, `store::Database`
      (stage, reservation, highest, receipt and reconciliation tables) and `recovery::Request`.
- [x] [depends:world-witness-ports] Implement durable claim staging and provider append without granting the provider branch authority. r[molten.world_witness.finalization]
      `world_witness::advance` stages via `Database::stage` before `lifecycle::reserve::acquire`; the external
      ordinal comes from `provider::ordinal::plan` over a fresh signed query. Authority is rechecked by
      `FreshAdmission` only.
- [x] [depends:world-witness-ports] Implement final local compare-and-swap with in-transaction head, generation, policy, authority, staged-claim, and highest-witness rechecks. r[molten.world_witness.finalization] r[molten.world_witness.currentness]
      `Database::finalize` (`store/finalize.rs`) rereads head, durable stage and retained highest inside one Redb
      write transaction and calls `plan_finalization` with the fresh `effects::Recheck` observation.
- [x] [depends:world-witness-finalization] Implement uncertain append and uncertain local-commit reconciliation through Transactional Reconciliation Core. r[molten.world_witness.reconciliation]
      `recovery::recover`, `recovery::binding`, `reconcile_and_resume`; lost reserve responses recover the exact
      provider-signed reservation (`Index::recover_reservation`) and never dispatch a new Reserve.
- [x] [parallel] Add detached receipts that keep authentication, authorization, witness currentness, local persistence, and release claims separate. r[molten.world_witness.receipts]
      `records::receipt::validate` rejects collapsed roles, witness-as-authority, local overclaims and dropped
      non-claims; tests `records/tests.rs`.

## Phase 3: Verification and documentation

- [x] [parallel] Add positive single-provider, admitted quorum, repeated read, orphan witnessed claim, already-complete, and bounded recovery fixtures. r[molten.world_witness.verification]
      Core: `admitted_checkpoint_advances_and_repeated_current_read_is_bounded`,
      `admitted_quorum_can_grow_…`, `exact_successful_append_recovers_to_finalization_and_complete_publication`.
      Shell: `lost_append_retains_exact_operation_…`, new
      `provider::tests::supersession::{resolved_orphan_high_water_is_the_only_admitted_external_gap,
      lost_reserve_across_gap_recovers_the_signed_external_position}`. Executed pilot: single provider, repeated
      reads, resolved-orphan supersession. See `evidence/2026-10-30-verification.md`.
- [x] [parallel] Add negative whole-store rollback, stale checkpoint, invalid inclusion, failed consistency, provider fork, split quorum, provider substitution, wrong branch, missing highest state, unavailable provider, uncertain append, uncertain local commit, witness-as-authority, and local-profile-overclaim fixtures. r[molten.world_witness.verification]
      Mapping per case in `evidence/2026-10-30-verification.md`; executed pilot negatives: unreviewed provider pin,
      unresolved orphan (advance and read), unavailable provider, complete consumer-root rollback.
- [x] [serial] Document assurance profiles, provider ownership, consumer durable state, staging, fork handling, availability trade-offs, and non-claims. r[molten.world_witness.receipts]
      `docs/world-branch-heads.md` (dependency cohort, orphans and external ordinals, pilot) and
      `docs/world-operator-workflows.md` (external ordinal recovery).
- [x] [depends:world-witness-verification] Run focused tests, provider conformance fixtures, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_witness.verification]
      Commands and results in `evidence/2026-10-30-verification.md`.
