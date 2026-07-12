    fn retention_cli_args(label: &str) -> RetentionEvidenceArgs {
        RetentionEvidenceArgs {
            requester_ref: Some(test_ref(&format!("retention-requester-{label}"))),
            policy_refs: vec![test_ref(&format!("retention-policy-{label}"))],
            authority_refs: vec![test_ref(&format!("retention-authority-{label}"))],
            evidence_refs: vec![test_ref(&format!("retention-evidence-{label}"))],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: Vec::new(),
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    #[derive(Clone, Copy)]
    struct RetentionCliObject<'a> {
        root: &'a std::path::Path,
        label: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
    }

    fn retention_cli_args_for_object(input: RetentionCliObject<'_>) -> RetentionEvidenceArgs {
        let requester_ref = test_ref(&format!("retention-requester-{}", input.label));
        let policy_refs = vec![store_cli_admission(
            input,
            molten::retention::ADMISSION_KIND_POLICY,
            &requester_ref,
        )];
        let authority_refs = vec![store_cli_admission(
            input,
            molten::retention::ADMISSION_KIND_AUTHORITY,
            &requester_ref,
        )];
        let evidence_refs = vec![store_cli_admission(
            input,
            molten::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            &requester_ref,
        )];
        let reference_index_refs = vec![store_cli_admission(
            input,
            molten::retention::ADMISSION_KIND_REFERENCE_INDEX,
            &requester_ref,
        )];
        RetentionEvidenceArgs {
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

    fn retention_apply_ref(
        input: RetentionCliObject<'_>,
        subsystem: &str,
        retention_args: &RetentionEvidenceArgs,
    ) -> String {
        let evidence = retention_args.clone().into_retention_evidence();
        let plan = molten::retention::store_gc_plan(molten::retention::GcPlanInput {
            root: input.root,
            subsystem,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            evidence: &evidence,
        })
        .expect("store CLI retention GC plan");
        molten::retention::apply_gc_plan(molten::retention::GcApplyFromPlanInput {
            root: input.root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply CLI retention GC plan")
        .apply_ref
    }

    fn store_cli_admission(input: RetentionCliObject<'_>, kind: &str, requester_ref: &str) -> String {
        molten::retention::store_evidence_admission(input.root, &molten::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            bound_refs: &[test_ref(&format!("{kind}-{}", input.label))],
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store cli retention admission")
        .admission_ref
    }

    fn only_blob_ref(iroh_store: &std::path::Path) -> String {
        let mut refs = fs::read_dir(iroh_store.join("blobs"))
            .expect("read iroh blobs")
            .map(|entry| {
                let file_name = entry.expect("blob entry").file_name().to_string_lossy().into_owned();
                let hex = file_name
                    .strip_prefix("blake3_")
                    .and_then(|name| name.strip_suffix(".bin"))
                    .expect("blob file name");
                molten::preserves_rail::content_ref_from_hex(hex).expect("canonical blob ref")
            })
            .collect::<Vec<_>>();
        refs.sort();
        assert_eq!(refs.len(), 1);
        refs.remove(0)
    }

    fn test_ref(label: &str) -> String {
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("test-ref", vec![
            molten::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }

    fn install_cli_stage_artifact(registry: &Path, operation: &str) -> String {
        let payload = molten::job_dag::builtin_stage_operation_value(operation).expect("stage operation");
        let installed = molten::artifacts::install_artifact(registry, &molten::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload,
            schema_refs: vec![cli_synthetic_ref("job-worker-stage-schema").expect("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-stage-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-stage-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-stage-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-stage-capability").expect("capability")],
        })
        .expect("install stage artifact");
        assert_eq!(installed.decision, "pass");
        installed.artifact_ref
    }

    fn install_cli_clean_octet_gate(registry: &Path) -> String {
        let gate_value = molten::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate");
        let gate_ref = canonical_hash(&gate_value).expect("gate ref");
        let installed = molten::artifacts::install_artifact(registry, &molten::artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-octet-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-octet-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-octet-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-octet-capability").expect("capability")],
        })
        .expect("install octet gate");
        assert_eq!(installed.decision, "pass");
        gate_ref
    }

    fn install_cli_job_execute_authority_context(registry: &Path, job_ref: &str) -> String {
        let subject_ref = cli_synthetic_ref("job-worker-target-subject").expect("subject");
        let context_value = molten::authority::context_value(molten::authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[molten::authority::Capability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[cli_synthetic_ref("job-worker-authority-policy").expect("policy")],
            evidence_refs: &[cli_synthetic_ref("job-worker-authority-evidence").expect("evidence")],
        })
        .expect("authority context");
        let context_ref = canonical_hash(&context_value).expect("authority context ref");
        let installed = molten::artifacts::install_artifact(registry, &molten::artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-authority-install-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-authority-install-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-authority-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-authority-install-capability").expect("capability")],
        })
        .expect("install authority context");
        assert_eq!(installed.decision, "pass");
        context_ref
    }

    pub(crate) fn temp_dir(label: &str) -> crate::test_support::ProcessWorkspace {
        crate::test_support::process_workspace(label).expect("create isolated process workspace")
    }
