## Why

Plugin extension export envelopes validate metadata but leave `payload` as `Dyn`. That permits metadata to be reviewed independently from the payload contract and weakens the relationship between `schema_id`, `export_identity`, and the actual plugin contract or grant being exported.

## What Changes

- Split generic plugin extension envelopes into typed contract and grant envelopes.
- Couple `schema_id` to the expected payload contract.
- Bind `export_identity` to payload fields such as extension id/version or plugin id/operation.
- Keep the runtime boundary unchanged: Rust consumes checked Preserves evidence and does not execute Nickel as authority.

## Impact

- **Files**: `docs/plugin-extension-contracts/envelope.ncl`, plugin contract/grant fixtures, generated envelope JSON, and drift-gate checks.
- **Testing**: positive typed envelope exports pass; negative fixtures for wrong schema id, wrong payload type, missing identity, and identity/payload mismatch fail.
