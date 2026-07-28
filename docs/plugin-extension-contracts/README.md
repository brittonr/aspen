# Plugin extension contracts

`contract.ncl` is the typed Nickel contract for human-maintained plugin extension definitions.

`storage.ncl` is the positive fixture. Each `storage-invalid-*` fixture tests one field or cross-field invariant before runtime admission.

`grant.ncl` is the typed Nickel contract for `plugin-capability-grant-v1` fixtures.

`storage.grant.ncl` and `storage-revoked.grant.ncl` are valid grants. Negative `storage-*.grant.ncl` fixtures cover invalid proof, delegation, validity, revocation, and evidence fields.

`envelope.ncl` defines the export metadata envelope for contract and grant review evidence.

The positive envelope exports are checked into `generated/` as drift-gated JSON. `storage-envelope-*.ncl` fixtures reject missing, stale, or unsupported metadata.

The runtime consumes checked-in canonical Preserves evidence. It does not execute Nickel or treat envelope metadata as authority.

Regenerate and check exports in the development shell. Review the generated JSON and Preserves exports before Rust validation consumes them.
