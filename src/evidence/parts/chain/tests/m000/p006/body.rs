
    fn delete_evidence_for(
        root: &Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
    ) -> crate::retention::DestructiveEvidence {
        let requester_ref = ref_for(&format!("{label}-requester"));
        crate::retention::DestructiveEvidence {
            requester_ref: Some(requester_ref.clone()),
            policy_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_POLICY,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            authority_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_AUTHORITY,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            evidence_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn store_delete_admission(
        root: &Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
    ) -> String {
        let bound_refs = vec![object_ref.to_string()];
        let diagnostics = vec![format!("{label}-{kind}")];
        crate::retention::store_evidence_admission(root, &crate::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            bound_refs: &bound_refs,
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &diagnostics,
        })
        .expect("store delete evidence admission")
        .admission_ref
    }

    fn delete_apply_ref_for(
        root: &Path,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        evidence: &crate::retention::DestructiveEvidence,
    ) -> String {
        let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
            root,
            subsystem: "ledger-gc",
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            evidence,
        })
        .expect("store unanchored GC plan");
        crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply unanchored GC plan")
        .apply_ref
    }
