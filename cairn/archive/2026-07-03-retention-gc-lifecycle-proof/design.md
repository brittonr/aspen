# Design: retention GC lifecycle proof

## Scope

This change proves the destructive retention GC lifecycle. It covers dry-run plan, plan recomputation, apply, retention receipt creation, tombstone binding, execution gate, destructive subsystem mutation, audit, remote clearance/import evidence, and no-mutation denial.

## Proof checklist

- **Proof claim**: destructive mutation is reachable only from a passing plan that recomputes unchanged, a passing normal destructive admission, a passing apply receipt, a matching execution gate, and bound retention/tombstone evidence when required.
- **Out of scope**: remote peer honesty beyond imported clearance evidence and filesystem durability after a passing mutation.
- **Trusted assumptions**: content refs and stored receipt refs are canonical and immutable once written.
- **Positive evidence**: a plan→apply→execute→audit trace binds candidate, requester, policy, authority, reference index, remote clearance, retention receipt, tombstone, and subsystem action refs.
- **Negative evidence**: plan drift, denied recomputed plan, missing authority, incomplete reference index, stale remote clearance, missing apply ref, scope mismatch, missing tombstone, and no-mutation denial.
- **Canonical refs**: plan ref, recomputed plan ref, apply ref, execution gate ref, retention receipt ref, tombstone ref, audit ref, candidate object ref, admission refs, and remote clearance refs.
- **Regeneration command**: `cargo test retention`.

## Functional core

Extract and test the lifecycle decision as pure data: original plan, recomputed plan, admission result, apply, execution gate, receipt, tombstone, and audit scope produce a decision and diagnostics. Mutation call sites consume only passing decisions.

## Non-goals

- No new deletion authority.
- No remote-GC trust from request/response transport receipts alone.
