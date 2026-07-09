## Context

Molten already emits release evidence bundles, has artifact refs, dependency indexes, catalog views, transcripts, and upgrade sessions. A release/package snapshot ties those pieces into a stable, semantic view that can be named by channels while preserving exact identity.

## Design

### Snapshot artifact

A `artifact-release-snapshot-v1` binds:

- snapshot id and namespace scope;
- exact artifact refs and optional artifact-set ref;
- dependency closure digest and dependency index ref;
- docs/transcripts and expected receipt refs;
- policy/provenance/source-gate/resource evidence refs;
- schema compatibility and migration receipts;
- upgrade-session refs and rollback/cutover refs;
- caveats, non-claim boundaries, redaction profile, and signatures.

The snapshot is immutable. Channels and names are separate mutable view records that point to snapshot refs.

### Verification

Snapshot verification recomputes artifact refs, dependency closure, expected members, evidence freshness, signatures, caveats, and redaction profile. Missing, duplicate, unexpected, tampered, stale, or unauthorized members produce deny receipts.

### Channel behavior

A channel update is a name-view mutation requiring capability and policy evidence. It may point `release/stable` to a new snapshot ref, but it does not change the snapshot and does not grant deployment or execution authority.

### Functional core and shell

Pure cores build snapshot manifests, verify closures, compare expected members, evaluate caveats, and decide verification outcomes from in-memory inputs. Shells read artifacts/ledgers, write bundles, sign/verify receipts, update channel views, and render summaries.

### Non-goals

- Do not adopt Unison packages, UCM namespaces, or Unison Share APIs.
- Do not replace Nix, Cargo, Cairn release readiness, or existing signed evidence bundles.
- Do not treat channel names as trust or deployment authority.
- Do not hide caveats or stale evidence behind a green rendered summary.