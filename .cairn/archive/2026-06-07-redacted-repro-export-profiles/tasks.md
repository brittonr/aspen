# Tasks: redacted-repro-export-profiles

- [x] [serial] r[molten.testing.redacted_repro_export_profiles.profile_schema] Define export profile schema and CLI selection for sealed repro export.
- [x] [serial] r[molten.testing.redacted_repro_export_profiles.transform_receipt] Add canonical redaction transform receipts bound to source report, output bundle, policy, and profile.
- [x] [serial] r[molten.testing.redacted_repro_export_profiles.transform_receipt] Ensure redacted diagnostic bundles remain diagnostic-only unless policy explicitly marks the transform gate-preserving.
- [x] [serial] r[molten.testing.redacted_repro_export_profiles.encrypted_ref_validation] Validate encrypted refs before accepting them in sealed artifacts.
- [x] [parallel] r[molten.testing.redacted_repro_export_profiles.encrypted_ref_validation] Add reveal receipt format and fail-closed unpack behavior for private encrypted material.
- [x] [parallel] r[molten.testing.redacted_repro_export_profiles.profile_schema] Add negative tests for missed markers, stale transform receipts, malformed encrypted refs, and unauthorized reveal attempts.
