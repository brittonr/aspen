# Tasks: vm-evidence-child-manifest

## Phase 1: Manifest expansion

- [x] [serial] r[molten.testing.vm_evidence.child_artifact_manifest_completeness] Add all live-control, service/job, coordination, fault, shard, aggregate, validation, and prod-soak child receipts to VM manifest emission.
- [x] [parallel] r[molten.testing.vm_evidence.child_artifact_manifest_completeness] Ensure diagnostic logs are included only with diagnostic-only classification and stable content refs.

## Phase 2: Closure validation

- [x] [serial] r[molten.testing.vm_evidence.manifest_reference_closure] Add pure manifest closure validation for missing child refs, duplicate paths, duplicate semantic artifacts, wrong kind, content-ref mismatch, unreferenced required evidence, and log-only child claims.
- [x] [parallel] r[molten.testing.vm_evidence.manifest_reference_closure] Wire closure validation into `molten test nixos-vm validate` or a focused manifest validation command.

## Phase 3: Fixtures and checks

- [x] [parallel] r[molten.testing.vm_evidence.child_artifact_manifest_completeness] Add positive fixtures for complete monolithic and sharded VM manifest closures.
- [x] [parallel] r[molten.testing.vm_evidence.manifest_reference_closure] Add negative fixtures for missing child, tampered content, duplicate path, wrong kind, unreferenced required child, and log-only child.
- [x] [serial] r[molten.testing.vm_evidence.manifest_reference_closure] Run focused VM manifest tests and update traceability coverage.
