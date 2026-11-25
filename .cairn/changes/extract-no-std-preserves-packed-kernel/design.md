# Design

## `crates/molten-preserves-core` Architecture (`#![no_std]` + `alloc`)

- Move `src/preserves/packed/bounds.rs` into `crates/molten-preserves-core/src/bounds.rs` as the first-stage preflight validator enforcing explicit byte, depth, container-length, and total-node ceilings before allocation.
- Define a deterministic `no_std + alloc` value model (`PackedValue` backed by `alloc::vec::Vec`, `alloc::string::String`, `alloc::collections::BTreeSet`, and `alloc::collections::BTreeMap` ordered strictly by canonical packed byte order—matching Preserves canonical set/dictionary ordering).
- Implement:
  - **Strict canonical binary encoder & decoder:** Full Preserves binary encoding (`0x80..0xB7`), verifying canonical varint lengths, canonical signed integer minimalism, canonical UTF-8 strings/symbols, sorted unique set elements, sorted unique dictionary keys, canonical NaN/float handling, and strict round-trip byte equality (`strict_canonical_decode`).
  - **Canonical BLAKE3 identity & typed references:** `ContentRef` requiring strict 64-char lowercase hex after `blake3:`, domain-separated hashing over canonical packed bytes, and unambiguous `<some "blake3:...">` / `<none>` optional reference encoding (rejecting bare-string ambiguity).
  - **Bounded `Embedded`-aware structural walker:** Traverse records, sequences, sets, dictionaries, annotations (if permitted by policy), and `Embedded` (`0x86`) payloads under explicit depth and node budgets, supplying `no_std` primitives for ambient-token, sensitive-marker, and content-reference detection.

## Thin `std` Adapter & Differential Oracle

- Keep `src/preserves/` in `molten` as a thin `std` bridge converting `PackedValue <-> preserves::IOValue` and handling Preserves text syntax for CLI/docs/fixtures.
- Add Hegel differential property tests in `src/preserves/` proving that for every generated canonical value, `molten_preserves_core::encode_canonical` and `preserves::PackedWriter` produce byte-identical output and identical BLAKE3 digests, and that non-canonical packed inputs rejected by either decoder are rejected by `molten-preserves-core`.
- Enforce `#![no_std]` compilation in `flake.nix` and workspace Octet checks.
