    use super::*;

    #[test]
    fn import_is_immutable_and_gc_preserves_pins() {
        let root = temp_dir("ledger");
        let artifact = crate::preserves_rail::parse_text("<example \"ok\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let duplicate = import_artifact(&root, &artifact).expect("import duplicate");
        assert_eq!(imported.artifact_ref, duplicate.artifact_ref);
        assert_eq!(list_artifacts(&root).expect("list artifacts").len(), 1);
        pin_artifact(&root, &imported.artifact_ref).expect("pin artifact");
        let retention_evidence = crate::retention::DestructiveEvidence::default();
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn rejects_malformed_and_missing_content_refs_before_path_use() {
        let root = temp_dir("ledger-ref-shape");
        ensure_dirs(&root).expect("ledger dirs");
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ] {
            assert!(read_artifact(&root, invalid).is_err(), "invalid read ref accepted: {invalid}");
            assert!(pin_artifact(&root, invalid).is_err(), "invalid pin ref accepted: {invalid}");
            assert!(export_artifact(&root, invalid, &root.join("out.preserves")).is_err());
        }
        let missing = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let error = read_artifact(&root, missing).expect_err("valid-shaped missing ref is not materialized");
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn read_detects_tampered_materialized_bytes() {
        let root = temp_dir("ledger-tampered-bytes");
        let artifact = crate::preserves_rail::parse_text("<example \"original\">").expect("parse original");
        let imported = import_artifact(&root, &artifact).expect("import original");
        let tampered = crate::preserves_rail::parse_text("<example \"tampered\">").expect("parse tampered");
        std::fs::write(
            content_path(&root, &imported.artifact_ref).expect("content path"),
            crate::preserves_rail::canonical_bytes(&tampered).expect("tampered canonical bytes"),
        )
        .expect("tamper ledger bytes");
        let error = read_artifact(&root, &imported.artifact_ref).expect_err("tampered bytes denied");
        assert!(error.to_string().contains("ledger content hash mismatch"));
    }

    #[test]
    fn gc_requires_retention_pass_before_removal() {
        let root = temp_dir("ledger-retention");
        let artifact = crate::preserves_rail::parse_text("<example \"retained\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let owner_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("ledger-test-ref", vec![
            crate::preserves_rail::string("owner"),
        ]))
        .expect("owner ref");
        let policy_refs = vec![
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("ledger-test-ref", vec![
                crate::preserves_rail::string("policy"),
            ]))
            .expect("policy ref"),
        ];
        let evidence_refs = vec![
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("ledger-test-ref", vec![
                crate::preserves_rail::string("evidence"),
            ]))
            .expect("evidence ref"),
        ];
        crate::retention::pin_object(&root, crate::retention::PinInput {
            object_ref: imported.artifact_ref.clone(),
            object_kind: imported.artifact_kind.clone(),
            retention_class: crate::retention::CLASS_AUDIT_RECEIPT.to_string(),
            source: crate::retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold".to_string(),
            owner_ref,
            expiry_ref: None,
            policy_refs,
            evidence_refs,
            has_authority: true,
        })
        .expect("retention pin");
        let retention_evidence = retention_evidence(
            &root,
            "retention-pin",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class(&imported.artifact_kind),
            crate::retention::ACTION_DELETE,
        );
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn gc_denies_missing_retention_authority_evidence() {
        let root = temp_dir("ledger-retention-missing-authority");
        let artifact = crate::preserves_rail::parse_text("<example \"missing-authority\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_evidence = retention_evidence_without_authority(
            &root,
            "missing-authority",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class(&imported.artifact_kind),
            crate::retention::ACTION_DELETE,
        );
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn gc_denies_missing_policy_and_supporting_evidence() {
        let root = temp_dir("ledger-retention-missing-policy-evidence");
        let artifact =
            crate::preserves_rail::parse_text("<example \"missing-policy-evidence\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_evidence = retention_evidence_without_policy_evidence(
            &root,
            "missing-policy-evidence",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class(&imported.artifact_kind),
            crate::retention::ACTION_DELETE,
        );
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc ledger");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
    }

    #[test]
    fn gc_requires_per_remote_clearance_before_removal() {
        let label = "remote-clearance";
        let root = temp_dir("ledger-retention-remote-clearance");
        let artifact = crate::preserves_rail::parse_text("<example \"remote-clearance\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = retention_class(&imported.artifact_kind);
        let mut retention_evidence = retention_evidence(
            &root,
            label,
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            crate::retention::ACTION_DELETE,
        );
        let peer = add_peer_gate(&root, label, &imported, retention_class, &mut retention_evidence);
        let denied = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("remote clearance missing denies");
        assert_eq!(denied.decision, "deny");
        assert!(denied.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        retention_evidence.remote_clearance_refs = vec![store_peer_pass(
            &root,
            &imported,
            retention_class,
            &retention_evidence,
            &peer,
        )];
        let apply_refs = vec![apply_ref_for(
            &root,
            "ledger-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        let passed = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("remote clearance pass removes");
        assert_eq!(passed.decision, "pass");
        assert_eq!(passed.removed_refs, vec![imported.artifact_ref.clone()]);
        assert!(read_artifact(&root, &imported.artifact_ref).is_err());
    }

    #[test]
    fn gc_requires_apply_ref_before_removal() {
        let root = temp_dir("ledger-execution-missing-apply");
        let artifact = crate::preserves_rail::parse_text("<example \"missing-apply\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "missing-apply",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            crate::retention::ACTION_DELETE,
        );
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc denied without apply");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        let gate =
            crate::retention::read_gc_execution_gate(&root, &gc.execution_gate_refs[0]).expect("read execution gate");
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-execute-apply-missing"));
    }

    #[test]
    fn gc_rejects_wrong_scope_apply_ref_before_removal() {
        let root = temp_dir("ledger-execution-wrong-scope");
        let artifact = crate::preserves_rail::parse_text("<example \"wrong-scope\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "wrong-scope",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            crate::retention::ACTION_DELETE,
        );
        let apply_refs = vec![apply_ref_for(
            &root,
            "chunk-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc denied with wrong apply scope");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        let gate =
            crate::retention::read_gc_execution_gate(&root, &gc.execution_gate_refs[0]).expect("read execution gate");
        assert!(
            gate.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-execute-apply-scope-mismatch"),
            "{:?}",
            gate.diagnostics
        );
    }
