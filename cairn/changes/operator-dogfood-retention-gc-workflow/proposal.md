# Change: Operator dogfood retention GC workflow

## Summary

Integrate the retention GC plan/apply/execute/audit and review-bundle workflow into the local operator dogfood run so release evidence proves that destructive deletion safety rails are exercised end-to-end.

## Motivation

Retention GC has explicit plan, apply, execute, audit, bundle, profile, verify, catalog, and MCP discovery artifacts. The current `molten dogfood local-node` workflow exercises node startup, services, remote dataspace, jobs, catalog, repro, and release gates, but it does not cover the retention deletion-safety chain. Operators need dogfood evidence that retention GC remains wired into release gating without granting deletion authority.

## Scope

- Add a dogfood retention GC sub-workflow using canonical retention evidence admissions, remote GC clearance, dry-run plan, apply receipt, execution gate, audit, explain, bundle export/profile/verify, and catalog/MCP search.
- Bind retention evidence refs into operator checkpoints and release/dogfood reports as evidence-only artifacts.
- Document that dogfood retention artifacts do not grant authority, policy, resource, provenance, transport, source-gate, remote-GC, execution, or deletion trust.

## Non-goals

- Replacing destructive admission, remote clearance import, retention plan/apply/execute gates, or subsystem deletion checks with dogfood receipts.
- Performing real production deletion outside the local dogfood fixture state root.
- Introducing live multi-host retention clearance in the dogfood release gate.
