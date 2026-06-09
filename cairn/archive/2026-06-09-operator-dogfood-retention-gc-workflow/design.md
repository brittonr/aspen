# Design: Operator dogfood retention GC workflow

## Overview

`molten dogfood local-node` gains a local retention store under the explicit dogfood state root. The runner creates deterministic fixture refs for requester, object, remote peer/cache, policy, authority, supporting evidence, reference-index evidence, remote-GC evidence, and remote clearance. It then executes the same retention workflow exposed by the CLI:

1. store evidence admissions and remote clearance;
2. emit a dry-run `retention-gc-plan-v1`;
3. apply the stored plan and emit `retention-gc-apply-v1` plus normal retention receipt/tombstone refs;
4. emit `retention-gc-execute-v1`;
5. audit the execution with `retention-gc-audit-v1`;
6. explain the candidate, export/profile/verify a review bundle, and run read-only `search_retention_gc` MCP discovery.

Each stage becomes an operator step/checkpoint. Retention bundle verification and catalog/MCP receipts are added to existing dogfood report evidence, while release gate checks remain evidence-only.

## Evidence boundary

The dogfood runner records retention refs so release evidence can demonstrate that safety rails were exercised, but dogfood records never replace the normal retention gates. Any future destructive subsystem operation must still require matching retention evidence admissions, plan/apply/execute gates, remote clearance, retention receipts, and tombstones as appropriate.

## Failure behavior

Every retention dogfood step is mandatory and deterministic/recorded. If a plan, apply, execute, audit, bundle verification, or catalog/MCP search denies, the dogfood report denies and no release-gate receipt is produced.

## Validation

Coverage includes the local dogfood integration test asserting pass, retention step presence, imported retention GC artifacts, bundle verify evidence, and read-only catalog/MCP discovery. Cairn proposal/design/tasks gates and normal Rust/Octet/Nix validation remain the release rails.
