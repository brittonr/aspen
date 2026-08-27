## Why

Molten identifies artifacts, schemas, durable values, tasks, histories, effects, scheduler state, time profiles, entropy commitments, runtime cohorts, and policies. It does not bind these facts into one immutable identity for a recoverable computational world.

Separate roots can drift between capture steps. A replay or restore can therefore combine facts that were never current together. Operators also lack one subject for branching, comparison, promotion, replication, retention, and detached evidence.

Molten needs a product-owned world commit. It must compose existing stack mechanisms without creating a stack-global release or authority unit.

## What Changes

- Add canonical `molten-world-commit-v1` core records with ordered parents and typed root references.
- Compute commit identity with domain-separated BLAKE3 framing over canonical Preserves bytes.
- Add a pure capture planner that binds immutable roots and mutable revision fences into one candidate snapshot.
- Publish the commit only after every required root is durable and every captured fence remains current.
- Add pure closure validation, restore planning, replay classification, and first-missing-root diagnostics.
- Keep signatures, attestations, head claims, and live authority outside the hashed commit core.
- Support logical runtime roots and an optional opaque machine-snapshot reference without claiming semantic interchangeability.

## Dependencies

- Existing Molten content refs, typed domain identities, Preserves codecs, replay, scheduler, time, entropy, task, effect, and runtime-profile contracts.
- Choregraph branchable event history for parent-DAG mechanics.
- Schema Identity Core and Schema Migration Core for exact schema references and lineage.
- Artifact Binding Core for complete-root inventory mechanics.

## Non-Goals

- A universal commit format for every OnixResearch repository.
- Generic native-process heap capture or semantic merge of opaque memory.
- Live capability transfer, effect dispatch, branch-head mutation, replication, retention, or garbage collection.
- A compatibility claim for the external `RealmCommit` proposal.

## Impact

- **Core**: world-commit DTOs, typed roots, canonical identity, capture plans, closure validation, restore plans, and diagnostics.
- **Shell**: root observers, durable object publication, revision rechecks, restore adapters, and operator inspection.
- **Schemas**: versioned Preserves world-commit and receipt records.
- **Testing**: positive complete captures plus negative drift, missing-root, wrong-domain, malformed-parent, non-canonical, secret-disclosure, and overclaim cases.
