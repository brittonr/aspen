# Validation evidence

## Scope

This change adds deterministic dataspace-access key and retention decisions.
It also adds a bounded runtime store with deferred release.

Base source commit: `812a1fd679b677cccf74179b1677fd9ca1b80f53`.

## Baseline

`nix develop -c cargo test -p molten-core` passed before implementation.
The baseline ran 179 tests with no failures.

The proposal, design, and tasks gates passed before implementation.

## Functional core

`crates/molten-core/src/fabric_durability/cache/` owns the pure decisions.
The core validates explicit capacity, watermarks, promotion threshold, projection bounds, and eviction order.

Key projection uses domain-separated BLAKE3 framing.
It binds dataspace identity, normalized arguments, and optional capability context.

The core provides strict LRU at threshold `0` and FIFO at threshold `100`.
Intermediate thresholds use deterministic supplied access counts.

## Imperative shell

`src/runtime/dataspace/cache/` owns the mutex, bounded values, loader calls, and deferred release.
A miss calls the supplied loader outside the mutex.
A loader error returns without adding an entry.

The store retains values in `Arc` objects.
It drops evicted values only after it releases the mutex.
A reentrant destructor fixture proves that the mutex is available during eviction cleanup.

## Positive and negative validation

The focused matrix covers these cases:

- equal and distinct key projections
- optional capability-context changes
- missing capacity
- reversed watermarks
- invalid thresholds
- strict LRU, lazy promotion, and FIFO
- high-watermark trimming
- a single-slot store
- duplicate eviction keys
- loader pass and denial
- deferred release after the guard

`nix develop -c cargo test -p molten-core` passed after implementation.
`nix develop -c cargo test -p molten` passed after implementation.
The root package ran 1,260 library tests, 51 binary tests, and 119 integration tests.

`nix develop -c cargo clippy -p molten-core -p molten --all-targets -- -D warnings` passed.
`nix develop -c cargo fmt --check` passed.

The focused Octet runs reported existing workspace findings.
They reported no finding for the new core or shell files.
These warning-only runs are not strict Octet acceptance evidence.

## Nix

`nix build .#checks.x86_64-linux.molten --no-link -L` passed with local builders.
The Nix nextest rail ran 1,373 tests with no failures or skips.
Its CI receipt is `blake3:54312c9c9bf46c1260bd32f0bec73d15485ce0fabb13ba7b8fc126290742ac53`.

## Cairn

Strict Cairn validation passed before sync.
The result covered 77 accepted specifications and had no issues.

Final gate receipts before sync:

- proposal: `78eee502209ee61f3bf3b45f4fdf7b41210015bd9b27999d354e9a5c8fce2a4a`
- design: `beaa47be82d97b622437216c438c97961f258fad955a625679730cf7bff9ef5d`
- tasks: `50a9f0b3ea4a44001b83c52fafa1c2337c509a7986cb32c60188168e67ad2ffd`

The sync dry-run passed with plan `8afd3a117190011224c5cdec1c01e3cb82e851600f82a33cefc3173b7b39fb59`.
The executed sync added all six requirement identifiers to the accepted specification.
Strict validation passed after sync and covered 78 specifications.

The archive dry-run passed with plan `19b809c89cdba3e74c4d6e4b6dab4ed9de1d14673f3d97dbc37df1110d24714e`.
Archive execution moved the package to `2026-08-24-add-bounded-dataspace-access-cache`.
The archive receipt is `62d9a329ceadd12a0c86fe3e0c8e2a7677c36b0935158dd47e011fcf5c5b1d94`.
Strict validation passed after archive execution.

## Cross-repository boundary

No published `bounded-cache` repository is available in the local OnixResearch workspace.
Molten keeps the implementation local until a compatible immutable component is published.
A sibling checkout is not a product dependency.

## Non-claims

A cache hit does not prove dataspace freshness or vat authority.
This change does not prove policy currentness, durability, remote availability, or release readiness.
The external LRUHashTable project remains a bounded, non-parity design reference.
