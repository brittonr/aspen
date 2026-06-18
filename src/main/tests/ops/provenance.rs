    #[test]
    fn cli_provenance_commands_work() {
        let dir = temp_dir("provenance-cli");
        let artifact_ref = cli_synthetic_ref("provenance-artifact").expect("artifact ref");
        let fixture_out = dir.join("reviewed.preserves");
        run_provenance_command(ProvenanceCommand::Fixture {
            artifact_ref: artifact_ref.clone(),
            out: Some(fixture_out.clone()),
        })
        .expect("write reviewed provenance fixture");
        run_provenance_command(ProvenanceCommand::Show {
            artifact: fixture_out.clone(),
        })
        .expect("show provenance fixture");
        let pass_receipt = dir.join("provenance-pass.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "install".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: artifact_ref.clone(),
            provenance_paths: vec![fixture_out.clone()],
            build_verification_paths: Vec::new(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(pass_receipt.clone()),
        })
        .expect("evaluate passing provenance");
        let pass_summary =
            provenance::provenance_summary(&read_preserves_file(&pass_receipt).expect("read provenance pass receipt"))
                .expect("summarize provenance pass receipt");
        assert!(pass_summary.contains("decision=pass"));

        let sandbox_ref = cli_synthetic_ref("provenance-sandbox-artifact").expect("sandbox ref");
        let sandbox_out = dir.join("sandbox.preserves");
        run_provenance_command(ProvenanceCommand::Record {
            artifact_ref: sandbox_ref.clone(),
            trust_state: provenance::TRUST_STATE_SANDBOX_ONLY.to_string(),
            source_refs: vec![cli_synthetic_ref("provenance-source").expect("source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-deps").expect("deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-toolchain").expect("toolchain ref")],
            builder_ref: cli_synthetic_ref("provenance-builder").expect("builder ref"),
            review_refs: Vec::new(),
            test_refs: Vec::new(),
            source_gate_refs: Vec::new(),
            policy_refs: vec![cli_synthetic_ref("provenance-policy").expect("policy ref")],
            build_record_refs: Vec::new(),
            out: Some(sandbox_out.clone()),
        })
        .expect("write sandbox provenance record");
        let deny_receipt = dir.join("provenance-deny.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "run".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: sandbox_ref,
            provenance_paths: vec![sandbox_out],
            build_verification_paths: Vec::new(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(deny_receipt.clone()),
        })
        .expect("evaluate denied provenance");
        let deny_summary =
            provenance::provenance_summary(&read_preserves_file(&deny_receipt).expect("read provenance deny receipt"))
                .expect("summarize provenance deny receipt");
        assert!(deny_summary.contains("decision=deny"));

        let build_record = dir.join("build-record.preserves");
        let actual_ref = cli_synthetic_ref("provenance-actual-artifact").expect("actual ref");
        run_provenance_command(ProvenanceCommand::BuildRecord {
            expected_artifact_ref: artifact_ref.clone(),
            source_refs: vec![cli_synthetic_ref("provenance-build-source").expect("build source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-build-deps").expect("build deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-build-toolchain").expect("build toolchain ref")],
            build_params: vec!["target=x86_64-linux".to_string()],
            builder_ref: cli_synthetic_ref("provenance-build-builder").expect("build builder ref"),
            nix_derivation_refs: vec![cli_synthetic_ref("provenance-build-derivation").expect("build derivation ref")],
            policy_refs: vec![cli_synthetic_ref("provenance-build-policy").expect("build policy ref")],
            evidence_refs: vec![cli_synthetic_ref("provenance-build-evidence").expect("build evidence ref")],
            out: Some(build_record.clone()),
        })
        .expect("write provenance build record");
        run_provenance_command(ProvenanceCommand::Show {
            artifact: build_record.clone(),
        })
        .expect("show provenance build record");
        let build_pass = dir.join("build-pass.preserves");
        run_provenance_command(ProvenanceCommand::VerifyBuild {
            build_record: build_record.clone(),
            actual_artifact_ref: artifact_ref.clone(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(build_pass.clone()),
        })
        .expect("verify provenance build pass");
        let build_pass_summary = provenance::provenance_summary(
            &read_preserves_file(&build_pass).expect("read provenance build pass receipt"),
        )
        .expect("summarize provenance build pass");
        assert!(build_pass_summary.contains("decision=pass"));
        let build_record_ref =
            canonical_hash(&read_preserves_file(&build_record).expect("read build record")).expect("build record ref");
        let reproducible_record = dir.join("reproducible.preserves");
        run_provenance_command(ProvenanceCommand::Record {
            artifact_ref: artifact_ref.clone(),
            trust_state: provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED.to_string(),
            source_refs: vec![cli_synthetic_ref("provenance-repro-source").expect("repro source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-repro-deps").expect("repro deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-repro-toolchain").expect("repro toolchain ref")],
            builder_ref: cli_synthetic_ref("provenance-repro-builder").expect("repro builder ref"),
            review_refs: Vec::new(),
            test_refs: Vec::new(),
            source_gate_refs: Vec::new(),
            policy_refs: Vec::new(),
            build_record_refs: vec![build_record_ref],
            out: Some(reproducible_record.clone()),
        })
        .expect("write reproducible provenance record");
        let reproducible_receipt = dir.join("provenance-reproducible-pass.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "install".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: artifact_ref.clone(),
            provenance_paths: vec![reproducible_record],
            build_verification_paths: vec![build_pass.clone()],
            prior_diagnostics: Vec::new(),
            receipt_out: Some(reproducible_receipt.clone()),
        })
        .expect("evaluate reproducible provenance");
        let reproducible_summary = provenance::provenance_summary(
            &read_preserves_file(&reproducible_receipt).expect("read reproducible receipt"),
        )
        .expect("summarize reproducible receipt");
        assert!(reproducible_summary.contains("decision=pass"));
        let build_deny = dir.join("build-deny.preserves");
        run_provenance_command(ProvenanceCommand::VerifyBuild {
            build_record,
            actual_artifact_ref: actual_ref,
            prior_diagnostics: Vec::new(),
            receipt_out: Some(build_deny.clone()),
        })
        .expect("verify provenance build deny");
        let build_deny_summary = provenance::provenance_summary(
            &read_preserves_file(&build_deny).expect("read provenance build deny receipt"),
        )
        .expect("summarize provenance build deny");
        assert!(build_deny_summary.contains("decision=deny"));
    }

    #[test]
    fn cli_service_runtime_commands_work() {
        let dir = temp_dir("service-cli");
        let out = dir.join("two-service");
        run_service_command(ServiceCommand::RunTwoService { out: out.clone() })
            .expect("run two-service service runtime");
        let report = out.join("report.preserves");
        assert!(report.exists());
        run_service_command(ServiceCommand::Show { report: report.clone() }).expect("show service runtime report");
        run_service_command(ServiceCommand::Replay { report: report.clone() }).expect("replay service runtime report");
        let rerun = dir.join("rerun");
        run_service_command(ServiceCommand::Run {
            suite: out.join("suite.preserves"),
            out: rerun.clone(),
        })
        .expect("rerun service runtime suite");
        assert!(read_preserves_file(&rerun.join("readiness-1.preserves")).is_ok());
    }

    #[test]
    fn cli_service_supervision_commands_work() {
        let dir = temp_dir("service-supervision-cli");
        let out = dir.join("supervision");
        run_service_command(ServiceCommand::RunSupervisionFixture { out: out.clone() })
            .expect("run service supervision fixture");
        let report = out.join("report.preserves");
        assert!(report.exists());
        run_service_command(ServiceCommand::ShowSupervision { report: report.clone() })
            .expect("show service supervision report");
        run_service_command(ServiceCommand::ReplaySupervision { report: report.clone() })
            .expect("replay service supervision report");
        let gate_receipt = dir.join("supervision-gate.preserves");
        run_service_command(ServiceCommand::GateSupervision {
            report: report.clone(),
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("gate service supervision report");
        let gate = service_supervision::parse_service_supervision_gate_receipt(
            &read_preserves_file(&gate_receipt).expect("read supervision gate receipt"),
        )
        .expect("parse supervision gate receipt");
        assert_eq!(gate.decision, "pass");
        let rerun = dir.join("supervision-rerun");
        run_service_command(ServiceCommand::Supervise {
            suite: out.join("suite.preserves"),
            out: rerun.clone(),
        })
        .expect("rerun service supervision suite");
        assert!(read_preserves_file(&rerun.join("monitor-notification-0.preserves")).is_ok());
    }

    #[test]
    fn cli_remote_dataspace_commands_work() {
        let dir = temp_dir("remote-cli");
        let payload = PathBuf::from("examples/remote-service-ready.preserves");
        let parsed_payload = read_preserves_file(&payload).expect("example payload parses");
        let payload_ref = canonical_hash(&parsed_payload).expect("payload ref");
        molten::preserves_rail::validate_content_ref(&payload_ref).expect("payload ref is canonical");
        let envelope_out = dir.join("envelope.preserves");
        run_remote_command(RemoteCommand::Envelope {
            command: RemoteEnvelopeCommand::Build {
                from_peer: "peer:a".to_owned(),
                from_actor: "producer".to_owned(),
                to_peer: "peer:b".to_owned(),
                topic: "services".to_owned(),
                operation: "assert".to_owned(),
                payload,
                content_refs: Vec::new(),
                capability_refs: Vec::new(),
                evidence_refs: Vec::new(),
                out: envelope_out.clone(),
            },
        })
        .expect("build remote envelope");
        let envelope = remote_dataspace::parse_envelope(&read_preserves_file(&envelope_out).expect("read envelope"))
            .expect("parse envelope");
        let transport_root = dir.join("transport");
        run_remote_command(RemoteCommand::PublishLocal {
            transport_root: transport_root.clone(),
            envelope: envelope_out.clone(),
            node: "peer:a".to_owned(),
            receipt_out: Some(dir.join("publish.preserves")),
        })
        .expect("publish remote envelope");
        run_remote_command(RemoteCommand::DeliverLocal {
            transport_root: transport_root.clone(),
            topic: "services".to_owned(),
            envelope_ref: envelope.envelope_ref,
            receiver_peer: "peer:b".to_owned(),
            out: Some(dir.join("delivered.preserves")),
            receipt_out: Some(dir.join("deliver.preserves")),
        })
        .expect("deliver remote envelope");

        let out = dir.join("two-peer");
        run_remote_command(RemoteCommand::RunTwoPeer {
            transport_root,
            out: out.clone(),
        })
        .expect("run two peer remote scenario");
        let turn_ref_value = read_preserves_file(&out.join("turn-context-ref.preserves")).expect("read turn ref");
        let turn_ref = turn_ref_value.as_string().expect("turn ref string").into_owned();
        let gate_out = dir.join("remote-gate.preserves");
        run_remote_command(RemoteCommand::Gate {
            delivery_log: out.join("delivery-log.preserves"),
            admission_receipts: vec![out.join("admission-receipt.preserves")],
            turn_context_refs: vec![turn_ref],
            receipt_out: Some(gate_out.clone()),
        })
        .expect("remote gate");
        let gate = read_preserves_file(&gate_out).expect("read remote gate");
        assert_eq!(ledger::artifact_kind(&gate), "remote-dataspace-gate-receipt");
        let missing = run_remote_command(RemoteCommand::Gate {
            delivery_log: out.join("delivery-log.preserves"),
            admission_receipts: Vec::new(),
            turn_context_refs: Vec::new(),
            receipt_out: None,
        })
        .expect_err("missing admission receipt denies remote gate");
        assert!(missing.to_string().contains("admission receipt"));
    }
