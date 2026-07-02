
    #[test]
    fn gc_rejects_drift_after_apply_before_removal() {
        let root = temp_dir("ledger-execution-drift");
        let artifact = crate::preserves_rail::parse_text("<example \"drift\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        let retention_class = retention_class(&imported.artifact_kind);
        let retention_evidence = retention_evidence(
            &root,
            "drift",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            crate::retention::ACTION_DELETE,
        );
        let apply_refs = vec![apply_ref_for(
            &root,
            "ledger-gc",
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            &retention_evidence,
        )];
        crate::retention::pin_object(&root, crate::retention::PinInput {
            object_ref: imported.artifact_ref.clone(),
            object_kind: imported.artifact_kind.clone(),
            retention_class: retention_class.to_string(),
            source: crate::retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "post-apply drift".to_string(),
            owner_ref: test_ref("owner", "drift"),
            expiry_ref: None,
            policy_refs: vec![test_ref("pin-policy", "drift")],
            evidence_refs: vec![test_ref("pin-evidence", "drift")],
            has_authority: true,
        })
        .expect("post-apply retention pin");
        let gc = gc(&root, GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc denied after drift");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_refs.is_empty());
        assert_eq!(read_artifact(&root, &imported.artifact_ref).expect("read artifact"), artifact);
        assert!(!gc.execution_gate_refs.is_empty());
    }

    #[test]
    fn detects_corrupted_content_bytes() {
        let root = temp_dir("ledger-corrupt");
        let artifact = crate::preserves_rail::parse_text("<example \"ok\">").expect("parse artifact");
        let imported = import_artifact(&root, &artifact).expect("import artifact");
        std::fs::write(content_path(&root, &imported.artifact_ref).expect("content path"), b"not preserves")
            .expect("corrupt artifact");
        let error = read_artifact(&root, &imported.artifact_ref).expect_err("corruption fails");
        assert!(["Preserves", "hash mismatch"].iter().any(|needle| error.to_string().contains(needle)));
    }

    struct PeerCase {
        peer: String,
        remote: String,
    }

    fn add_peer_gate(
        root: &std::path::Path,
        label: &str,
        imported: &Import,
        retention_class: &str,
        evidence: &mut crate::retention::DestructiveEvidence,
    ) -> PeerCase {
        let peer = PeerCase {
            peer: test_ref("remote-peer", label),
            remote: test_ref("remote-cache", label),
        };
        evidence.remote_peer_refs = vec![peer.peer.clone()];
        evidence.remote_refs = vec![peer.remote.clone()];
        evidence.remote_gc_refs = vec![store_admission(
            root,
            crate::retention::ADMISSION_KIND_REMOTE_GC,
            label,
            evidence.requester_ref.as_deref().expect("requester"),
            &imported.artifact_ref,
            &imported.artifact_kind,
            retention_class,
            crate::retention::ACTION_DELETE,
            &evidence.remote_refs,
            true,
        )];
        peer
    }

    fn store_peer_pass(
        root: &std::path::Path,
        imported: &Import,
        retention_class: &str,
        evidence: &crate::retention::DestructiveEvidence,
        peer: &PeerCase,
    ) -> String {
        crate::retention::store_remote_gc_clearance(root, &crate::retention::RemoteGcClearanceInput {
            decision: "pass",
            requester_ref: evidence.requester_ref.as_deref().expect("requester"),
            peer_ref: &peer.peer,
            object_ref: &imported.artifact_ref,
            object_kind: &imported.artifact_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            remote_ref: &peer.remote,
            policy_ref: &evidence.policy_refs[0],
            authority_ref: &evidence.authority_refs[0],
            evidence_refs: &evidence.evidence_refs,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store remote clearance")
        .clearance_ref
    }

    fn apply_ref_for(
        root: &std::path::Path,
        subsystem: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        evidence: &crate::retention::DestructiveEvidence,
    ) -> String {
        let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
            root,
            subsystem,
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            evidence,
        })
        .expect("store ledger GC plan");
        crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply ledger GC plan")
        .apply_ref
    }

    fn retention_evidence(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> crate::retention::DestructiveEvidence {
        let requester_ref = test_ref("requester", label);
        let policy_refs = vec![store_admission(
            root,
            crate::retention::ADMISSION_KIND_POLICY,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let authority_refs = vec![store_admission(
            root,
            crate::retention::ADMISSION_KIND_AUTHORITY,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let evidence_refs = vec![store_admission(
            root,
            crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        let reference_index_refs = vec![store_admission(
            root,
            crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
            label,
            &requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            &[],
            true,
        )];
        crate::retention::DestructiveEvidence {
            requester_ref: Some(requester_ref),
            policy_refs,
            authority_refs,
            evidence_refs,
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs,
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn retention_evidence_without_authority(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> crate::retention::DestructiveEvidence {
        let mut evidence = retention_evidence(root, label, object_ref, object_kind, retention_class, action);
        evidence.authority_refs.clear();
        evidence
    }

    fn retention_evidence_without_policy_evidence(
        root: &std::path::Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
    ) -> crate::retention::DestructiveEvidence {
        let mut evidence = retention_evidence(root, label, object_ref, object_kind, retention_class, action);
        evidence.policy_refs.clear();
        evidence.evidence_refs.clear();
        evidence
    }

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        action: &str,
        remote_refs: &[String],
        is_reference_index_complete: bool,
    ) -> String {
        crate::retention::store_evidence_admission(root, &crate::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            bound_refs: &[test_ref(kind, label)],
            retained_refs: &[],
            remote_refs,
            is_reference_index_complete,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retention admission")
        .admission_ref
    }

    fn test_ref(kind: &str, label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("ledger-test-ref", vec![
            crate::preserves_rail::string(kind),
            crate::preserves_rail::string(label),
        ]))
        .expect("ledger test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
