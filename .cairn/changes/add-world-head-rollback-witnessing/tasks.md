## Phase 1: Provider contract and pure core

- [ ] [serial] [blocked:workspace-witness-owner] Record the governed release-channel ownership decision and pin the exact provider contract, source revision, license, and non-claims. r[molten.world_witness.provider_boundary]
- [ ] [depends:world-head-protocol] Record baseline head-claim, generation, authority, reconciliation, and artifact-auth tests. r[molten.world_witness.verification]
- [ ] [serial] Define local and independent witness profiles plus normalized append, inclusion, consistency, checkpoint, quorum, fork, unavailable, currentness, and diagnostic DTOs. r[molten.world_witness.profiles] r[molten.world_witness.currentness]
- [ ] [depends:world-witness-dtos] Implement pure provider, branch, claim, checkpoint-chain, quorum, freshness, and highest-admitted-state validation. r[molten.world_witness.currentness] r[molten.world_witness.provider_boundary]
- [ ] [depends:world-witness-dtos] Implement pure stage, witness-request, finalization, and uncertain reconciliation planning. r[molten.world_witness.finalization] r[molten.world_witness.reconciliation]

## Phase 2: Shell and persistence

- [ ] [depends:world-witness-core] Add narrow witness-provider, staged-claim, durable-currentness, local-head-transaction, clock-observation, and reconciliation ports. r[molten.world_witness.provider_boundary] r[molten.world_witness.finalization]
- [ ] [depends:world-witness-ports] Implement durable claim staging and provider append without granting the provider branch authority. r[molten.world_witness.finalization]
- [ ] [depends:world-witness-ports] Implement final local compare-and-swap with in-transaction head, generation, policy, authority, staged-claim, and highest-witness rechecks. r[molten.world_witness.finalization] r[molten.world_witness.currentness]
- [ ] [depends:world-witness-finalization] Implement uncertain append and uncertain local-commit reconciliation through Transactional Reconciliation Core. r[molten.world_witness.reconciliation]
- [ ] [parallel] Add detached receipts that keep authentication, authorization, witness currentness, local persistence, and release claims separate. r[molten.world_witness.receipts]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive single-provider, admitted quorum, repeated read, orphan witnessed claim, already-complete, and bounded recovery fixtures. r[molten.world_witness.verification]
- [ ] [parallel] Add negative whole-store rollback, stale checkpoint, invalid inclusion, failed consistency, provider fork, split quorum, provider substitution, wrong branch, missing highest state, unavailable provider, uncertain append, uncertain local commit, witness-as-authority, and local-profile-overclaim fixtures. r[molten.world_witness.verification]
- [ ] [serial] Document assurance profiles, provider ownership, consumer durable state, staging, fork handling, availability trade-offs, and non-claims. r[molten.world_witness.receipts]
- [ ] [depends:world-witness-verification] Run focused tests, provider conformance fixtures, Octet, Clippy with warnings denied, Cairn validation and gates, lifecycle checks, and relevant Nix checks. r[molten.world_witness.verification]
