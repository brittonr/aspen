## Why

Molten already treats Preserves as the canonical communication and evidence spine, but many boundary records still rely on manual record parsing and local shape checks. That keeps boundary compatibility implicit in Rust helper code and makes it harder to prove that schema identity, canonical bytes, and routing patterns are stable across storage, transport, plugin, and runtime surfaces.

## What Changes

- Promote schema-backed typed codecs for more high-risk Preserves boundary families while keeping semantic admission in Molten's pure core.
- Require strict canonical decode before schema or semantic admission for external bytes.
- Bind schema refs, value refs, and compatibility decisions into boundary receipts and diagnostics.
- Define a reusable bounded Preserves pattern AST for dataspace routing and policy-visible matching instead of equality-only matching.
- Add canonical positive and negative fixture corpora for boundary decoding, schema validation, and pattern routing.

## Impact

- **Files**: `preserves_rail`, runtime dataspace pattern modules, boundary parsers/builders, schema artifacts, tests, and documentation.
- **Testing**: valid canonical fixtures continue to pass; non-canonical bytes, malformed schemas, unsupported patterns, and missing schema refs fail closed before side effects.
