
    fn gc_apply_refs(
        root: &std::path::Path,
        manifest_refs: &[String],
        chunk_refs: &[String],
        evidence: &crate::retention::DestructiveEvidence,
    ) -> Vec<String> {
        let mut apply_refs = Vec::with_capacity(manifest_refs.len() + chunk_refs.len());
        for manifest_ref in manifest_refs {
            let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
                root,
                subsystem: "chunk-gc",
                object_ref: manifest_ref,
                object_kind: "chunk-manifest",
                retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT,
                action: crate::retention::ACTION_DELETE,
                evidence,
            })
            .expect("store manifest GC plan");
            apply_refs.push(
                crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
                    root,
                    plan_ref: &plan.plan_ref,
                })
                .expect("apply manifest GC plan")
                .apply_ref,
            );
        }
        for chunk_ref in chunk_refs {
            let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
                root,
                subsystem: "chunk-gc",
                object_ref: chunk_ref,
                object_kind: "chunk",
                retention_class: crate::retention::CLASS_DURABLE_VALUE,
                action: crate::retention::ACTION_DELETE,
                evidence,
            })
            .expect("store chunk GC plan");
            apply_refs.push(
                crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
                    root,
                    plan_ref: &plan.plan_ref,
                })
                .expect("apply chunk GC plan")
                .apply_ref,
            );
        }
        apply_refs
    }

    fn retention_evidence(root: &std::path::Path, label: &str) -> crate::retention::DestructiveEvidence {
        let requester_ref = chunk_test_ref("requester", label);
        let mut policy_refs = Vec::new();
        let mut authority_refs = Vec::new();
        let mut evidence_refs = Vec::new();
        let mut reference_index_refs = Vec::new();
        for manifest_ref in list_manifest_refs(root).expect("list manifests for retention evidence") {
            push_admissions(
                root,
                label,
                &requester_ref,
                &manifest_ref,
                "chunk-manifest",
                crate::retention::CLASS_PUBLIC_ARTIFACT,
                &mut policy_refs,
                &mut authority_refs,
                &mut evidence_refs,
                &mut reference_index_refs,
            );
        }
        for chunk_ref in list_chunk_refs(root).expect("list chunks for retention evidence") {
            push_admissions(
                root,
                label,
                &requester_ref,
                &chunk_ref,
                "chunk",
                crate::retention::CLASS_DURABLE_VALUE,
                &mut policy_refs,
                &mut authority_refs,
                &mut evidence_refs,
                &mut reference_index_refs,
            );
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

    fn push_admissions(
        root: &std::path::Path,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        policy_refs: &mut Vec<String>,
        authority_refs: &mut Vec<String>,
        evidence_refs: &mut Vec<String>,
        reference_index_refs: &mut Vec<String>,
    ) {
        policy_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_POLICY,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        authority_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_AUTHORITY,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        evidence_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
        reference_index_refs.push(store_admission(
            root,
            crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
            label,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            &[],
            true,
        ));
    }

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
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
            action: crate::retention::ACTION_DELETE,
            bound_refs: &[chunk_test_ref(kind, label)],
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

    fn chunk_test_ref(kind: &str, label: &str) -> String {
        canonical_hash(&record("chunk-test-ref", vec![string(kind), string(label)])).expect("chunk test ref")
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
