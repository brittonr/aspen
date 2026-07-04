# Plugin extension contracts

`contract.ncl` is the typed Nickel authoring contract for human-maintained plugin extension definitions. `storage.ncl` is the positive fixture; `storage-missing-schema.ncl` is a negative fixture that must fail Nickel validation before runtime admission.

`grant.ncl` is the typed Nickel authoring contract for `plugin-capability-grant-v1` fixtures. `storage.grant.ncl` is a valid storage-read grant; `storage-missing-proof.grant.ncl` is a negative fixture whose checked-in Preserves export is rejected by Rust validation because typed grants must bind proof evidence.

The runtime consumes checked-in canonical Preserves evidence (`*.contract.preserves` and `*.grant.preserves`) and does not execute Nickel as authority. Regenerate/check exports in the dev shell, then keep the Preserves export under review before Rust validation consumes it.
