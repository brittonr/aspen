## Why

Molten has more than one content-ref newtype and many parsed DTOs still store canonical refs as raw strings. Duplicate ref types and raw strings make invalid refs easy to pass internally and force repeated validation at every use site.

## What Changes

- Promote a single shared canonical content-ref type from `preserves_rail` for BLAKE3 Preserves content refs.
- Use the shared type in runtime envelopes and selected artifact, typed-storage, job, eval-cache, catalog, and schema DTOs.
- Preserve CLI and Preserves wire formats as strings while validating once at parse boundaries.
- Add serde, ordering, parse, display, and negative tests for invalid prefixes, uppercase hex, wrong length, and non-hex input.

## Impact

- **Files**: `preserves_rail`, runtime envelope, artifact/typed storage refs, job/eval/catalog/schema DTOs, and tests.
- **Testing**: valid refs remain string-compatible; invalid refs are rejected earlier and cannot be represented by the shared type.
