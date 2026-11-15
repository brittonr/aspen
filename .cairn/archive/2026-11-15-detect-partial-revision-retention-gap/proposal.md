## Why

Issue #35 demonstrates a returned next-event cursor silently skipping an evicted ordinal when a multi-event revision is partially retained. Consumers must receive an explicit retention gap rather than incomplete committed history.

## What Changes

- Derive the ordinal retention boundary from the oldest retained event while preserving inclusive next-event cursors and the untrimmed start position.
- Retain the exact two-commit regression, boundary advances, pagination, empty-stream and watermark coverage.
- Clarify the existing cursor requirement and architecture description; no transport, identity, bound or authority changes.

## Impact

- **Files**: committed-stream subscription core/tests, committed-change-stream specification, architecture cursor documentation.
- **Testing**: record the exact regression failing before the fix, then run the focused committed-stream tests and required source/lifecycle gates.
- **Owner / consumer**: Molten committed-stream core; resumable batch progress consumes the corrected behavior. The repeatable capability is gap-explicit recovery at event granularity.
