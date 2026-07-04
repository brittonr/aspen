# Tasks: contract-export-drift-gates

- [ ] [serial] r[molten.evidence.contract_export_drift.source_export_rust_alignment] Add deterministic checks that regenerate contract exports from Nickel source and compare them with checked-in JSON or Preserves artifacts.
- [ ] [serial] r[molten.evidence.contract_export_drift.source_export_rust_alignment] Verify Preserves boundary schema labels, schema ids, and arity for affected contract exports against Rust admission expectations.
- [ ] [serial] r[molten.evidence.contract_export_drift.source_export_rust_alignment] Ensure Rust admission accepts valid checked-in exports and rejects negative exports or malformed generated artifacts.
- [ ] [parallel] r[molten.evidence.contract_export_drift.local_deterministic_gate] Wire the drift gate into the smallest deterministic local CI/release-review check without network, credentials, or mutable runtime state.
- [ ] [serial] r[molten.evidence.contract_export_drift.source_export_rust_alignment] Run the drift gate and `cairn validate --root .`, or record the blocker and next best check.
