# Artifact Registry Delta: Dependency Impact Index

## ADDED Requirements

### Requirement: Dependency edges are canonical records
r[molten.artifacts.dependency_edge_records] Molten MUST model direct dependency facts as canonical records that bind source ref, target ref, relation, requiredness, scope, and evidence refs for artifact, schema, policy, effect, capability, handler profile, storage, transcript, migration, and release relationships.

#### Scenario: Direct schema dependency is recorded
- GIVEN an artifact declares that its input payload uses schema ref S
- WHEN Molten admits the artifact metadata
- THEN it records a direct dependency edge from the artifact ref to schema ref S
- AND the edge has a stable relation and evidence refs.

#### Scenario: Opaque inferred dependency is not authoritative
- GIVEN a source file text mentions a schema name but no canonical dependency edge exists
- WHEN Molten computes normative impact analysis
- THEN the text mention is diagnostic only
- AND mutation gates require a canonical edge or explicit evidence explaining the omission.

### Requirement: Reverse dependency indexes are rebuildable
r[molten.artifacts.reverse_dependency_index] Molten MUST maintain reverse dependency indexes that can be deterministically rebuilt from canonical registry and ledger dependency-edge records.

#### Scenario: Rebuild produces stable reverse index
- GIVEN the same sorted dependency-edge records and redaction policy
- WHEN Molten rebuilds the reverse index twice
- THEN both rebuilds produce identical reverse index digests
- AND the rebuild receipt binds the source edge set.

#### Scenario: Stale reverse index denies normative impact gate
- GIVEN a reverse index digest does not match the current edge set
- WHEN an upgrade, retention, or release gate requires impact evidence
- THEN Molten denies use of the stale index
- AND diagnostics require rebuild or diagnostic-only treatment.

### Requirement: Impact queries emit planning receipts
r[molten.artifacts.impact_query_receipts] Molten MUST emit impact query receipts that bind query subject, relation filters, direct dependents, requested transitive dependents, redaction decisions, index refs, and diagnostics.

#### Scenario: Upgrade planning receives dependents
- GIVEN a schema ref is targeted for migration
- WHEN Molten computes an impact query
- THEN the receipt lists direct and requested transitive dependents
- AND the upgrade session can bind that receipt as planning evidence.

#### Scenario: Redacted dependency is not leaked
- GIVEN an impact query runs under a public catalog profile and a dependent ref is private
- WHEN Molten renders the query result
- THEN the receipt records a redaction decision
- AND the public view does not reveal the hidden target.

### Requirement: Index rebuilds are deterministic
r[molten.artifacts.index_rebuild_determinism] Molten MUST sort and canonicalize dependency edges, duplicate handling, cycle diagnostics, and reverse-index entries so the same registry and ledger inputs produce the same index digest.

#### Scenario: Duplicate edges are normalized
- GIVEN two equivalent dependency-edge records are present
- WHEN the reverse index is rebuilt
- THEN Molten records duplicate diagnostics
- AND the canonical index output is deterministic.

#### Scenario: Cycle diagnostics are stable
- GIVEN artifacts depend on each other cyclically
- WHEN impact analysis computes transitive dependents
- THEN traversal terminates with stable cycle diagnostics
- AND no unbounded walk or nondeterministic order is used.

### Requirement: Dependency index validation covers positive and negative paths
r[molten.artifacts.dependency_index_validation] Molten MUST include positive and negative fixtures for complete graphs, missing edges, duplicate edges, cycles, stale indexes, and unauthorized hidden dependency exposure.

#### Scenario: Complete graph fixture passes
- GIVEN a fixture with declared artifacts, edges, and expected reverse dependents
- WHEN validation runs
- THEN Molten recomputes the expected index digest and passes.

#### Scenario: Missing edge fixture denies
- GIVEN a fixture omits a required dependency edge for an artifact that declares the target in canonical metadata
- WHEN validation runs
- THEN Molten emits deny diagnostics
- AND impact evidence from that graph cannot satisfy mutation gates.