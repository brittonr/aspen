# retention-candidate-bundle-export Design

## Overview
The bundle export workflow is a read-only handoff over a previously generated `retention-candidate-explain-v1` artifact. The operator supplies the retention root, explain artifact, and output directory. Molten writes a deterministic directory containing the explain artifact, a canonical bundle manifest, and every referenced local plan/apply/execute/audit/receipt/tombstone artifact it can read.

## Output Layout
The export directory contains:

- `explain.preserves` — the supplied explain artifact,
- `bundle.preserves` — canonical `retention-candidate-bundle-v1` manifest,
- `artifacts/gc-plans/*.preserves`,
- `artifacts/gc-applies/*.preserves`,
- `artifacts/gc-executes/*.preserves`,
- `artifacts/gc-audits/*.preserves`,
- `artifacts/receipts/*.preserves`,
- `artifacts/tombstones/*.preserves`.

The bundle manifest binds the explain ref, object/scope filters, grouped refs, all exported artifact refs, diagnostics, and checks.

## Missing Local Artifacts
The bundle only packages local artifacts reachable from the supplied retention root. Missing referenced artifacts are reported as diagnostics in the bundle manifest instead of minting replacement evidence.

## Safety Boundaries
The bundle is review/handoff evidence only. It MUST NOT authorize deletion, replace plan/apply/execute gates, replace destructive admission, or act as policy, authority, resource, provenance, transport, execution, source-gate, remote-GC clearance, or remote-clearance-import trust.
