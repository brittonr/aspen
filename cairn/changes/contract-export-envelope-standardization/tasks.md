# Tasks: contract-export-envelope-standardization

- [ ] [serial] r[molten.evidence.contract_exports.metadata_envelope] Define the standard Nickel export envelope fields and map existing contract export surfaces to the envelope.
- [ ] [serial] r[molten.evidence.contract_exports.metadata_envelope] Add or adapt plugin extension contract and grant authoring exports so reviewers can inspect schema id, schema version, source language, stable identity, and payload.
- [ ] [parallel] r[molten.evidence.contract_exports.evidence_only_metadata] Document that export metadata identifies evidence shape only and grants no runtime authority.
- [ ] [parallel] r[molten.evidence.contract_exports.metadata_envelope] Add positive metadata-envelope fixtures and negative fixtures for missing, stale, or unsupported metadata.
- [ ] [serial] r[molten.evidence.contract_exports.metadata_envelope] Run focused export/regeneration checks, Rust admission tests for affected Preserves artifacts, and `cairn validate --root .`, or record the blocker and next best check.
