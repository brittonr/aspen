# Change: nickel-contract-prelude-library

## Why

Nickel contract modules repeat common helpers such as non-empty strings, BLAKE3 refs, positive integers, exact metadata checks, array predicates, uniqueness checks, and allowed-value predicates. Copying these helpers across production profiles, multinode fixtures, peer profiles, plugin contracts, and Cairn policy contracts increases drift risk and makes contract tightening uneven.

## What

- Introduce a shared Nickel contract prelude for reusable pure predicates and contracts.
- Migrate contract modules to import the prelude for common domain helpers while keeping module-specific invariants local.
- Document the import boundary and keep runtime Rust on checked-in exports rather than runtime Nickel evaluation.

## Impact

Contract behavior becomes easier to review and reuse. Future domain hardening can update one shared helper instead of chasing copied predicates across modules.
