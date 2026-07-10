    #[test]
    fn cli_coordination_commands_work() {
        let dir = temp_dir("coordination-cli");
        let manifest = coordination_manifest_file(&dir);
        let request = coordination_request_file(&dir);
        coordination_apply_requests(&dir, manifest, request);
    }

    fn coordination_manifest_file(dir: &Path) -> PathBuf {
        check_coordination_fixture(dir);
        write_coordination_manifest(dir)
    }

    fn check_coordination_fixture(dir: &Path) {
        let out = dir.join("coordination-fixture");
        run_coordination_command(CoordinationCommand::RunFixture { out: out.clone() }).expect("coordination fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read coordination report");
        assert!(
            to_text(&report_value)
                .expect("render coordination report")
                .contains("coordination-fixture-report-v1")
        );
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read coordination manifest");
        let parsed =
            molten::coordination::parse_coordination_service_manifest(&manifest_value).expect("parse coordination manifest");
        assert_eq!(parsed.service_id, "coordination:local");
        run_coordination_command(CoordinationCommand::Show { artifact: manifest }).expect("coordination show manifest");
    }

    fn write_coordination_manifest(dir: &Path) -> PathBuf {
        let policy_ref = cli_synthetic_ref("coordination-cli-policy").expect("policy ref");
        let resource_ref = cli_synthetic_ref("coordination-cli-resource").expect("resource ref");
        let generated_manifest = dir.join("coordination.manifest.preserves");
        run_coordination_command(CoordinationCommand::Manifest {
            service_id: "coordination:local".to_string(),
            services: vec![molten::coordination::SERVICE_QUEUE.to_string()],
            control_group_ref: None,
            queue_capacity: 2,
            semaphore_capacity: molten::coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
            rate_limit: molten::coordination::DEFAULT_COORDINATION_RATE_LIMIT,
            barrier_parties: molten::coordination::DEFAULT_COORDINATION_BARRIER_PARTIES,
            policy_refs: vec![policy_ref],
            resource_refs: vec![resource_ref],
            out: Some(generated_manifest.clone()),
        })
        .expect("coordination manifest");
        let generated_manifest_value = read_preserves_file(&generated_manifest).expect("read generated manifest");
        let generated_manifest_parsed = molten::coordination::parse_coordination_service_manifest(&generated_manifest_value)
            .expect("parse generated coordination manifest");
        assert_eq!(generated_manifest_parsed.services, vec![molten::coordination::SERVICE_QUEUE.to_string()]);
        generated_manifest
    }

    fn coordination_request_file(dir: &Path) -> PathBuf {
        let policy_ref = cli_synthetic_ref("coordination-cli-policy").expect("policy ref");
        let resource_ref = cli_synthetic_ref("coordination-cli-resource").expect("resource ref");
        let authority_ref = cli_synthetic_ref("coordination-cli-authority").expect("authority ref");
        let operation_id_ref = cli_synthetic_ref("coordination-cli-operation").expect("operation ref");
        let payload = dir.join("queue-item.preserves");
        write_file(&payload, r#"<item "cli-one">"#).expect("write queue payload");
        let request = dir.join("coordination.request.preserves");
        run_coordination_command(CoordinationCommand::Request {
            service: molten::coordination::SERVICE_QUEUE.to_string(),
            operation: molten::coordination::OP_ENQUEUE.to_string(),
            key: "queue:cli".to_string(),
            client_session: "client-cli".to_string(),
            operation_id_ref,
            read_consistency_mode: molten::coordination::READ_CONSISTENCY_LINEARIZABLE.to_string(),
            payload: Some(payload),
            authority_refs: vec![authority_ref],
            resource_refs: vec![resource_ref],
            policy_refs: vec![policy_ref],
            out: Some(request.clone()),
        })
        .expect("coordination request");
        run_coordination_command(CoordinationCommand::Show {
            artifact: request.clone(),
        })
        .expect("show request");
        request
    }

    fn coordination_apply_requests(dir: &Path, manifest: PathBuf, request: PathBuf) {
        let apply_out = dir.join("coordination-apply");
        run_coordination_command(CoordinationCommand::Apply {
            manifest,
            requests: vec![request.clone(), request],
            out: apply_out.clone(),
        })
        .expect("coordination apply");
        let apply_report = read_preserves_file(&apply_out.join("report.preserves")).expect("read apply report");
        let parsed_report = molten::coordination::parse_coordination_apply_report(&apply_report).expect("parse apply report");
        assert_eq!(parsed_report.decision, "pass");
        assert_eq!(parsed_report.receipt_refs.len(), 2);
        assert_ne!(parsed_report.receipt_refs[0], parsed_report.receipt_refs[1]);
        assert_eq!(parsed_report.assertion_refs.len(), 1);
        run_coordination_command(CoordinationCommand::Show {
            artifact: apply_out.join("report.preserves"),
        })
        .expect("coordination show apply report");
    }
