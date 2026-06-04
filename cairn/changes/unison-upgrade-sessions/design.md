## Context

Once artifacts are content-addressed, changing code or schemas means installing new artifacts and moving metadata pointers. That is safer than mutating existing definitions, but it creates coordination work: which names move, which durable records migrate, which protocol sessions can continue, which handlers must change, and which docs/tests prove the new version works?

Unison's structured refactoring model suggests a better UX: keep the existing codebase valid, create a structured todo list, and cut over deliberately. Molten's version needs stronger distributed-runtime evidence: policy admission, capability checks, receipt validation, and rollback/cleanup records.

## Goals

- Make runtime evolution explicit, inspectable, and receipt-backed.
- Prevent alias/name changes from invalidating active sessions or durable records by surprise.
- Compute upgrade impact from artifact dependency and reverse-dependency indexes.
- Support compatibility periods where old and new artifacts coexist.
- Tie typed-storage migrations, protocol upgrades, and handler-profile changes into one plan when needed.
- Support rollback of metadata pointers and staged tasks where side effects allow it.

## Non-Goals

- Do not adopt UCM or replace Git/Cargo/Nix development workflows.
- Do not guarantee automatic migration for arbitrary semantic changes.
- Do not mutate immutable artifact content.
- Do not hide incompatible changes behind name reuse.
- Do not let upgrade sessions bypass normal policy, capability, choreography, storage, or receipt boundaries.

## Upgrade session model

An upgrade session is an artifact or control-plane record containing:

- `session_id` and content hash of the plan,
- reason and human-readable summary,
- initiator identity and capabilities,
- affected artifact ids and metadata pointers,
- dependency impact set,
- planned new artifact ids,
- required migrations and compatibility bridges,
- required handler profile changes,
- docs/transcripts/tests that must pass,
- ordered tasks with preconditions and postconditions,
- rollback and cleanup rules,
- policy refs and receipt refs.

The plan itself should be canonical and content-addressed. Mutable task status is metadata with receipts.

## Upgrade task kinds

Initial task kinds should include:

- install artifact,
- move name/alias/tag/channel pointer,
- add compatibility alias,
- deprecate artifact or name,
- migrate durable storage records,
- install protocol compatibility bridge,
- drain active sessions,
- update effect handler binding policy,
- rerun executable transcript,
- update docs metadata,
- cut over default channel,
- rollback pointer,
- garbage-collect unreferenced artifact after safety checks.

Each task has an admission gate and emits a receipt. Tasks that perform side effects must be idempotent or have explicit compensation/rollback semantics.

## Compatibility windows

Old and new artifacts may coexist. During a compatibility window:

- names may point to old default plus new candidate/channel metadata,
- active protocol sessions continue on their installed protocol artifact,
- storage loads use stored schema refs and admitted migrations rather than latest names,
- handler policies can admit both old and new effect manifests,
- docs and transcripts can compare old/new behavior where expected.

Cutover is a metadata and policy event, not mutation of old artifacts.

## Rollback and cleanup

Rollback moves metadata pointers or handler policies back to prior admitted states. It cannot erase external side effects that already occurred, so rollback receipts must distinguish reversible metadata changes from irreversible storage migrations, remote calls, or external effects.

Cleanup requires registry proof that an artifact is not referenced by active sessions, durable storage refs, receipts, policies, docs, or pinned metadata. If proof is unavailable, cleanup is denied by default.

## Policy and evidence

Upgrade sessions should route through:

- Nickel contracts for static plan shape, allowed task kinds, policy scopes, and compatibility windows.
- Basalt/UCAN for initiator authority and delegated upgrade capabilities.
- Steel predicates only for reviewed dynamic compatibility checks or custom readiness gates.
- Trellis predicates for dependency impact, replay bounds, task ordering, and cleanup safety.
- Cairn receipts for plan creation, task state transitions, cutover, rollback, and cleanup.
- Octet/Valence evidence for artifact provenance and test/transcript results.

## Open Questions

- Should upgrade sessions live in a Raft-backed control-plane registry once consensus is available?
- Which task kinds should be supported before typed storage and remote execution are implemented?
- How should distributed peers learn about cutover events while preserving local admission autonomy?
- What is the minimum CLI/TUI workflow for presenting upgrade todos without inventing a full UCM clone?
