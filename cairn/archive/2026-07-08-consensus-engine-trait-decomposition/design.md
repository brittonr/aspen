# Design: consensus engine trait decomposition

## Scope

This change refactors the consensus control-plane abstraction. It does not change the canonical Raft command, receipt, snapshot, or recovery record schemas unless a later implementation task explicitly records a schema migration.

## Proof checklist

- **Proof claim**: consensus state transition logic can be tested as a deterministic pure core and engine capability support is explicit.
- **Out of scope**: adding a new consensus algorithm or changing Raft semantics.
- **Trusted assumptions**: existing Raft transition and parser logic remains the source of behavioral truth during extraction.
- **Positive evidence**: valid proposals, reads, snapshots, and recoveries produce the same canonical refs after refactor.
- **Negative evidence**: unsupported capability calls deny explicitly; invalid commands do not mutate runtime state.
- **Canonical refs**: before/after registry receipt refs, commit receipt refs, read receipt refs, snapshot refs, recovery receipt refs.
- **Regeneration command**: focused consensus tests and property tests.

## Functional core

Define pure transition inputs and outputs for proposal admission and state-machine application. The core returns the proposed log entry, registry receipt, commit receipt, next state, diagnostics, and idempotency result without mutating the runtime object.

## Imperative shell

The shell receives a pure transition result, updates `ControlRegistryRuntime`, writes receipts or durable store records, and exposes the existing public API. Unsupported capability shells call explicit denial paths.

## Trait shape

Use separate traits for descriptor/readback, proposal transitions, reads, snapshots, and recovery. Engines implement only the capabilities they support, and production admission checks declared capabilities against implemented traits and conformance receipts.
