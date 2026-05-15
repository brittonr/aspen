## Context

Aspen stores durable state through KV, Redb, blobs, and secret/config mechanisms. A typed facade can reduce repeated key/serialization mistakes without hiding the important migration and compatibility boundaries.

## Goals / Non-Goals

**Goals:**
- Provide typed durable resource handles with explicit schema hashes.
- Keep serialization, migration, and redaction deterministic and reviewable.
- Make transactions and batched reads easy for service/job code.

**Non-Goals:**
- Inventing a new database engine.
- Hiding incompatible schema evolution.
- Supporting arbitrary ad hoc unbounded scans.

## Decisions

### 1. Schema hash is part of durable type identity

**Choice:** Each typed resource records codec version and schema/type hash alongside values or metadata.

**Rationale:** Aspen should learn from Unison typed storage while preserving explicit compatibility proofs.

### 2. Start with Cell and OrderedTable

**Choice:** Implement `Cell<T>` and `OrderedTable<K,V>` first because they map directly to existing KV/table behavior.

**Rationale:** This proves the facade without broad storage churn.

### 3. Secrets remain separate redacted resources

**Choice:** `ConfigSecret` exposes handles and redacted summaries, not raw secret values in receipts.

**Rationale:** Receipt safety is a product/security boundary.

## Risks / Trade-offs

**Schema hash churn** → Version codecs and provide migration hooks later.

**False sense of type safety** → Runtime validation must reject mismatches; docs must state this is not compiler-level cross-version proof.

**Unbounded scans** → OrderedTable range APIs must require explicit bounds/limits.
