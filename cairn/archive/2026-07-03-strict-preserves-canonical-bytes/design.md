# Design: strict Preserves canonical bytes

## Scope

This change hardens packed Preserves decode paths that claim to consume canonical bytes. It covers pure decode helpers and shell call sites that read bytes from files, databases, network payloads, Wasm memory, or transport stores.

## Proof checklist

- **Proof claim**: any value accepted through a canonical byte boundary was encoded exactly as `preserves_rail::canonical_bytes` would encode it.
- **Out of scope**: changing the canonical packed encoding algorithm or textual `.preserves` diagnostics.
- **Trusted assumptions**: the upstream Preserves writer remains deterministic for supported `IOValue` values.
- **Positive evidence**: canonical bytes produced by Molten roundtrip unchanged and preserve refs.
- **Negative evidence**: parseable but non-canonical, trailing, truncated, or tampered bytes deny before import or side effects.
- **Canonical refs**: accepted value ref, raw byte content ref, boundary kind, and mismatch diagnostics.
- **Regeneration command**: `cargo test preserves_rail ledger chunk typed remote wasm`.

## Functional core

Add a pure strict decoder that parses packed bytes, re-encodes the resulting value, compares byte-for-byte, and returns a typed decode result carrying the value, canonical bytes, and content ref. Imperative shells only read bytes and map strict decode errors into existing receipts or diagnostics.

## Non-goals

- No compatibility mode that silently normalizes non-canonical input at trust boundaries.
- No change to human-readable text export commands beyond preserving diagnostic output.
