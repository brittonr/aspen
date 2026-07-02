include!("provenance/provenance.rs");

#[test]
fn cli_service_runtime_commands_work() {
    let dir = temp_dir("service-cli");
    let out = dir.join("two-service");
    run_service_command(ServiceCommand::RunTwoService { out: out.clone() })
        .expect("run two-service service runtime");
    let report = out.join("report.preserves");
    assert!(report.exists());
    run_service_command(ServiceCommand::Show {
        report: report.clone(),
    })
    .expect("show service runtime report");
    run_service_command(ServiceCommand::Replay {
        report: report.clone(),
    })
    .expect("replay service runtime report");
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
    run_service_command(ServiceCommand::ShowSupervision {
        report: report.clone(),
    })
    .expect("show service supervision report");
    run_service_command(ServiceCommand::ReplaySupervision {
        report: report.clone(),
    })
    .expect("replay service supervision report");
    run_service_command(ServiceCommand::GateSupervision {
        report: report.clone(),
        receipt_out: Some(dir.join("supervision-gate.preserves")),
    })
    .expect("gate service supervision report");
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
    let envelope_out = build_remote_envelope(&dir);
    let envelope = read_remote_envelope(&envelope_out);
    publish_and_deliver_remote_envelope(&dir, &envelope_out, envelope.envelope_ref);
    gate_remote_two_peer(&dir);
}

fn build_remote_envelope(dir: &Path) -> PathBuf {
    let payload = dir.join("payload.preserves");
    write_file(&payload, r#"<remote-payload "ready">"#).expect("write remote payload");
    let envelope_out = dir.join("envelope.preserves");
    run_remote_command(RemoteCommand::Envelope {
        command: RemoteEnvelopeCommand::Build {
            from_peer: "peer:a".to_owned(),
            from_actor: "actor:producer".to_owned(),
            to_peer: "peer:b".to_owned(),
            topic: "services".to_owned(),
            operation: "assert".to_owned(),
            payload,
            content_refs: Vec::new(),
            capability_refs: vec![cli_synthetic_ref("remote-capability").expect("remote capability ref")],
            evidence_refs: vec![cli_synthetic_ref("remote-evidence").expect("remote evidence ref")],
            out: envelope_out.clone(),
        },
    })
    .expect("build remote envelope");
    envelope_out
}

fn read_remote_envelope(path: &Path) -> molten::remote_dataspace::RemoteDataspaceEnvelope {
    let value = read_preserves_file(path).expect("read remote envelope");
    molten::remote_dataspace::parse_envelope(&value).expect("parse remote envelope")
}

fn publish_and_deliver_remote_envelope(dir: &Path, envelope_out: &Path, envelope_ref: String) {
    let transport_root = dir.join("transport");
    fs::create_dir_all(&transport_root).expect("create remote transport root");
    run_remote_command(RemoteCommand::PublishLocal {
        transport_root: transport_root.clone(),
        envelope: envelope_out.to_path_buf(),
        node: "peer:a".to_owned(),
        receipt_out: Some(dir.join("publish.preserves")),
    })
    .expect("publish remote envelope");
    run_remote_command(RemoteCommand::DeliverLocal {
        transport_root,
        topic: "services".to_owned(),
        envelope_ref,
        receiver_peer: "peer:b".to_owned(),
        out: Some(dir.join("delivered.preserves")),
        receipt_out: Some(dir.join("deliver.preserves")),
    })
    .expect("deliver remote envelope");
}

fn gate_remote_two_peer(dir: &Path) {
    let transport_root = dir.join("two-peer-transport");
    let out = dir.join("two-peer");
    run_remote_command(RemoteCommand::RunTwoPeer {
        transport_root,
        out: out.clone(),
    })
    .expect("run two peer remote scenario");
    let turn_ref = read_turn_context_ref(&out.join("turn-context-ref.preserves"));
    let gate_out = dir.join("remote-gate.preserves");
    run_remote_command(RemoteCommand::Gate {
        delivery_log: out.join("delivery-log.preserves"),
        admission_receipts: vec![out.join("admission-receipt.preserves")],
        turn_context_refs: vec![turn_ref],
        receipt_out: Some(gate_out.clone()),
    })
    .expect("remote gate");
    let gate = read_preserves_file(&gate_out).expect("read remote gate");
    assert_eq!(molten::ledger::artifact_kind(&gate), "remote-dataspace-gate-receipt");
    let missing = run_remote_command(RemoteCommand::Gate {
        delivery_log: out.join("delivery-log.preserves"),
        admission_receipts: Vec::new(),
        turn_context_refs: Vec::new(),
        receipt_out: None,
    })
    .expect_err("missing admission receipt denies remote gate");
    assert!(missing.to_string().contains("admission receipt"));
}

fn read_turn_context_ref(path: &Path) -> String {
    read_preserves_file(path)
        .expect("read turn ref")
        .as_string()
        .expect("turn ref string")
        .into_owned()
}
