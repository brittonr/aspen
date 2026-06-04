# Tasks: redacted-repro-export-profiles

- [ ] [serial] r[molten.testing.redacted_repro_export_profiles.profile_schema] Define export profile schema and CLI selection for sealed repro export.
- [ ] [serial] r[molten.testing.redacted_repro_export_profiles.transform_receipt] Add canonical redaction transform receipts bound to source report, output bundle, policy, and profile.
- [ ] [serial] r[molten.testing.redacted_repro_export_profiles.diagnostic_only] Ensure redacted diagnostic bundles remain diagnostic-only unless policy explicitly marks the transform gate-preserving.
- [ ] [serial] r[molten.testing.redacted_repro_export_profiles.encrypted_ref_validation] Validate encrypted refs before accepting them in sealed artifacts.
- [ ] [parallel] r[molten.testing.redacted_repro_export_profiles.reveal_receipts] Add reveal receipt format and fail-closed unpack behavior for private encrypted material.
- [ ] [parallel] r[molten.testing.redacted_repro_export_profiles.tests] Add negative tests for missed markers, stale transform receipts, malformed encrypted refs, and unauthorized reveal attempts.
