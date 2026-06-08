# Retention GC Apply From Plan UX

## Summary

Add an operator-facing retention GC apply workflow that requires a previously stored dry-run plan ref, recomputes that plan immediately before mutation, fails closed on drift, and emits a canonical apply receipt linking the original plan, recomputed plan, admitted evidence, retention receipt, and tombstone refs.

## Motivation

`retention-gc-plan-v1` gives operators a safe preview of destructive gates, but applying later must not blindly trust the preview. Active pins, reference indexes, remote-GC admissions, or imported peer clearance can change between planning and mutation. Operators need a clear apply step that proves the exact plan is still current before writing destructive retention receipts or tombstones.

## Scope

- Add canonical `retention-gc-apply-v1` receipts.
- Add a CLI command that requires `--plan-ref` and applies only from a stored plan.
- Recompute the plan from its embedded candidate and evidence before mutation.
- Deny without writing retention receipts or tombstones when the plan is denied, stale, drifted, or admission no longer passes.
- Preserve normal destructive admission and retention receipt generation for passing unchanged plans.

## Non-goals

- Treating a plan as authority, policy, remote clearance, source-gate, transport, resource, provenance, or execution trust.
- Removing subsystem-specific GC receipts.
- Adding distributed transaction semantics across peers.
