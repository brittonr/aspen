## Why

Molten now emits reproducible build records and build verification receipts, but provenance evaluation must not admit a self-asserted `reproducible-verified` trust state unless the caller also supplies matching build verification evidence.

## What Changes

- Bind `reproducible-verified` provenance admission to passing `provenance-build-verify-receipt-v1` evidence.
- Require provenance records that claim reproducible verification to name the expected build record refs.
- Extend `molten test provenance evaluate` to accept explicit build verification receipt files.
- Make node-control install/run provenance gates split provenance records from build verification receipts and deny missing, denied, mismatched, or unbound build verification evidence before side effects.
- Keep build verification receipts evidence-only: they may satisfy provenance evidence, but they do not grant authority, policy, resource, transport, execution, or source-gate trust.

## Impact

Operators can now distinguish reviewed artifacts from reproducibly verified artifacts with fail-closed evidence binding. Hash-only or self-asserted reproducible trust remains denied unless the build verification receipt matches the artifact and the provenance record's expected build record evidence.
