## Why

Molten's plugin host boundary already records artifact-backed manifests, lifecycle receipts, hostcall receipts, and deny-by-default behavior. The next contract hardening step is to make those receipts impossible to mis-bind or forge coherently: hostcall operations must match their canonical refs, every receipt must carry the manifest identity it claims to authorize, and parsers must reject pass receipts whose checks say a required gate failed.

These are the immediate safety fixes before richer extension contracts land. They close gaps where a receipt could be syntactically parseable but semantically stale, mislabeled, or internally inconsistent.

## What Changes

- Require plugin hostcall receipts to verify operation name and hostcall ref consistency before passing.
- Require install, permission, lifecycle, hostcall, health, removal, upgrade, and future compatibility receipts to parse and expose manifest identity used for lifecycle validation.
- Require receipt parsers and lifecycle state evaluation to reject stale manifest bindings.
- Require receipt decision/check coherence: `pass` receipts cannot carry failed required checks, and denied receipts must expose the failed gate evidence.
- Add positive and negative tests for valid hostcall binding, operation/ref mismatch, stale manifest receipt reuse, and forged pass receipts.

## Impact

- **Specs**: `plugin-host` gains stricter receipt coherence and binding rules.
- **Core**: plugin host receipt constructors/parsers and lifecycle state evaluator become stricter pure validation boundaries.
- **Tests**: add happy-path hostcall receipt coverage plus negative fixtures for mismatches and incoherent receipts.
