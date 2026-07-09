# Artifact Release Snapshots Specification

## Purpose

Defines immutable release/package snapshot artifacts and mutable channel views for exact artifact closures.

## ADDED Requirements

### Requirement: Release snapshots are immutable exact-ref artifacts
r[molten.release_snapshots.namespace_snapshot_artifacts] Molten MUST define immutable release/package snapshot artifacts that bind namespace scope, exact artifact refs, dependency closure digest, docs/transcripts, policy refs, provenance refs, source-gate refs, resource refs, compatibility refs, migration refs, caveats, redaction profile, signatures, and non-claim boundaries.

#### Scenario: Snapshot records exact artifact closure
- GIVEN an operator creates a release snapshot for a named package scope
- WHEN Molten builds the snapshot artifact
- THEN it records exact artifact refs, dependency closure digest, docs/transcripts, evidence refs, caveats, redaction profile, and signatures.

#### Scenario: Name-only snapshot denies
- GIVEN a snapshot request lists artifacts only by mutable names or channels
- WHEN Molten validates the snapshot artifact
- THEN it denies until exact artifact refs or admitted resolution receipts are bound.

### Requirement: Snapshot verification recomputes closure integrity
r[molten.release_snapshots.closure_integrity] Molten MUST verify snapshot closure integrity by recomputing artifact refs, dependency indexes, expected members, signature subjects, caveats, evidence freshness, and redaction profile before accepting snapshot pass evidence.

#### Scenario: Valid snapshot verifies
- GIVEN all snapshot members are present, hashes match, signatures bind the expected subject, and evidence freshness checks pass
- WHEN Molten verifies the snapshot
- THEN it emits a passing snapshot verification receipt.

#### Scenario: Tampered member denies
- GIVEN a snapshot member's bytes no longer match its recorded artifact ref
- WHEN Molten verifies the snapshot
- THEN verification denies
- AND diagnostics identify the tampered member ref.

### Requirement: Channels are mutable non-authority views
r[molten.release_snapshots.channel_view_non_authority] Molten MUST model release channels as mutable name view records that point to immutable snapshot refs and MUST NOT treat channel names as authority, deployment approval, execution trust, provenance, or policy admission.

#### Scenario: Authorized channel update points to new snapshot
- GIVEN an operator has admitted capability and policy evidence to update a channel
- WHEN the channel is moved from snapshot S1 to snapshot S2
- THEN Molten emits a channel view receipt
- AND S1 and S2 remain immutable and addressable.

#### Scenario: Channel-only trust denies deployment
- GIVEN a snapshot is reachable through a channel named `stable`
- WHEN deployment or execution admission lacks required release, policy, provenance, source-gate, authority, or resource evidence
- THEN Molten denies admission
- AND reports that channel names are non-authority.

### Requirement: Snapshot readback surfaces caveats
r[molten.release_snapshots.evidence_caveats] Molten MUST surface caveats, pilot scope, stale evidence, redactions, unsupported claims, and non-claim boundaries in snapshot summaries, catalog views, and verification receipts.

#### Scenario: Pilot caveat is rendered
- GIVEN a snapshot is scoped to an internal pilot
- WHEN Molten renders snapshot readback
- THEN the pilot caveat appears in the summary and verification receipt.

#### Scenario: Hidden caveat denies release promotion evidence
- GIVEN a snapshot verification request omits or hides a required caveat
- WHEN Molten evaluates the snapshot for promotion evidence
- THEN verification denies until the caveat is bound and rendered.

### Requirement: Release snapshot validation covers positive and negative paths
r[molten.release_snapshots.validation] Molten MUST include positive and negative fixtures for snapshot creation, verification, channel update, tampered members, missing closure members, stale evidence, redaction, unauthorized channel moves, rollback, and channel-only trust denial.

#### Scenario: Snapshot verification fixture passes
- GIVEN a fixture has exact refs, complete closure, valid signatures, fresh evidence, and caveats
- WHEN validation runs
- THEN Molten emits a passing snapshot verification receipt.

#### Scenario: Missing closure fixture denies
- GIVEN a fixture omits a required dependency from the snapshot closure
- WHEN validation runs
- THEN verification denies
- AND diagnostics identify the missing dependency ref.