## Why

Manual traceability coverage strings are useful bootstrap data, but they are easy to mistype and do not prove that the named command and artifact came from the same run. Receipt-driven coverage derives traceability entries from canonical proof receipts so release review follows evidence rather than claims.

## What Changes

- Add a receipt-driven traceability source model.
- Derive requirement id, coverage kind, target, command identity, and artifact refs from verification/proof receipts.
- Keep raw coverage strings as compatibility-only evidence with explicit labeling.
- Use Hegel RS properties to show derived coverage is deterministic and stale receipts cannot satisfy current requirements.

## Impact

- **Files**: traceability CLI/core, coverage parser, docs, tests.
- **Testing**: Hegel RS generated receipt sets plus positive and negative derivation fixtures.
