## Why

Preserves boundary schema refs currently identify the boundary family, version, record label, schema id, and arity, but not the field labels, field kinds, or constraint set. A contract can therefore change while preserving the same arity and schema ref, which weakens review and evidence identity for high-risk boundary records.

## What Changes

- Extend boundary schema artifact identity to include every field label, field kind, and declared constraint descriptor.
- Treat field order, field kind changes, required/non-empty/unique/bounded constraints, and domain vocabulary changes as schema-ref-affecting contract changes.
- Emit diagnostics that name stale or mismatched schema contracts before semantic admission.
- Preserve existing canonical record layouts unless an explicit later migration changes the wire shape.

## Impact

- **Files**: `preserves_rail` boundary schema identity, schema validation tests, and call sites that bind schema refs in diagnostics or receipts.
- **Testing**: positive tests prove stable refs for unchanged contracts; negative tests prove same-arity field kind or constraint drift changes refs and fails stale-evidence checks.
