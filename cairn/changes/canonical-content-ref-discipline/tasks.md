# Tasks: Canonical Content Ref Discipline

## Phase 1: Shared ref parser

- [x] [serial] r[molten.runtime_spine.canonical_content_refs.shape] Add a shared BLAKE3 content-ref parser/newtype and replace local shape-only helpers in the first migrated modules.
- [x] [serial] r[molten.runtime_spine.canonical_content_refs.not_trust] Document and test that a well-shaped content ref does not grant authority, policy, provenance, source-gate, retention, resource, or transport trust.

## Phase 2: Node-control boundary hardening

- [x] [serial] r[molten.runtime_spine.canonical_content_refs.node_control] Require node-control request, ingress, payload, and transport receipt refs to use the shared parser.
- [x] [serial] r[molten.runtime_spine.canonical_content_refs.materialized_readback] Add missing/stale/tampered local materialization diagnostics for node-control payload/envelope readback where local storage is claimed.

## Phase 3: Runtime and harness refs

- [x] [serial] r[molten.runtime_spine.canonical_content_refs.runtime_values] Expose canonical refs for runtime values, messages, assertions, events, turn journals, and state snapshots.
- [ ] [parallel] r[molten.runtime_spine.canonical_content_refs.negative_tests] Add malformed-ref, wrong-length, non-hex, missing-artifact, and tampered-bytes regression tests. (Started: parser matrix, node-runtime short fixture denial, node-daemon short fixture denial, tampered ingress envelope materialization, ledger malformed/missing/tampered materialization tests, artifact malformed/missing/tampered/name-not-identity tests, job-ref malformed/missing/inline-denial tests, and coordination/service-runtime malformed-ref tests.)

## Phase 4: Broader migration

- [ ] [parallel] r[molten.runtime_spine.canonical_content_refs.migration] Migrate artifact registry, catalog, coordination, protocol session, service runtime, transcripts, provenance, redaction, secrets, and job DAG validators to the shared ref helper in bounded slices. (Started: artifact registry, job DAG, coordination, and service runtime validators now use the shared helper and readback checks where local materialization is claimed.)
- [ ] [serial] r[molten.runtime_spine.canonical_content_refs.negative_tests] Run Molten validation gates, Octet, Cairn strict validation, and Nix nextest for the migrated slices.
