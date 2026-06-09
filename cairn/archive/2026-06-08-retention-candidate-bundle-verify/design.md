# retention-candidate-bundle-verify Design

## Overview
The verification workflow reads an exported retention candidate bundle directory and emits a canonical verification receipt. It is read-only and operates entirely over the supplied bundle directory.

## Verification Steps
The verifier:

1. Parses `bundle.preserves` as `retention-candidate-bundle-v1`.
2. Parses `explain.preserves` as `retention-candidate-explain-v1` and checks its canonical ref matches the bundle's explain ref.
3. Compares object and scope fields between the bundle and explain artifact.
4. Compares grouped plan/apply/execute/audit/receipt/tombstone refs with the bundle artifact manifest.
5. Reads expected files under `artifacts/gc-plans`, `artifacts/gc-applies`, `artifacts/gc-executes`, `artifacts/gc-audits`, `artifacts/receipts`, and `artifacts/tombstones`.
6. Recomputes each packaged artifact ref and validates the expected artifact kind.
7. Scans packaged files to report duplicate refs, unreferenced files, missing listed refs, and unlisted files.

## Receipt
`retention-candidate-bundle-verify-v1` records the bundle ref, explain ref, object/scope, listed artifact refs, observed file refs, decision, diagnostics, and evidence-only checks.

The decision is `pass` only when the bundle is internally consistent and every referenced local artifact is present, parseable, canonical, and listed. Any missing, tampered, duplicate, or unreferenced packaged artifact causes `deny` diagnostics.

## Safety Boundaries
Verification evidence is review/handoff evidence only. It MUST NOT authorize deletion, import remote clearance, replace destructive admission, or act as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust.
