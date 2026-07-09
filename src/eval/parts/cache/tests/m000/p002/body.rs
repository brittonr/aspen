
    fn rkyv_manifest_input(label: &str) -> (RkyvDerivedArchiveManifestInput, Vec<RkyvSourceDigest>) {
        let source_value = record("rkyv-source", vec![string(label)]);
        let source_ref = canonical_hash(&source_value).expect("source ref");
        let source = rkyv_source_digest(&source_ref, &source_value).expect("source digest");
        let validation_receipt_ref = test_ref(&format!("rkyv-validation-{label}"));
        let input = RkyvDerivedArchiveManifestInput {
            cache_purpose: RKYV_PURPOSE_REPLAY_INDEX.to_string(),
            artifact_kind: "eval-cache-index".to_string(),
            profile_version: RKYV_CURRENT_PROFILE.to_string(),
            producer_tool_ref: test_ref(&format!("rkyv-producer-{label}")),
            producer_version: "tool-v1".to_string(),
            source_digests: vec![source.clone()],
            archive_byte_digest: test_ref(&format!("rkyv-archive-{label}")),
            validation_required: true,
            validation_receipt_ref: Some(validation_receipt_ref),
            rebuild_capability: Some("rebuild-from-preserves".to_string()),
            retention_class: RKYV_RETENTION_EPHEMERAL_CACHE.to_string(),
            identity_claim: RKYV_IDENTITY_DERIVED_SIDECAR.to_string(),
        };
        (input, vec![source])
    }

    #[test]
    fn rkyv_manifest_admits_current_validated_sidecar() {
        let (input, current_sources) = rkyv_manifest_input("admit");
        let manifest = rkyv_derived_archive_manifest(&input).expect("manifest");
        let admission = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &manifest,
            current_sources: &current_sources,
            observed_archive_digest: &input.archive_byte_digest,
            observed_validation_receipt_ref: input.validation_receipt_ref.as_deref(),
            validation_passed: true,
            caller_allows_rebuild: true,
        })
        .expect("admission");
        assert_eq!(admission.decision, RKYV_DECISION_ADMIT);
        let text = crate::preserves_rail::to_text(&admission.value).expect("admission text");
        assert!(text.contains("archive-bytes-not-identity"));
    }

    #[test]
    fn rkyv_manifest_requests_rebuild_for_stale_or_tampered_archive() {
        let (input, mut current_sources) = rkyv_manifest_input("rebuild");
        let manifest = rkyv_derived_archive_manifest(&input).expect("manifest");
        current_sources[0].blake3_digest = test_ref("rkyv-new-source-digest");
        let stale = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &manifest,
            current_sources: &current_sources,
            observed_archive_digest: &input.archive_byte_digest,
            observed_validation_receipt_ref: input.validation_receipt_ref.as_deref(),
            validation_passed: true,
            caller_allows_rebuild: true,
        })
        .expect("stale admission");
        assert_eq!(stale.decision, RKYV_DECISION_REBUILD);
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));

        let (_, current_sources) = rkyv_manifest_input("tamper");
        let mut tamper_input = input.clone();
        tamper_input.source_digests = current_sources.clone();
        let tamper_manifest = rkyv_derived_archive_manifest(&tamper_input).expect("tamper manifest");
        let tampered = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &tamper_manifest,
            current_sources: &current_sources,
            observed_archive_digest: &test_ref("tampered-rkyv-archive"),
            observed_validation_receipt_ref: tamper_input.validation_receipt_ref.as_deref(),
            validation_passed: true,
            caller_allows_rebuild: true,
        })
        .expect("tampered admission");
        assert_eq!(tampered.decision, RKYV_DECISION_REBUILD);
        assert!(tampered.diagnostics.iter().any(|diagnostic| diagnostic.contains("byte digest")));
    }

    #[test]
    fn rkyv_manifest_denies_missing_validation_profile_and_identity_overclaims() {
        let (input, current_sources) = rkyv_manifest_input("deny");
        let manifest = rkyv_derived_archive_manifest(&input).expect("manifest");
        let missing_validation = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &manifest,
            current_sources: &current_sources,
            observed_archive_digest: &input.archive_byte_digest,
            observed_validation_receipt_ref: None,
            validation_passed: false,
            caller_allows_rebuild: true,
        })
        .expect("missing validation admission");
        assert_eq!(missing_validation.decision, RKYV_DECISION_DENY);
        assert!(missing_validation.diagnostics.iter().any(|diagnostic| diagnostic.contains("validation")));

        let mut unsupported_input = input.clone();
        unsupported_input.profile_version = "rkyv-derived-cache-v0".to_string();
        let unsupported_manifest = rkyv_derived_archive_manifest(&unsupported_input).expect("unsupported manifest");
        let unsupported = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &unsupported_manifest,
            current_sources: &current_sources,
            observed_archive_digest: &unsupported_input.archive_byte_digest,
            observed_validation_receipt_ref: unsupported_input.validation_receipt_ref.as_deref(),
            validation_passed: true,
            caller_allows_rebuild: true,
        })
        .expect("unsupported profile admission");
        assert_eq!(unsupported.decision, RKYV_DECISION_DENY);
        assert!(unsupported.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported")));

        let mut overclaim_input = input.clone();
        overclaim_input.identity_claim = "canonical-cache-key".to_string();
        let overclaim_manifest = rkyv_derived_archive_manifest(&overclaim_input).expect("overclaim manifest");
        let overclaim = admit_rkyv_derived_archive(RkyvArchiveAdmissionInput {
            manifest: &overclaim_manifest,
            current_sources: &current_sources,
            observed_archive_digest: &overclaim_input.archive_byte_digest,
            observed_validation_receipt_ref: overclaim_input.validation_receipt_ref.as_deref(),
            validation_passed: true,
            caller_allows_rebuild: true,
        })
        .expect("overclaim admission");
        assert_eq!(overclaim.decision, RKYV_DECISION_DENY);
        assert!(overclaim.diagnostics.iter().any(|diagnostic| diagnostic.contains("canonical identity")));

        let mut bare_archive = input.clone();
        bare_archive.source_digests = Vec::new();
        assert!(rkyv_derived_archive_manifest(&bare_archive).is_err());
    }
