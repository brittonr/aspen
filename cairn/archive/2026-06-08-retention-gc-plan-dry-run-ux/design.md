# Design: Retention GC plan dry-run UX

## Artifact shape
Add canonical `retention-gc-plan-v1` records with:

- schema marker `molten.retention.gc-plan.v1`;
- decision and mode (`dry-run`);
- subsystem and action;
- candidate object ref/kind/class/requester;
- embedded computed `retention-reference-index-v1` proof;
- embedded destructive evidence summary;
- per-gate records for requester, policy, authority, supporting evidence, reference-index, local-retention, remote-GC, remote-clearance, and evidence-only boundary checks;
- aggregate diagnostics and checks proving preflight-only behavior.

## Gate evaluation
The plan reuses the existing destructive-retention admission readers for policy, authority, supporting evidence, reference-index, remote-GC, and imported remote-clearance refs. It also computes the local reference index and local retention diagnostics without calling `evaluate_retention`, so no retention receipt or tombstone is written during planning.

## CLI
Add `molten test retention gc-plan` with the same explicit destructive evidence flags used by subsystem GC commands. The command writes/stores the plan and prints a concise summary.

## Safety boundary
`retention-gc-plan-v1` is preview evidence only. Apply-mode destructive subsystem paths continue to require their own retention receipt generation and deletion gate checks. The plan does not make live transport, workflow, policy, authority, source-gate, provenance, resource, or remote-GC assertions authoritative.
