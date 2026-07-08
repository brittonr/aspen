## Why

Core DTOs still use raw `String` values for content refs, decisions, check statuses, schema ids, plugin ids, operation names, replay classes, and profile names. Raw strings make invalid states easy to construct inside pure cores and force many parsers to re-check the same invariants late.

## What Changes

- Promote common domains into small Rust newtypes or enums with pure constructors and deterministic parse/format behavior.
- Reuse the existing `ContentRef` type more broadly for canonical BLAKE3 refs.
- Introduce typed decisions, check statuses, schema ids, stable ids, operation ids, replay classes, and profile names where they cross trust or evidence boundaries.
- Keep wire-compatible Preserves and JSON rendering by converting typed domains at boundary builders/parsers.

## Impact

- **Files**: core Preserves rail, plugin host, capability, chunk store, consensus, evidence, and high-risk parser DTOs.
- **Testing**: positive construction/roundtrip tests and negative invalid-domain tests prove typed DTOs fail before canonical evidence is built.
