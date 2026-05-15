## Context

Runtime service specs already model service identity and receipts, but Aspen lacks a first-class mutable-name-to-immutable-hash registry analogous to Unison's `ServiceName` and `ServiceHash` split.

## Goals / Non-Goals

**Goals:**
- Make deployed service identity immutable and hash-addressed.
- Make human names explicit mutable Raft state with generations.
- Support rollback by assigning a prior hash to the name.
- Emit receipt evidence for all pointer changes.

**Non-Goals:**
- Implementing an HTTP ingress layer.
- Replacing Git refs or CI pipeline names in this change.
- Dynamic native plugin ABI admission.

## Decisions

### 1. Registry state goes through Raft

**Choice:** Name assignment and update are cluster-wide state and MUST be committed through Raft.

**Rationale:** Operators need linearizable service-name resolution and auditable updates.

**Alternative:** Store name pointers in local config. Rejected because it splits cluster truth.

### 2. Hash target is immutable service deployment identity

**Choice:** `ServiceHash` initially references a validated closure/artifact/service manifest hash, not arbitrary text.

**Rationale:** Receipts and rollbacks are only meaningful if the target is immutable and validated.

### 3. Receipts include previous and next targets

**Choice:** Update receipts include name, previous hash when present, next hash, generation, actor/capability summary, and timestamp/log index.

**Rationale:** This provides rollback/audit evidence without log scraping.

## Risks / Trade-offs

**Name hijacking** → Require explicit capability admission for assign/update/rollback.

**Hash orphaning** → Registry lookup does not imply garbage-collection liveness; GC must account for active pointers later.

**Confusing name vs hash** → CLI/API output must always show both when resolving a name.
