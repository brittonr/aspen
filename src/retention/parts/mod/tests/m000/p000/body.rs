    use n0_future::StreamExt;

    use super::*;
    use std::fs;

    type PathBuf = std::path::PathBuf;

    fn store_dir(root: &Path) -> PathBuf {
        root.join(STORE_DIR)
    }

    fn gc_plans_dir(root: &Path) -> PathBuf {
        store_dir(root).join(GC_PLAN_DIR)
    }

    fn gc_applies_dir(root: &Path) -> PathBuf {
        store_dir(root).join(GC_APPLY_DIR)
    }

    fn gc_executes_dir(root: &Path) -> PathBuf {
        store_dir(root).join(GC_EXECUTE_DIR)
    }

    fn gc_audits_dir(root: &Path) -> PathBuf {
        store_dir(root).join(GC_AUDIT_DIR)
    }

    fn receipts_dir(root: &Path) -> PathBuf {
        store_dir(root).join(RECEIPT_DIR)
    }

    fn tombstones_dir(root: &Path) -> PathBuf {
        store_dir(root).join(TOMBSTONE_DIR)
    }

    fn gc_execute_path(root: &Path, reference: &str) -> Result<PathBuf> {
        Ok(gc_executes_dir(root).join(format!("{}.preserves", ref_file_name(reference)?)))
    }

    fn tombstone_path(root: &Path, reference: &str) -> Result<PathBuf> {
        Ok(tombstones_dir(root).join(format!("{}.preserves", ref_file_name(reference)?)))
    }

    fn write_store_value(path: &Path, value: &IoValue) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(MoltenError::from)?;
        }
        fs::write(path, crate::preserves_rail::to_text(value)?).map_err(MoltenError::from)
    }

    fn read_store_value(path: &Path) -> Result<IoValue> {
        let text = fs::read_to_string(path).map_err(MoltenError::from)?;
        crate::preserves_rail::parse_text(&text)
    }

    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Duration = std::time::Duration;
    type Ipv4Addr = std::net::Ipv4Addr;
    type Ordering = std::sync::atomic::Ordering;

    #[test]
    fn pinned_objects_are_not_delete_eligible_until_unpinned() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = crate::test_support::process_workspace("retention_pinned")
            .expect("create isolated retention workspace");
        let root = CapabilityRetentionRoot::open(&root_path).expect("open retention capability root");
        let object_ref = fake_ref("object");
        let owner_ref = fake_ref("owner");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let pin = pin_object_with_root(&root, PinInput {
            object_ref: object_ref.clone(),
            object_kind: "artifact".to_string(),
            retention_class: CLASS_PUBLIC_ARTIFACT.to_string(),
            source: SOURCE_ARTIFACT.to_string(),
            reason: "installed artifact".to_string(),
            owner_ref: owner_ref.clone(),
            expiry_ref: None,
            policy_refs: policy_refs.clone(),
            evidence_refs: evidence_refs.clone(),
            has_authority: true,
        })
        .expect("pin");
        let denied = evaluate_with_root(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            requester_ref: &owner_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("deny delete");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
        let unpin = unpin_object_with_root(UnpinObjectInput {
            root: &root,
            pin_ref: &pin.pin.pin_ref,
            requester_ref: &owner_ref,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_authority: true,
        })
        .expect("unpin");
        assert_eq!(unpin.decision, "pass");
        let allowed = evaluate_with_root(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_TOMBSTONE,
            requester_ref: &owner_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("allow tombstone");
        assert_eq!(allowed.receipt.decision, "pass");
        assert!(allowed.tombstone.is_some());
    }

    #[test]
    fn incomplete_reference_proof_denies_gc() {
        let root = temp_dir("retention-incomplete");
        let object_ref = fake_ref("object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let receipt = evaluate(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_AUDIT_RECEIPT,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: false,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("incomplete deny")
        .receipt;
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "incomplete-reference-proof"));
    }

    #[test]
    fn retained_dependencies_and_legal_holds_deny_deletion() {
        let root = temp_dir("retention-retained");
        let object_ref = fake_ref("object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let retained_refs = vec![fake_ref("receipt-dependency")];
        let retained = evaluate(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_AUDIT_RECEIPT,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &retained_refs,
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("retained deny");
        assert_eq!(retained.receipt.decision, "deny");
        let legal = evaluate(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "receipt",
            retention_class: CLASS_LEGAL_HOLD,
            action: ACTION_DELETE,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &[],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("legal deny");
        assert_eq!(legal.receipt.decision, "deny");
    }

    #[test]
    fn tombstone_summary_preserves_audit_without_secret_content() {
        let root = temp_dir("retention-tombstone");
        let object_ref = fake_ref("secret-object");
        let requester_ref = fake_ref("requester");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("redaction")];
        let evaluation = evaluate(EvaluationInput {
            root: &root,
            object_ref: &object_ref,
            object_kind: "encrypted-ref",
            retention_class: CLASS_PRIVATE_SECRET_REF,
            action: ACTION_REDACT,
            requester_ref: &requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &[],
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            has_delete_authority: true,
            has_remote_gc_clearance: true,
        })
        .expect("redact");
        let tombstone = evaluation.tombstone.expect("tombstone");
        let text = crate::preserves_rail::to_text(&tombstone.value).expect("text");
        assert!(text.contains("redacted-or-deleted"));
        assert!(!text.contains("plaintext"));
        let summary = summary(&tombstone.value).expect("summary");
        assert!(summary.contains("retention tombstone"));
    }

    #[test]
    fn hegel_like_no_dangling_retained_ref_and_deny_on_incomplete_proof() {
        for count in 0..8_u64 {
            let root = temp_dir("retention-hegel-like");
            let object_ref = fake_ref(&format!("object-{count}"));
            let requester_ref = fake_ref("requester");
            let policy_refs = vec![fake_ref("policy")];
            let evidence_refs = vec![fake_ref("evidence")];
            let retained_refs =
                (0..count).map(|index| fake_ref(&format!("retained-{count}-{index}"))).collect::<Vec<_>>();
            let evaluation = evaluate(EvaluationInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: "audit-receipt",
                retention_class: CLASS_AUDIT_RECEIPT,
                action: ACTION_DELETE,
                requester_ref: &requester_ref,
                is_reference_index_complete: count % 2 == 0,
                retained_refs: &retained_refs,
                remote_refs: &[],
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_delete_authority: true,
                has_remote_gc_clearance: true,
            })
            .expect("evaluate");
            if count == 0 {
                assert_eq!(evaluation.receipt.decision, "pass");
            } else {
                assert_eq!(evaluation.receipt.decision, "deny");
            }
        }
    }

    #[test]
    fn destructive_admission_rejects_forged_and_mismatched_refs() {
        let root = temp_dir("retention-admission-forged");
        let requester_ref = fake_ref("requester");
        let object_ref = fake_ref("object");
        let wrong_object_ref = fake_ref("wrong-object");
        let wrong_policy = store_test_admission(TestAdmissionInput {
            root: &root,
            kind: ADMISSION_KIND_POLICY,
            label: "wrong-policy",
            requester_ref: &requester_ref,
            object_ref: &wrong_object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        });
        let evidence = DestructiveEvidence {
            requester_ref: Some(requester_ref),
            policy_refs: vec![wrong_policy],
            authority_refs: vec![fake_ref("forged-authority")],
            evidence_refs: vec![fake_ref("forged-evidence")],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![fake_ref("forged-index")],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        };
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root: &root,
            evidence: &evidence,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
        })
        .expect("admission denial");
        assert_eq!(admission.decision, "deny");
        assert!(!admission.has_delete_authority);
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("scope-mismatch")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("unreadable")));
    }

    #[test]
    fn destructive_admission_rejects_stale_and_revoked_refs() {
        let root = temp_dir("retention-admission-stale");
        let requester_ref = fake_ref("requester");
        let object_ref = fake_ref("object");
        let stale_authority =
            scoped_ref(&root, ADMISSION_KIND_AUTHORITY, "stale-authority", &requester_ref, &object_ref, false, &[
                fake_ref("revocation"),
            ]);
        let policy = scoped_ref(&root, ADMISSION_KIND_POLICY, "policy", &requester_ref, &object_ref, true, &[]);
        let support =
            scoped_ref(&root, ADMISSION_KIND_SUPPORTING_EVIDENCE, "support", &requester_ref, &object_ref, true, &[]);
        let index = scoped_ref(&root, ADMISSION_KIND_REFERENCE_INDEX, "index", &requester_ref, &object_ref, true, &[]);
        let evidence = DestructiveEvidence {
            requester_ref: Some(requester_ref),
            policy_refs: vec![policy],
            authority_refs: vec![stale_authority],
            evidence_refs: vec![support],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![index],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        };
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root: &root,
            evidence: &evidence,
            object_ref: &object_ref,
            object_kind: "artifact",
            retention_class: CLASS_PUBLIC_ARTIFACT,
            action: ACTION_DELETE,
        })
        .expect("admission denial");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("revoked")));
    }
