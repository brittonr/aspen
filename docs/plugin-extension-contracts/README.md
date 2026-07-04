# Plugin extension contracts

`contract.ncl` is the typed Nickel authoring contract for human-maintained plugin extension definitions. `storage.ncl` is the positive fixture; negative fixtures named `storage-invalid-*`, `storage-malformed-ref.ncl`, `storage-empty-evidence.ncl`, `storage-duplicate-descriptor.ncl`, and `storage-missing-schema.ncl` each target one field-domain or cross-field invariant before runtime admission.

`grant.ncl` is the typed Nickel authoring contract for `plugin-capability-grant-v1` fixtures. `storage.grant.ncl` and `storage-revoked.grant.ncl` are valid grants; negative fixtures named `storage-*.grant.ncl` cover missing proofs, malformed refs, over-delegation, inverted validity windows, missing revocation evidence, and empty evidence refs.

`envelope.ncl` defines the standard export metadata envelope for plugin contract and grant review evidence. `storage.contract-envelope.ncl` and `storage.grant-envelope.ncl` are the positive envelope exports, checked into `generated/` as drift-gated JSON. `storage-envelope-*.ncl` fixtures reject missing, stale, or unsupported metadata.

The runtime consumes checked-in canonical Preserves evidence (`*.contract.preserves` and `*.grant.preserves`) and does not execute Nickel or envelope metadata as authority. Regenerate/check exports in the dev shell, then keep the generated JSON and Preserves exports under review before Rust validation consumes them.
