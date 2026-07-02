## Why

Strict Octet source-gate validation records consumer source scopes, but downstream validation currently trusts coarse receipt checks rather than requiring evidence that the bound object corpus covers the consumer's full source scope. The same path also treats ref-shaped strings as content refs when they merely have a `blake3:` or `b3:` prefix.

## What Changes

- Require strict Octet gate receipts to prove configured source-gate scope coverage before downstream source-gate validation can pass.
- Reject malformed Octet artifact refs and object-set hashes with non-canonical BLAKE3 hex suffixes.
- Add positive and negative regression tests for source-scope coverage and canonical ref denial paths.

## Impact

- **Files**: `src/octet/parts/gate/*`, gate regression tests, evidence-gates spec delta
- **Testing**: focused Octet gate/source-gate tests plus formatting and Cairn validation
