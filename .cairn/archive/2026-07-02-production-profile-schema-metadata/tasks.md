# Tasks: production-profile-schema-metadata

- [x] [serial] r[molten.prod_ops.profile_schema_metadata.root_identity] Add schema id, schema version, source language, and stable profile identity metadata to production profile exports.
- [x] [serial] r[molten.prod_ops.profile_schema_metadata.receipt_binding] Bind profile metadata into deployment-profile and startup evidence validation.
- [x] [serial] r[molten.prod_ops.profile_schema_metadata.evidence_only] Document that profile metadata identifies evidence shape but grants no authority or subsystem trust.
- [x] [parallel] r[molten.prod_ops.profile_schema_metadata.root_identity] Add negative coverage for missing, unsupported, or mismatched metadata.
- [x] [parallel] r[molten.prod_ops.profile_schema_metadata.receipt_binding] Add receipt validation coverage for stale or tampered metadata bindings.
