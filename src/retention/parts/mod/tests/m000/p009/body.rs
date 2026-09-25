
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
