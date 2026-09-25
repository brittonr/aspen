    use super::*;

    #[test]
    fn artifact_identity_is_stable_across_names_and_changes_with_payload_kind_or_deps() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("artifact-identity");
        let root = CapabilityArtifactRoot::open(&root_path).expect("open artifact capability root");
        let payload = record("module", vec![string("v1")]);
        let input = ArtifactInstallInput {
            kind: "steel".to_string(),
            payload: payload.clone(),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        };
        let first = install_artifact_with_root(&root, &input).expect("install first");
        let duplicate = install_artifact_with_root(&root, &input).expect("install duplicate");
        let identity_receipt = parse_artifact_identity_receipt(&first.identity_receipt_value).expect("identity receipt");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.identity_receipt_ref, identity_receipt.receipt_ref);
        assert_eq!(identity_receipt.decision, "pass");
        assert_eq!(identity_receipt.artifact_ref.as_deref(), Some(first.artifact_ref.as_str()));
        assert_eq!(first.artifact_ref, duplicate.artifact_ref);
        let pointer = set_name_pointer_with_root(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/main",
            artifact_ref: &first.artifact_ref,
            policy_refs: &input.policy_refs,
            evidence_refs: &input.evidence_refs,
        })
        .expect("set name");
        assert_eq!(pointer.artifact_ref, first.artifact_ref);
        assert_eq!(read_payload_with_root(&root, &first.artifact_ref).expect("payload"), payload);

        let changed_payload = install_artifact_with_root(&root, &ArtifactInstallInput {
            payload: record("module", vec![string("v2")]),
            ..input.clone()
        })
        .expect("changed payload");
        assert_ne!(first.artifact_ref, changed_payload.artifact_ref);
        let changed_kind = install_artifact_with_root(&root, &ArtifactInstallInput {
            kind: "wasm".to_string(),
            ..input.clone()
        })
        .expect("changed kind");
        assert_ne!(first.artifact_ref, changed_kind.artifact_ref);
        let changed_deps = install_artifact_with_root(&root, &ArtifactInstallInput {
            dependency_refs: vec![first.artifact_ref.clone()],
            ..input
        })
        .expect("changed deps");
        assert_ne!(first.artifact_ref, changed_deps.artifact_ref);
    }

    #[test]
    fn artifact_identity_receipts_are_stable_and_domain_separated() {
        // r[verify molten.artifacts.canonical_identity_validation]
        let payload_ref = test_ref("canonical-payload");
        let schema_refs = vec![test_ref("schema")];
        let dependency_summary_refs = vec![test_ref("dependency-summary")];
        let policy_refs = vec![test_ref("policy")];
        let provenance_refs = vec![test_ref("provenance")];
        let schema_domain = domain_for_kind("schema");
        let schema_input = identity_input(
            "schema",
            &schema_domain,
            &payload_ref,
            &schema_refs,
            &dependency_summary_refs,
            &policy_refs,
            &provenance_refs,
        );
        let first = artifact_identity_receipt(&schema_input).expect("first identity receipt");
        let repeated = artifact_identity_receipt(&schema_input).expect("repeated identity receipt");
        let policy_domain = domain_for_kind("policy");
        let policy_input = ArtifactIdentityInput {
            kind: "policy",
            identity_domain: &policy_domain,
            ..schema_input
        };
        let different_domain = artifact_identity_receipt(&policy_input).expect("policy identity receipt");

        assert_eq!(first.decision, "pass");
        assert_eq!(first.artifact_ref, repeated.artifact_ref);
        assert_eq!(first.receipt_ref, repeated.receipt_ref);
        assert_ne!(first.artifact_ref, different_domain.artifact_ref);
        assert_eq!(parse_artifact_identity_receipt(&first.value).expect("parse").decision, "pass");
    }

    #[test]
    fn artifact_identity_receipts_deny_noncanonical_or_unsupported_identity() {
        // r[verify molten.artifacts.canonical_identity_validation]
        let payload_ref = test_ref("canonical-payload");
        let schema_refs = vec![test_ref("schema")];
        let dependency_summary_refs = Vec::new();
        let policy_refs = vec![test_ref("policy")];
        let provenance_refs = vec![test_ref("provenance")];
        let steel_domain = domain_for_kind("steel");
        let base = identity_input(
            "steel",
            &steel_domain,
            &payload_ref,
            &schema_refs,
            &dependency_summary_refs,
            &policy_refs,
            &provenance_refs,
        );
        let missing_payload = artifact_identity_receipt(&ArtifactIdentityInput {
            canonical_payload_ref: "",
            ..base
        })
        .expect("missing payload receipt");
        let wrong_domain = artifact_identity_receipt(&ArtifactIdentityInput {
            identity_domain: "molten.artifacts.domain.v1:policy",
            ..base
        })
        .expect("wrong domain receipt");
        let raw_source = artifact_identity_receipt(&ArtifactIdentityInput {
            canonicalizer: RAW_SOURCE_CANONICALIZER,
            ..base
        })
        .expect("raw source receipt");
        let unsupported_hash = artifact_identity_receipt(&ArtifactIdentityInput {
            hash_algorithm: "sha256",
            ..base
        })
        .expect("unsupported hash receipt");
        let unknown_domain = domain_for_kind("unknown-kind");
        let unsupported_kind = artifact_identity_receipt(&ArtifactIdentityInput {
            kind: "unknown-kind",
            identity_domain: &unknown_domain,
            ..base
        })
        .expect("unsupported kind receipt");

        assert_eq!(missing_payload.decision, "deny");
        assert_eq!(wrong_domain.decision, "deny");
        assert_eq!(raw_source.decision, "deny");
        assert_eq!(unsupported_hash.decision, "deny");
        assert_eq!(unsupported_kind.decision, "deny");
        assert!(missing_payload.artifact_ref.is_none());
        assert!(unsupported_hash
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires blake3")));
    }

    #[test]
    fn artifact_registry_rejects_malformed_refs_and_missing_materialization() {
        let root = temp_dir("artifact-ref-shape");
        let mut input = test_input("steel", "bad-ref", &[]);
        input.schema_refs = vec!["blake3:fixture".to_string()];
        let error = install_artifact(&root, &input).expect_err("short schema ref denied");
        assert!(error.to_string().contains("canonical blake3 content ref"));

        let content_payload = ArtifactPayloadRef::ContentRef {
            manifest_ref: "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            length: 128,
        };
        let artifact_error = artifact_value(ArtifactValueInput {
            kind: "doc",
            payload: &content_payload,
            schema_refs: &[test_ref("schema")],
            dependency_refs: &[],
            effect_manifest_ref: None,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect_err("uppercase content manifest ref denied");
        assert!(artifact_error.to_string().contains("canonical blake3 content ref"));

        let missing = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let missing_error = read_artifact(&root, missing).expect_err("valid-shaped missing artifact denied");
        assert!(missing_error.to_string().contains("not found"));
    }

    #[test]
    fn artifact_registry_detects_tampered_materialized_artifact_bytes() {
        let root = temp_dir("artifact-tampered-bytes");
        let first = install_artifact(&root, &test_input("steel", "first", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second", &[])).expect("second artifact");
        assert_ne!(first.artifact_ref, second.artifact_ref);
        let capability_root = CapabilityArtifactRoot::open(&root).expect("open artifact capability root");
        let db = ensure_index_tables(&capability_root).expect("artifact db");
        let write_txn = db.begin_write().expect("write txn");
        {
            let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).expect("artifacts table");
            let second_bytes = canonical_bytes(&second.artifact.value).expect("second bytes");
            artifacts
                .insert(first.artifact_ref.as_str(), second_bytes.as_slice())
                .expect("tamper artifact bytes");
        }
        write_txn.commit().expect("commit tamper");
        drop(db);
        let error = read_artifact(&root, &first.artifact_ref).expect_err("tampered artifact bytes denied");
        assert!(error.to_string().contains("artifact registry content hash mismatch"), "unexpected error: {error}");
    }

    #[test]
    fn artifact_names_do_not_substitute_for_content_identity() {
        let root = temp_dir("artifact-name-not-identity");
        let first = install_artifact(&root, &test_input("steel", "first-name", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second-name", &[])).expect("second artifact");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &first.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("first name pointer");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &second.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("second name pointer");
        assert_eq!(
            read_payload(&root, &first.artifact_ref).expect("first payload"),
            record("payload", vec![string("first-name")])
        );
        assert_eq!(
            read_payload(&root, &second.artifact_ref).expect("second payload"),
            record("payload", vec![string("second-name")])
        );
        assert_ne!(first.artifact_ref, second.artifact_ref);
    }
