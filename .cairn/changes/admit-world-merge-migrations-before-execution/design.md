# Design: Admit world merge migrations before execution

## Context and Evidence

F06 is a high-severity source finding at `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
The shell calls `materialize_migration` for each matching source schema, then overwrites `schema_ref`.
The core checks equal schemas before it checks the admission flag.
A binding with `admitted: false` can therefore reach an adapter and then evade the later schema check.

No runtime reproduction ran during the audit. The first implementation step must reproduce this sequence through the public preparation API.
The existing core baseline passed 359 tests. That result does not cover the defective shell sequence.

## Decisions

### Admission is a pure decision over original values

The core receives the original typed roots, schemas, migration binding, availability, and named bounds.
It returns an explicit conversion plan or typed denial before the shell invokes migration code.
No clock, storage, handler, or adapter lookup belongs in this decision.

### The shell executes only admitted conversion plans

The existing migration port remains the external capability boundary.
Its request must carry the exact admitted binding, not a mutable request whose schema equality serves as implicit permission.
The shell loads bounded source bytes and invokes only the requested conversion.
Result validation binds the original root and schema to the target schema and output identity.

### Conversion evidence survives normalization

Prepared values must retain the original source identity and conversion identity.
An equal target schema proves neither conversion admission nor output validity.
A typed prepared-value representation can distinguish unchanged values from admitted materialized values.
The implementation must reject impossible combinations rather than infer state from optional fields.

### Publication remains last

Current merge authority, conflict handling, and causal parent checks remain separate from migration admission.
Failed result validation emits no generated root and no merge commit.
The shell must not reuse the original content reference as the identity of changed bytes.

## Compatibility and Rollout

Review any changed prepared-value or port contract and version its canonical carrier when identity meaning changes.
Keep old conversion evidence readable only through an explicit compatibility path that cannot confer current admission.
Do not rewrite old receipts or claim that old passing plans validated this boundary.
Coordinate with `distinguish-absent-world-merge-roots` for typed output construction and combined shell tests.

## Test Design

Positive cases cover unchanged values, admitted conversion, exact result binding, mixed source/target inputs, and ordinary publication.
Negative cases cover an unadmitted flag, missing or malformed binding, wrong source, unavailable bytes, excessive output, adapter failure, and stale plan reuse.
A recording adapter must establish admission-before-call and denial-before-publication order.
Fixtures must cover the public preparation path, not only `plan_world_merge`.

## Ownership and Reuse

Molten world-merge maintainers own the deterministic conversion policy and shell integration.
The existing schema-migration contract supplies schema vocabulary, not ambient admission.
No new shared mechanism or dependency is required by this design.

## Validation and Non-claims

Run focused world-merge core and shell tests before and after implementation.
Run workspace tests, Clippy with denied warnings, scoped strict Octet checks, relevant Nix checks, and Cairn gates without suppressing existing failures.
Record exact commands and blockers rather than claim unrun checks.
This change does not prove migration semantics, adapter safety, global authority freshness, or release readiness.
