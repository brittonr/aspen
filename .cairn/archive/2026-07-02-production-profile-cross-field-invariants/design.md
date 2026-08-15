# Design: Production profile cross-field invariants

## Context

The production profile is a compact declaration of state layout, evidence refs, required adapters, and resource limits. Its correctness depends on relationships between fields that plain record contracts do not express.

## Invariant groups

Validate these relationships at export time:

- Required evidence arrays are non-empty where production startup cannot proceed without evidence, especially source-gate inputs.
- Required adapters include the reviewed core production adapter set.
- State layout subdirectories are unique relative names so two logical stores do not share one directory by accident.
- Resource limits are coherent, including store capacity being at least as large as the maximum receipt size and timing limits preserving the intended delivery-before-recovery ordering.

## Implementation approach

Keep scalar contracts on individual fields and add one profile-level custom validator for relationships that need the whole record. The validator must be immediate, deterministic, and idempotent. It should return focused diagnostics naming the violated relationship and avoid filesystem reads or environment-dependent checks.

## Boundaries

Nickel verifies profile consistency only. Runtime startup still checks current source-gate receipts, adapter preflight evidence, permissions, available storage, and live resource pressure before side effects.
