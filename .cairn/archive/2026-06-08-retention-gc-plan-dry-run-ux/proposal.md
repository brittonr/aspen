# Proposal: Retention GC plan dry-run UX

## Summary
Add an operator-facing retention GC plan artifact and CLI command that previews destructive retention eligibility before any subsystem mutates ledger, chunk, cache, or secret state.

## Motivation
Retention deletion is now guarded by policy, authority, supporting evidence, reference-index admission, remote-GC admission, and imported per-peer remote clearance. Operators need a deterministic dry-run view that lists these gates together for a candidate object before applying deletion, tombstoning, redaction, or compaction.

## Goals
- Emit canonical plan evidence that binds candidate object, action, subsystem, computed reference index, destructive evidence inputs, gate refs, diagnostics, and final dry-run decision.
- Keep the plan evidence-only: it MUST NOT grant authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC trust.
- Preserve the existing destructive admission boundary: apply-mode mutation still requires the subsystem to run retention admission and retention receipt generation.

## Non-Goals
- This change does not delete content, write tombstones, or make subsystem GC apply decisions by itself.
- This change does not relax remote-clearance import requirements or accept live transport receipts as clearance.
