## Why

Several Nickel contracts validate that arrays contain well-formed elements, but do not consistently declare uniqueness, sortedness, non-empty, maximum-length, or membership invariants. For evidence refs, adapter lists, policy refs, lifecycle callbacks, and scenario refs, duplicate or oversized arrays can make review ambiguous even when every element is valid.

## What Changes

- Add shared Nickel prelude helpers for unique arrays, non-empty unique arrays, bounded arrays, bounded ref arrays, and subset/contains-all combinations.
- Apply the helpers to production profiles, peer profiles, multinode scenarios, plugin extension contracts, plugin grants, and Cairn policy contracts where uniqueness or bounds are part of the reviewed domain.
- Add targeted negative fixtures for duplicates, oversized arrays, missing required members, and contradictory list semantics.

## Impact

- **Files**: `docs/nickel-contract-prelude.ncl`, repository-owned Nickel contract modules, fixtures, generated JSON, and flake drift gates.
- **Testing**: valid fixtures export; duplicate refs, duplicate adapters, oversized refs, missing required members, and duplicate descriptor identities fail export.
