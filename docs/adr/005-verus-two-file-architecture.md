# 5. Verus Two-File Verification Architecture

**Status:** accepted

## Context

Verus is a formal verification tool for Rust that proves properties hold for all possible inputs. Aspen uses Verus to verify correctness of pure functions (lock expiration, fencing token monotonicity, queue ordering, etc.).

Verus requires its own syntax extensions (`verus! { }` blocks, `spec fn`, `proof fn`, `requires`/`ensures` clauses). Code inside `verus!` blocks cannot be compiled by standard `cargo build`. This creates a tension: production code must compile with cargo, but verified specs need Verus syntax.

## Decision

Use a two-file architecture:

1. **Production code** (`src/verified/*.rs`): Pure functions compiled by standard cargo. No Verus syntax. These are the functions that ship.
2. **Verification specs** (`verus/*.rs`): Standalone Verus files with `spec fn`, `proof fn`, and `ensures` clauses. Verified by `nix run .#verify-verus`. Never compiled by cargo.

The two files must be kept in sync manually — the production function's logic must match the Verus exec function's body. Verus specs reference the same function signatures and prove properties about them.

Current verified crates:

- `aspen-coordination`: 20 verified modules, 27 spec files (locks, elections, queues, barriers)
- `aspen-raft`: 10 verified modules, 13 spec files (storage, chain integrity, batching)
- `aspen-core`: 1 verified module, 6 spec files (HLC, tuple encoding, directory ops)

Alternatives considered:

- (+) Inline Verus in production code: single source of truth
- (-) Inline Verus: breaks `cargo build`, requires Verus toolchain for all developers
- (+) Two-file: production code compiles normally, zero runtime overhead
- (-) Two-file: manual sync between files, drift risk
- (+) Only tests (no formal verification): simpler toolchain
- (-) Only tests: can't prove properties for ALL inputs, only sampled ones

## Consequences

- `cargo build` works without installing Verus — no toolchain dependency for normal development
- `nix run .#verify-verus` checks proofs separately, typically in CI
- Zero runtime overhead — Verus specs are never compiled into the binary
- Drift between production and spec files is a real risk; discipline and review are required
- Trusted axioms (CAS linearizability, clock monotonicity, bounded skew) are documented in each `verus/lib.rs`
