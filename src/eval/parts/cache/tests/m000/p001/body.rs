
    fn value_input(
        tier: &str,
        status: &str,
        output: Option<IoValue>,
        key: &KeyInput,
        evidence_refs: &[String],
    ) -> ValueInput {
        ValueInput {
            tier: tier.to_string(),
            status: status.to_string(),
            output,
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: key.policy_refs.clone(),
            evidence_refs: evidence_refs.to_vec(),
            diagnostics: Vec::new(),
        }
    }

    fn apply_ref(root: &std::path::Path, key_ref: &str, evidence: &crate::retention::DestructiveEvidence) -> String {
        let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
            root,
            subsystem: "eval-cache-invalidate",
            object_ref: key_ref,
            object_kind: "eval-cache-key",
            retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
            action: crate::retention::ACTION_TOMBSTONE,
            evidence,
        })
        .expect("store cache invalidation plan");
        crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply cache invalidation plan")
        .apply_ref
    }

    fn retention_evidence(root: &std::path::Path, label: &str) -> crate::retention::DestructiveEvidence {
        let requester_ref = test_ref(&format!("retention-requester-{label}"));
        let summaries = list(root, &ListFilter::default()).expect("list cache for retention evidence");
        let mut policy_refs = Vec::with_capacity(summaries.len());
        let mut authority_refs = Vec::with_capacity(summaries.len());
        let mut evidence_refs = Vec::with_capacity(summaries.len());
        let mut reference_index_refs = Vec::with_capacity(summaries.len());
        for summary in summaries {
            policy_refs.push(store_admission(
                root,
                crate::retention::ADMISSION_KIND_POLICY,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            authority_refs.push(store_admission(
                root,
                crate::retention::ADMISSION_KIND_AUTHORITY,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            evidence_refs.push(store_admission(
                root,
                crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            reference_index_refs.push(store_admission(
                root,
                crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
        }
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

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        key_ref: &str,
        remote_refs: &[String],
        is_reference_index_complete: bool,
    ) -> String {
        crate::retention::store_evidence_admission(root, &crate::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref: key_ref,
            object_kind: "eval-cache-key",
            retention_class: crate::retention::CLASS_EPHEMERAL_CACHE,
            action: crate::retention::ACTION_TOMBSTONE,
            bound_refs: &[test_ref(&format!("{kind}-{label}"))],
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

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("eval-cache-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
