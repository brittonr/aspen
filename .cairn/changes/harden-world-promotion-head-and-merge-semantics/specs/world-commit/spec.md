# World commit deltas

## ADDED Requirements

### Requirement: World promotion authority is action-bound, canonical-Preserves-signed, and head-current
r[molten.world_commit.promotion_authority_action_and_canonical_binding] `SignedPromotionAuthority` MUST bind explicit action/purpose, exact target (`candidate_head` or `reservation_ref`), nonce/expiry, and generation over canonical packed Preserves bytes (never delimiter-joined text). Operator mutations (`deny`, `retry-plan`, `abandon`) MUST validate authority against the live branch head.

#### Scenario: Deny grant cannot authorize candidate promotion
- GIVEN a signed authority grant issued at generation `G` for `DenyReservation`
- WHEN a caller presents that grant to promote a candidate from generation `G` to `G + 1`
- THEN promotion admission MUST deny due to action and target mismatch.

#### Scenario: Newline or comma injection cannot forge signing bytes
- GIVEN branch or intent identifiers containing `\n` or `,`
- WHEN signing or verifying promotion authority or closure messages
- THEN canonical packed Preserves encoding MUST prevent distinct structs from colliding.

### Requirement: World promotion dispatch and read-back preserve single-transaction atomicity and recovery
r[molten.world_commit.promotion_atomic_dispatch_and_readback] Reservation and attempt transitions MUST commit atomically in one Redb write transaction, abandonment MUST transition both attempt and reservation, `update_reservation` MUST enforce immutable identity fields, transaction fact validation MUST run inside `begin_write()`, and `read_back_promotion` MUST classify partial reservation persistence as `Corrupt` rather than `Prior`.

#### Scenario: Crash between reservation and attempt updates cannot wedge dispatch
- GIVEN an interrupted dispatch transition
- WHEN recovery inspects the promotion store
- THEN no reservation can be left in `Attempting` without a matching recoverable attempt record.

#### Scenario: Partial promotion persistence reports Corrupt
- GIVEN the branch head equals `plan.after` while fewer than `plan.reservations.len()` reservations exist
- WHEN `read_back_promotion` runs
- THEN it MUST return `Corrupt` rather than `Prior`.

### Requirement: World head transitions emit truthful outcome receipts and verify signer roles from policy
r[molten.world_commit.world_head_truthful_receipts_and_verified_signers] `execute_world_head_transition` MUST emit distinct canonical receipts matching the actual CAS outcome (`applied`, `stale`, or `uncertain`) and MUST derive signer `role` and `authority_admitted` from verified signed statements and authority ports rather than unsigned carrier fields.

#### Scenario: Stale CAS conflict does not return admitted receipt
- GIVEN `store.apply_transition` returns `WorldHeadMutationOutcome::Stale` or `Uncertain`
- WHEN `execute_world_head_transition` returns
- THEN the canonical receipt decision MUST reflect `stale` or `uncertain` and MUST NOT be `"admitted"`.

#### Scenario: Forged unsigned carrier role denies admission
- GIVEN a carrier with tampered `role` or `authority_admitted = true` inconsistent with `AuthenticationPolicy` and `WorldHeadAuthorityPort`
- WHEN world-head signer admission runs
- THEN it MUST ignore the unsigned carrier flags and deny.

### Requirement: World merge enforces profile, runtime-sensitivity, availability, and input_schema before shortcuts, and commit closure rejects detached cycles
r[molten.world_commit.merge_conservative_precedence_and_full_graph_closure] `world_merge::reduce_root` MUST check runtime-sensitivity, material availability, and declared `profile.root_modes` before any equal-root shortcut; application handlers MUST enforce `profile.input_schema`; `diff_world_roots` MUST distinguish creation/deletion from `Absent`; and `worldcommit::validate_parent_graph` MUST reject unreachable nodes and detached cycles.

#### Scenario: Equal profile-excluded or runtime-sensitive roots deny merge
- GIVEN `left.root == right.root` for a root kind excluded from `profile.root_modes` or marked runtime-sensitive (`Tasks`, `Entropy`)
- WHEN `reduce_root` evaluates the root
- THEN it MUST reject the root rather than silently selecting it.

#### Scenario: Application handler rejects mismatched input_schema and closure rejects detached cycle
- GIVEN a root whose schema differs from `profile.input_schema` or a `parent_graph` containing an unreachable `X -> Y -> X` cycle
- WHEN merge planning or closure validation runs
- THEN validation MUST fail closed.
