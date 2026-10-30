struct RecordedCliFixture {
    _workspace: test_support::ProcessWorkspace,
    sender: std::path::PathBuf,
    receiver: std::path::PathBuf,
    transport: std::path::PathBuf,
    introduction: std::path::PathBuf,
    message: std::path::PathBuf,
    reference: String,
}

impl RecordedCliFixture {
    fn new() -> CliResult<Self> {
        let workspace = temp_dir("cli-recorded-remote-session")?;
        let sender = workspace.join("sender");
        let receiver = workspace.join("receiver");
        for (root, node) in [(&sender, "node:recorded-cli-sender"), (&receiver, "node:recorded-cli-receiver")] {
            let output = molten_cmd().args(["node", "init", "--state-root"]).arg(root)
                .args(["--node-id", node, "--config-out"]).arg(root.with_extension("config.preserves")).output()?;
            assert_success(&output, "initialize actual recorded-session node");
        }
        let transport = workspace.join("transport");
        let reference = molten::remote_dataspace::store_content_blob(&transport, b"actual recorded CLI content")?;
        let introduction = workspace.join("introduction.preserves");
        let message = workspace.join("message.preserves");
        std::fs::write(&introduction, format!("<ready \"{reference}\">"))?;
        std::fs::write(&message, format!("<use \"état 🦀\" \"{reference}\">"))?;
        Ok(Self { _workspace: workspace, sender, receiver, transport, introduction, message, reference })
    }

    fn session(&self, operation: &str, receipt: &std::path::Path) -> std::process::Command {
        let mut command = molten_cmd();
        command.args(["test", "remote", "session", operation, "--sender-state-root"]).arg(&self.sender)
            .arg("--receiver-state-root").arg(&self.receiver).args(["--topic", "recorded-cli", "--receipt-out"])
            .arg(receipt);
        command
    }

    fn source(&self, command: &mut std::process::Command, generation: u64) {
        command.arg("--sender-state-root").arg(&self.sender).arg("--receiver-state-root").arg(&self.receiver)
            .arg("--session-generation").arg(generation.to_string());
    }

    fn build(&self, generation: u64, out: &std::path::Path, receipt: &std::path::Path) -> std::process::Command {
        let mut command = molten_cmd();
        command.args(["test", "remote", "envelope", "build", "--from-peer", "node:recorded-cli-sender", "--from-actor", "producer",
            "--to-peer", "node:recorded-cli-receiver", "--topic", "recorded-cli", "--operation", "message", "--payload"])
            .arg(&self.message).arg("--content-ref").arg(&self.reference)
            // Neither carried list may establish receiver knowledge.
            .arg("--capability-ref").arg(&self.reference).arg("--evidence-ref").arg(&self.reference)
            .arg("--out").arg(out).arg("--receipt-out").arg(receipt);
        self.source(&mut command, generation);
        command
    }

    fn publish(&self, generation: u64, envelope: &std::path::Path, receipt: &std::path::Path) -> std::process::Command {
        let mut command = molten_cmd();
        command.args(["test", "remote", "publish-local", "--transport-root"]).arg(&self.transport)
            .arg("--envelope").arg(envelope).args(["--node", "node:recorded-cli-sender", "--receipt-out"]).arg(receipt);
        self.source(&mut command, generation);
        command
    }
}

fn recorded_cli_generation(path: &std::path::Path) -> CliResult<u64> {
    let receipt = read_preserves(path)?;
    let commit = receipt.collect_simple_record("remote-recorded-session-commit-v1", Some(2))
        .ok_or_else(|| std::io::Error::other("missing actual receiver commit receipt"))?;
    let observation = commit[0].collect_simple_record("remote-recorded-session-observation-v1", Some(8))
        .ok_or_else(|| std::io::Error::other("missing canonical committed observation"))?;
    let generation = observation[1].as_u64().ok_or_else(|| std::io::Error::other("generation is not an integer"))?
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(generation)
}

fn recorded_cli_build_denial(path: &std::path::Path, reference: &str) -> CliResult<()> {
    let receipt = read_preserves(path)?;
    let fields = receipt.collect_simple_record("remote-dataspace-envelope-build-denial-v1", Some(7))
        .ok_or_else(|| std::io::Error::other("missing canonical sender denial"))?;
    assert_eq!(fields[1].as_string().as_deref(), Some("deny"));
    let diagnostics = fields[6].collect_sequence().ok_or_else(|| std::io::Error::other("denial diagnostics are not a sequence"))?;
    assert!(diagnostics.iter().any(|value| value.as_string().is_some_and(|diagnostic| diagnostic.contains(reference))));
    Ok(())
}

fn recorded_cli_snapshot_ref(fixture: &RecordedCliFixture, generation: u64) -> CliResult<String> {
    let sender = molten::node_state::DirectoryRoot::open_existing(&fixture.sender)?;
    let receiver = molten::node_state::DirectoryRoot::open_existing(&fixture.receiver)?;
    let owner = molten::remote_dataspace::RecordedSessionStore::open(&receiver)?;
    let view = owner.sender_view(&sender, "recorded-cli", generation)?;
    let snapshot = view.snapshot()?;
    Ok(snapshot.snapshot_ref()?)
}

fn recorded_cli_reject_late_receive(fixture: &RecordedCliFixture, generation: u64, envelope: &std::path::Path) -> CliResult<()> {
    let before = recorded_cli_snapshot_ref(fixture, generation)?;
    let denied = fixture.receiver.with_extension("receiver-denial.preserves");
    let output = molten_cmd().args(["test", "remote", "session", "receive", "--receiver-state-root"]).arg(&fixture.receiver)
        .arg("--generation").arg(generation.to_string()).arg("--envelope").arg(envelope)
        .arg("--receipt-out").arg(&denied).output()?;
    assert_failure(&output, "receiver refuses a late envelope after retraction");
    let receipt = read_preserves(&denied)?;
    let fields = receipt.collect_simple_record("remote-recorded-session-denial-v1", Some(8))
        .ok_or_else(|| std::io::Error::other("missing canonical receiver denial"))?;
    assert_eq!(fields[0].as_string().as_deref(), Some("deny"));
    let diagnostics = fields[6].collect_sequence().ok_or_else(|| std::io::Error::other("missing denial diagnostics"))?;
    assert!(diagnostics.iter().any(|value| value.as_string().is_some_and(|text| text.contains(&fixture.reference))));
    assert_eq!(recorded_cli_snapshot_ref(fixture, generation)?, before, "receiver denial must leave runtime state unchanged");
    Ok(())
}

fn recorded_cli_positive_delivery(fixture: &RecordedCliFixture, generation: u64) -> CliResult<std::path::PathBuf> {
    let introduced = fixture.receiver.with_extension("assert-commit.preserves");
    let output = fixture.session("commit", &introduced).arg("--generation").arg(generation.to_string())
        .args(["--operation", "assert", "--payload"]).arg(&fixture.introduction).output()?;
    assert_success(&output, "receiver commits actual introducing assertion");
    let envelope = fixture.receiver.with_extension("message-envelope.preserves");
    let build_receipt = fixture.receiver.with_extension("message-build.preserves");
    assert_success(&fixture.build(generation, &envelope, &build_receipt).output()?, "introduced message builds");
    let parsed = molten::remote_dataspace::parse_envelope(&read_preserves(&envelope)?)?;
    let published = fixture.receiver.with_extension("published.preserves");
    assert_success(&fixture.publish(generation, &envelope, &published).output()?, "introduced envelope publishes");
    let delivered = fixture.receiver.with_extension("delivered-envelope.preserves");
    let delivery_receipt = fixture.receiver.with_extension("delivered.preserves");
    let output = molten_cmd().args(["test", "remote", "deliver-local", "--transport-root"]).arg(&fixture.transport)
        .args(["--topic", "recorded-cli", "--envelope-ref"]).arg(&parsed.envelope_ref)
        .args(["--receiver-peer", "node:recorded-cli-receiver", "--out"]).arg(&delivered)
        .arg("--receipt-out").arg(&delivery_receipt).output()?;
    assert_success(&output, "actual local transport delivers canonical envelope");
    let committed = fixture.receiver.with_extension("message-committed.preserves");
    let output = molten_cmd().args(["test", "remote", "session", "receive", "--receiver-state-root"]).arg(&fixture.receiver)
        .arg("--generation").arg(generation.to_string()).arg("--envelope").arg(&delivered)
        .arg("--receipt-out").arg(&committed).output()?;
    assert_success(&output, "receiver commits delivered message through real runtime");
    let sender_root = molten::node_state::DirectoryRoot::open_existing(&fixture.sender)?;
    let receiver_root = molten::node_state::DirectoryRoot::open_existing(&fixture.receiver)?;
    let source = molten::remote_dataspace::RecordedSessionStore::open(&receiver_root)?;
    let snapshot = source.sender_view(&sender_root, "recorded-cli", generation)?.snapshot()?;
    let expected_body = read_preserves(&fixture.message)?;
    assert!(snapshot.messages.iter().any(|message| message.to == "node:recorded-cli-receiver:inbox"
        && message.body.as_iovalue() == &expected_body));
    let receipt = read_preserves(&committed)?;
    let fields = receipt.collect_simple_record("remote-recorded-session-commit-v1", Some(2))
        .ok_or_else(|| std::io::Error::other("missing receiver commit evidence"))?;
    let observation = fields[0].collect_simple_record("remote-recorded-session-observation-v1", Some(8))
        .ok_or_else(|| std::io::Error::other("missing received-envelope observation"))?;
    let source_refs = observation[7].collect_sequence()
        .ok_or_else(|| std::io::Error::other("missing exact source-envelope binding"))?;
    let observed = source_refs.iter().next().and_then(|value| value.as_string());
    assert_eq!(observed.as_deref(), Some(parsed.envelope_ref.as_str()));
    let state_ref = snapshot.snapshot_ref()?;
    assert_eq!(fields[1].as_string().as_deref(), Some(state_ref.as_str()));
    Ok(envelope)
}

#[test]
fn cli_recorded_remote_session_refuses_unknown_and_revoked_references() -> CliResult<()> {
    // r[verify molten.remote_dataspace.recorded_session_source]
    // r[verify molten.runtime_spine.reference_introduction_rule]
    let fixture = RecordedCliFixture::new()?;
    let admitted = fixture.receiver.with_extension("admitted.preserves");
    assert_success(&fixture.session("admit", &admitted).output()?, "admit actual local recorded session");
    let generation = recorded_cli_generation(&admitted)?;
    let unknown_out = fixture.receiver.with_extension("unknown-envelope.preserves");
    let unknown_receipt = fixture.receiver.with_extension("unknown-denial.preserves");
    assert_failure(&fixture.build(generation, &unknown_out, &unknown_receipt).output()?, "unknown reference build");
    recorded_cli_build_denial(&unknown_receipt, &fixture.reference)?;
    assert!(!unknown_out.exists(), "denied build must not emit an envelope");
    let built = recorded_cli_positive_delivery(&fixture, generation)?;
    let retracted = fixture.receiver.with_extension("retracted.preserves");
    let output = fixture.session("commit", &retracted).arg("--generation").arg(generation.to_string())
        .args(["--operation", "retract", "--payload"]).arg(&fixture.introduction).output()?;
    assert_success(&output, "retract actual introducing assertion");
    let revoked_out = fixture.receiver.with_extension("revoked-envelope.preserves");
    let revoked_receipt = fixture.receiver.with_extension("revoked-denial.preserves");
    assert_failure(&fixture.build(generation, &revoked_out, &revoked_receipt).output()?, "revoked reference build");
    recorded_cli_build_denial(&revoked_receipt, &fixture.reference)?;
    assert!(!revoked_out.exists(), "retracted introduction must not produce an envelope");
    recorded_cli_reject_late_receive(&fixture, generation, &built)?;
    let stale_publication = fixture.receiver.with_extension("stale-publication-denial.preserves");
    assert_failure(&fixture.publish(generation, &built, &stale_publication).output()?, "previously built artifact publication");
    recorded_cli_build_denial(&stale_publication, &fixture.reference)?;
    let closed = fixture.receiver.with_extension("closed.preserves");
    let output = fixture.session("close", &closed).arg("--generation").arg(generation.to_string()).output()?;
    assert_success(&output, "close recorded session generation");
    let readmitted = fixture.receiver.with_extension("readmitted.preserves");
    assert_success(&fixture.session("admit", &readmitted).output()?, "readmit actual local recorded session");
    assert_eq!(recorded_cli_generation(&readmitted)?, generation + 1);
    let stale_out = fixture.receiver.with_extension("stale-generation-envelope.preserves");
    let stale_receipt = fixture.receiver.with_extension("stale-generation-denial.preserves");
    assert_failure(&fixture.build(generation, &stale_out, &stale_receipt).output()?, "old source generation build");
    recorded_cli_build_denial(&stale_receipt, &fixture.reference)?;
    assert!(!stale_out.exists());
    Ok(())
}
