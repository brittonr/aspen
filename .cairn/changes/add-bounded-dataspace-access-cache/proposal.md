## Why

Aspen routes reads to dataspaces and re-derives the same access and capability facts across requests. Repeating this work without a bounded retention contract grows memory without limit.

The stack owns the product-neutral `bounded-cache` retention contract. Aspen should own a bounded dataspace-access cache. Retention decisions belong in a pure core. Keys are BLAKE3 digests over canonical access projections.

## What Changes

- Add a pure retention decision core: canonical key projection, eviction selection, promotion disposition, watermark trim plans, and deferred-release plans from supplied values.
- Add a bounded store that executes core decisions and releases results after the guard releases.
- Key every entry by BLAKE3 digest. Require an explicit capacity. Add no implicit default bound.
- Record the cross-repo contract: the cache policy follows the `bounded-cache` retention contract, and the store can later adopt a published `bounded-cache` revision.

## Impact

- **Files**: decision core module, bounded store, tests, and this lifecycle package.
- **Testing**: workspace tests, Clippy, formatting, Cairn validation and gates, and Nix checks.

## Dependencies

New sibling component: `bounded-cache` (consumed by an exact published Git revision once available; retained as a policy reference until then).

External reference: `adanil-code/LRUHashTable` under Apache-2.0.

## Non-goals

- Do not change dataspace semantics. The cache is advisory only.
- Do not add sibling-path dependencies.
- Do not read clocks in the core.
- Do not claim that a cache hit proves record freshness or vat authority.
