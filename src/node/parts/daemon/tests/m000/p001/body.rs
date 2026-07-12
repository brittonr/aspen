
    #[test]
    fn control_provenance_gate_denies_missing_and_tampered_evidence_before_side_effects() {
        let root = initialized_control_root("node-control-provenance", "node:provenance");
        let refs = case_refs("provenance");

        assert_missing_case(&root, &refs);
        assert_queued_case(&root, &refs);
        assert_tampered_case(&root, &refs);
    }

    struct CaseRefs {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn case_refs(label: &str) -> CaseRefs {
        CaseRefs {
            authority_refs: vec![local_ref("node-control-authority", label).expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", label).expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", label).expect("resource ref")],
        }
    }

    fn request_value(payload_ref: &str, refs: &CaseRefs, evidence_refs: &[String]) -> IoValue {
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(payload_ref),
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs,
        })
        .expect("install request")
    }

    fn assert_registry_empty(root: &Path) {
        assert!(
            crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );
    }

    fn assert_missing_case(root: &Path, refs: &CaseRefs) {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "missing-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload_value).expect("import payload");
        let request = request_value(&payload_ref, refs, &[]);
        let dispatch = submit_and_dispatch(root, &request);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(
            receipt
                .subreceipt_refs
                .iter()
                .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
        );
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
        assert_registry_empty(root);
    }

    fn assert_queued_case(root: &Path, refs: &CaseRefs) {
        let payload =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "queued-missing-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload).expect("import queued payload");
        let request = request_value(&payload_ref, refs, &[]);
        let queued = crate::node_runtime::parse_control_request(&request).expect("queued request parse");
        submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value: &request,
        })
        .expect("submit queued missing provenance");
        let loop_result = run_control_loop(&ControlLoopInput {
            state_root: root,
            max_requests: 1,
        })
        .expect("process queued missing provenance");
        assert_eq!(loop_result.processed_request_refs, vec![queued.request_ref.clone()]);
        let state_root = crate::node_state::NodeStateRoot::open(root).expect("open node state root");
        let value = read_preserves(
            &state_root,
            &control_outbox_receipt_path(&queued.request_ref).expect("outbox receipt path"),
        )
        .expect("queued receipt value");
        let receipt = crate::node_runtime::parse_control_receipt(&value).expect("queued receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance evidence")));
    }

    fn assert_tampered_case(root: &Path, refs: &CaseRefs) {
        let payload =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "tampered-provenance",
            )]);
        let payload_ref = import_artifact(root, &payload).expect("import tampered payload");
        let wrong_artifact_ref = local_ref("node-control-wrong-provenance-artifact", "tampered").expect("wrong ref");
        let wrong_provenance =
            crate::provenance::synthetic_reviewed_record(&wrong_artifact_ref).expect("wrong provenance");
        let wrong_ref = import_artifact(root, &wrong_provenance).expect("import wrong provenance");
        let evidence_refs = vec![wrong_ref];
        let request = request_value(&payload_ref, refs, &evidence_refs);
        let dispatch = submit_and_dispatch(root, &request);
        let receipt =
            crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("tampered receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("no provenance record matches")));
        assert_registry_empty(root);
    }

    #[test]
    fn control_reproducible_provenance_requires_build_verification_binding() {
        let root = temp_dir("node-control-reproducible-provenance");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:reproducible-provenance",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");

        let case = build_case(&root);
        assert_install_passes(&root, &case);
    }

    struct BuildMaterial {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        payload_ref: String,
        source_refs: Vec<String>,
        toolchain_refs: Vec<String>,
        dependency_ref: String,
        builder_ref: String,
    }

    struct BuildCase {
        material: BuildMaterial,
        evidence_refs: Vec<String>,
    }

    fn build_case(root: &Path) -> BuildCase {
        let material = build_material(root);
        let evidence_refs = verified_refs(root, &material);
        BuildCase {
            material,
            evidence_refs,
        }
    }

    fn build_material(root: &Path) -> BuildMaterial {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "reproducible-provenance",
            )]);
        BuildMaterial {
            authority_refs: vec![local_ref("node-control-authority", "reproducible").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "reproducible").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "reproducible").expect("resource ref")],
            payload_ref: import_artifact(root, &payload_value).expect("import payload"),
            source_refs: vec![local_ref("node-control-source", "reproducible").expect("source ref")],
            toolchain_refs: vec![local_ref("node-control-toolchain", "reproducible").expect("toolchain ref")],
            dependency_ref: local_ref("node-control-deps", "reproducible").expect("deps ref"),
            builder_ref: local_ref("node-control-builder", "reproducible").expect("builder ref"),
        }
    }

    fn build_record_for(material: &BuildMaterial) -> IoValue {
        crate::provenance::build_record_value(&crate::provenance::BuildRecordInput {
            expected_artifact_ref: &material.payload_ref,
            source_refs: &material.source_refs,
            dependency_closure_ref: &material.dependency_ref,
            toolchain_refs: &material.toolchain_refs,
            build_params: &[],
            builder_ref: &material.builder_ref,
            nix_derivation_refs: &[],
            policy_refs: &material.policy_refs,
            evidence_refs: &[],
        })
        .expect("build record")
    }

    fn provenance_record_for(material: &BuildMaterial, build_record_refs: &[String]) -> IoValue {
        crate::provenance::record_value(&crate::provenance::RecordInput {
            artifact_ref: &material.payload_ref,
            trust_state: crate::provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &material.source_refs,
            dependency_closure_ref: &material.dependency_ref,
            toolchain_refs: &material.toolchain_refs,
            builder_ref: &material.builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &material.policy_refs,
            build_record_refs,
        })
        .expect("reproducible provenance")
    }

    fn verified_refs(root: &Path, material: &BuildMaterial) -> Vec<String> {
        let build_record = build_record_for(material);
        let build_record_ref = import_artifact(root, &build_record).expect("import build record");
        let build_verification = crate::provenance::verify_build(&crate::provenance::BuildVerificationInput {
            build_record_value: &build_record,
            actual_artifact_ref: &material.payload_ref,
            prior_diagnostics: &[],
        })
        .expect("verify build");
        let build_verification_ref =
            import_artifact(root, &build_verification.receipt_value).expect("import build verification");
        let build_record_refs = vec![build_record_ref];
        let provenance_record = provenance_record_for(material, &build_record_refs);
        let provenance_ref = import_artifact(root, &provenance_record).expect("import provenance");
        vec![provenance_ref, build_verification_ref]
    }

    fn install_request_for(case: &BuildCase) -> IoValue {
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&case.material.payload_ref),
            authority_refs: &case.material.authority_refs,
            policy_refs: &case.material.policy_refs,
            resource_refs: &case.material.resource_refs,
            evidence_refs: &case.evidence_refs,
        })
        .expect("reproducible install request")
    }

    fn assert_install_passes(root: &Path, case: &BuildCase) {
        let request = install_request_for(case);
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: root,
            request_value: &request,
        })
        .expect("submit reproducible request");
        let dispatch = dispatch_control_request_entry(&ControlDispatchEntryInput {
            state_root: root,
            request_entry: Some(&submitted.inbox_entry),
        })
        .expect("dispatch reproducible request");
        let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value)
            .expect("reproducible control receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert!(
            !crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );
    }

    #[test]
    fn control_ingress_enqueues_once_and_preserves_provenance_gate() {
        let root = temp_dir("node-control-ingress");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];

        let payload_value =
            crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
                "missing-provenance",
            )]);
        let payload_ref = import_artifact(&root, &payload_value).expect("import payload");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("install request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:operator",
            to_node: "node:ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
        assert_enqueued_then_denied(&root, &envelope);
    }
