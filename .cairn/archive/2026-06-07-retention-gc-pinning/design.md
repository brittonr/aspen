## Context

Content addressing makes artifacts and blobs easy to reference, but hard to safely delete. Deterministic replay and Cairn receipts further increase retention obligations. Molten needs explicit retention classes and pinning so storage can be reclaimed without undermining evidence.

## Goals

- Track all references that prevent deletion of artifacts, blobs, receipts, snapshots, traces, cache entries, and storage records.
- Make retention policies explicit and policy-gated.
- Prove GC eligibility before deletion.
- Keep deletion/compaction auditable with receipts.
- Support privacy-motivated deletion/redaction without pretending evidence remains complete.

## Non-Goals

- Do not delete content solely because no mutable name points to it.
- Do not garbage collect receipt/evidence dependencies needed for active or retained records.
- Do not make public artifact availability imply permanent local pinning.
- Do not hide deletion from replay or audit surfaces.

## Pin sources

Pin sources include:

- active actors, sessions, vats, jobs, and handler bindings,
- installed protocols and projected endpoints,
- typed durable refs and storage records,
- receipts and evidence chains,
- snapshots and replay logs,
- executable transcripts and expected outputs,
- docs and catalog entries,
- upgrade sessions and rollback plans,
- artifact aliases/tags/channels/releases,
- remote sync cache pins,
- operator/legal/compliance holds.

Each pin has scope, reason, owner, expiry/renewal policy, and evidence refs.

## Retention classes

Retention classes may include:

- `ephemeral_cache`
- `debug_trace`
- `replay_snapshot`
- `audit_receipt`
- `durable_value`
- `public_artifact`
- `private_secret_ref`
- `upgrade_rollback`
- `legal_hold`

Policies define minimum retention, maximum retention where applicable, redaction rules, encryption requirements, and deletion authority.

## GC eligibility

An object is eligible only if:

- no active pin requires it,
- no retained receipt/evidence chain requires it,
- no durable typed ref or snapshot references it,
- no upgrade rollback/cleanup task depends on it,
- policy admits deletion for its retention class,
- remote replicas/cache contracts are considered where relevant.

If proof is incomplete, deny deletion by default.

## Deletion and compaction

Deletion is an operation with receipts. Some objects may be physically deleted, some tombstoned, some redacted, and some compacted into summary evidence. Redaction must preserve enough metadata to explain that content was intentionally removed and under which authority, without leaking secrets.

## Open Questions

- Which reference index should be authoritative before Raft-backed control-plane state exists?
- How should remote peers communicate deletion/tombstone state?
- What retention classes are mandatory for the first milestone?
