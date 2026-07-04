# Plugin extension contracts

`contract.ncl` is the typed Nickel authoring contract for human-maintained plugin extension definitions. `storage.ncl` is the positive fixture; `storage-missing-schema.ncl` is a negative fixture that must fail Nickel validation before runtime admission.

The runtime consumes checked-in canonical Preserves evidence (`*.contract.preserves`) and does not execute Nickel as authority. Regenerate/check exports in the dev shell, then keep the Preserves export under review before Rust validation consumes it.
