# retention-bundle-export-profiles Design

## Overview
`retention bundle-export` gains a `--profile` argument:

- `internal` keeps the existing full-fidelity local review bundle and records a pass profile receipt.
- `public` scans the explain, bundle manifest, and packaged local artifacts for sensitive retention markers. If any are found, it records a deny profile receipt.
- `diagnostic` scans the same inputs, records marker evidence, and writes a separate `redacted/` review view where sensitive marker tokens are replaced by redaction-marker placeholders.

The canonical source bundle remains unchanged so `retention bundle-verify` continues to verify canonical artifact refs. Diagnostic redacted views are for human review and are not a replacement for bundle verification.

## Sensitive Markers
The profile scanner treats sensitive record labels and strings such as `secret`, `confidential`, `credential`, `private`, `encrypted-ref`, `secret-ref-v1`, `encrypted-ref-v1`, and `private-secret-ref` as sensitive markers. Each finding is represented by a deterministic marker ref derived from the bundle ref, path, and token.

## Receipt
`retention-candidate-bundle-profile-v1` records profile, loss classification, decision, bundle ref, marker refs, diagnostics, and evidence-only checks.

## Safety Boundaries
Profile receipts and redacted views are review evidence only. They MUST NOT authorize deletion, import trust, replace verification, or grant authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust.
