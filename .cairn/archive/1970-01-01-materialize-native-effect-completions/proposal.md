## Why

The native host delivers a canonical effect-completion receipt to the callback. That receipt carries only the provider output reference.

A hosted application cannot derive provider terminal meaning from a BLAKE3 reference alone. It needs the exact bounded provider output bytes.

The current shape forces honest consumers to classify every hosted effect completion as `Unknown`. It blocks a useful Kiln-on-Aspen canary.

## What Changes

- Add an optional exact materialized output value to the generic fabric effect result.
- Bind that value into a version-two canonical effect-completion record.
- Require the value for native profiles that require materialized callback values.
- Reject missing, oversized, identity-mismatched, or reference-mismatched provider output before callback delivery.
- Preserve reference-only generic effect routing for profiles that do not require materialized values.
- Add positive and negative separate-process tests and update the native-host documentation.

## Impact

- **Contract:** `molten.system-extension.effect-completion.v2` replaces the native callback completion record used by materializing hosts.
- **Compatibility:** Native consumers must decode the exact version-two record. There is no legacy fallback.
- **Risk:** Provider effects may already have completed when their output value fails admission. The host must not retry them automatically.
- **Non-goals:** This change does not interpret provider output, add a workload branch, provide durable value storage, or claim provider success.
