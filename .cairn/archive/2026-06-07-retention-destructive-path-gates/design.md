## Context

`retention-gc-pinning` introduced canonical pins, reference indexes, retention receipts, and tombstones. The next hardening step is to make storage subsystems consume those decisions before changing local state.

## Goals

- Evaluate retention before every evidence-ledger GC removal.
- Evaluate retention before every chunk-store manifest/chunk removal and before chunk tombstone evidence is emitted.
- Evaluate retention before evaluation-cache tombstones are written.
- Require secret cleanup evidence to bind passing retention receipts whose object/action/tombstone match the secret cleanup target.
- Keep subsystem receipts evidence-only while exposing retention receipt refs for audit.

## Non-Goals

- Do not make retention receipts grant authority, provenance, policy, resource, transport, execution, or source-gate trust.
- Do not introduce mutable-name based deletion eligibility.
- Do not require remote peers to delete content globally.
- Do not hide denied GC behind successful subsystem receipts.

## Design

Each destructive subsystem first computes its candidate set without mutating files or indexes. It then evaluates retention for every candidate using the existing bounded reference-index model. If any retention decision denies, the subsystem emits a deny receipt with bound retention receipt refs and performs no destructive mutation.

Dry-run commands use retention eligibility decisions and do not create tombstones. Applying commands use destructive retention actions and bind retention/tombstone refs before removing content or writing subsystem tombstone receipts.

Secret cleanup already emits a cleanup receipt, so the builder now parses supplied retention receipt values, requires at least one passing private-secret retention receipt for the secret, verifies that the cleanup tombstone matches the retention tombstone ref, and denies cleanup when evidence is missing, stale, or mismatched.

## Receipt semantics

Subsystem receipts include retention receipt refs as audit evidence. These refs are not authority tokens; callers must still provide normal authority/policy/resource/source-gate evidence where those subsystems require it.
