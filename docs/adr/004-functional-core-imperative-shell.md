# 4. Functional Core, Imperative Shell

**Status:** accepted

## Context

Aspen's codebase mixes business logic with I/O, async runtime interactions, and system calls. This makes functions hard to test in isolation, incompatible with deterministic simulation (madsim), and impossible to formally verify with Verus (which requires pure, deterministic functions).

The Functional Core, Imperative Shell (FCIS) pattern separates code into:

- **Core**: Pure, deterministic functions with no I/O, no async, time as an explicit parameter
- **Shell**: Thin async handlers that call core functions, manage I/O, and interact with stores

## Decision

Each crate that contains business logic implements FCIS via a `src/verified/` module:

- `src/verified/*.rs` — Pure functions compiled by standard cargo. No I/O, no async, no system calls. Time and randomness passed as explicit parameters.
- `src/*.rs` (shell layer) — Async handlers that call verified functions, manage I/O, interact with KV stores, and add error context.

Pattern example from `aspen-coordination`:

```rust
// Verified core (src/verified/lock.rs)
fn compute_next_fencing_token(current: Option<&LockEntry>) -> u64
fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> bool

// Imperative shell (src/lock.rs)
async fn try_acquire(&self) -> Result<LockGuard<S>, CoordinationError>
```

Alternatives considered:

- (+) No separation: simpler code, less boilerplate
- (-) No separation: can't unit-test logic without mocking I/O, can't use Verus
- (+) Trait-based abstraction: mock I/O via traits
- (-) Trait-based abstraction: proliferates generic parameters, doesn't help with Verus
- (+) FCIS: testable core, Verus-verifiable, madsim-compatible
- (~) FCIS: requires discipline to keep the boundary clean

## Consequences

- Verified functions are unit-testable without any I/O setup
- Verus can formally verify the core functions (see ADR-005)
- Madsim deterministic tests work because core functions have no hidden time dependencies
- Property-based testing with proptest/Bolero targets the pure core directly
- The shell layer stays thin — mostly wiring, error context, and async coordination
- New features require thinking about which logic is pure vs. which is I/O — a small upfront cost that pays off in testability
