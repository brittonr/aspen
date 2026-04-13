## Why

`crates/aspen-coordination` now ships a real catalog of distributed primitives, but the main OpenSpec coverage still only captures a narrow slice of that surface. `openspec/specs/coordination/spec.md` talks about distributed locks and a few model-based tests, while the crate and client API now expose leader election, counters, sequences, rate limiting, barriers, semaphores, read-write locks, queues, service registry, and worker coordination.

That gap makes review harder. New coordination changes have no single spec anchor for the user-visible behavior they are supposed to preserve, and the split between native handlers and plugin-backed handlers is only implied by code.

## What Changes

- Expand the `coordination` OpenSpec domain so it describes the full coordination primitive surface exposed by Aspen today.
- Add requirements for leader election, counters, sequences, rate limiting, barriers, semaphores, read-write locks, queues, service registry, worker coordination, and the shared coordination RPC contract.
- Keep `distributed-lockset` as the detailed spec for multi-resource lock acquisition instead of duplicating those rules in the umbrella coordination spec.
- Clarify which behaviors are contract-level semantics versus backend details such as native-handler routing or plugin implementation choice.

## Capabilities

### Modified Capabilities

- `coordination-primitives`: The coordination domain covers the full primitive catalog instead of only distributed locks and a few test expectations.
- `coordination-rpc-contract`: The spec captures one wire-level contract for native and plugin-backed coordination operations.

## Impact

- **Files**: `openspec/changes/coordination-primitives-spec/` only.
- **APIs**: Documents existing coordination library and RPC APIs; no wire changes in this spec-only change.
- **Dependencies**: None.
- **Testing**: Spec text should be checked against `crates/aspen-coordination/`, `crates/aspen-client-api/`, and existing coordination tests before implementation changes rely on it.
