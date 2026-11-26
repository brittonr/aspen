# Runtime spine deltas

## ADDED Requirements

### Requirement: Canonical packed Preserves encoding, decoding, hashing, and Embedded-aware walking live in a no_std kernel
r[molten.runtime_spine.no_std_preserves_packed_kernel] Aspen MUST provide a `#![no_std]` + `alloc` `molten-preserves-core` workspace crate implementing bounded packed preflight, strict canonical Preserves binary encoding and decoding, byte-exact equivalence with `preserves` packed output, strict lowercase `ContentRef` validation, unique `<some>`/`<none>` optional-ref encoding, BLAKE3 identity hashing, and `Embedded`-aware bounded structural traversal.

#### Scenario: Differential equivalence matches preserves packed bytes
- GIVEN any canonical value generated across all Preserves value classes (including nested sets, dictionaries, records, signed integers, floats, and `Embedded`)
- WHEN encoded by `molten-preserves-core` and by `preserves::PackedWriter`
- THEN the encoded byte sequences and BLAKE3 digests MUST be byte-identical.

#### Scenario: Non-canonical bytes, depth exhaustion, or Embedded markers fail closed
- GIVEN non-canonical set/dictionary ordering, over-budget nesting depth, uppercase `blake3:` hex, or a prohibited token inside an `Embedded` (`0x86`) value
- WHEN `molten-preserves-core` decodes or scans the payload
- THEN it MUST reject non-canonical/over-budget bytes and MUST detect the marker inside `Embedded`.
