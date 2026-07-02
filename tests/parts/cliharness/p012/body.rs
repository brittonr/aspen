
fn write_status_request(
    path: &std::path::Path,
    authority_ref: &str,
    policy_ref: &str,
    resource_ref: &str,
    label: &str,
) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-request",
                "--operation",
                "status",
                "--authority",
            ])
            .arg(authority_ref)
            .args(["--policy"])
            .arg(policy_ref)
            .args(["--resource"])
            .arg(resource_ref)
            .args(["--out"])
            .arg(path)
            .output()?,
        label,
    );
    Ok(())
}

struct GrantArgs<'a> {
    root: &'a std::path::Path,
    grant: &'a std::path::Path,
    peer: &'a str,
    node: &'a str,
    policy_ref: &'a str,
    label: &'a str,
}

fn grant_fixture(args: GrantArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "authority-grant-fixture", "--state-root"])
            .arg(args.root)
            .args([
                "--peer",
                args.peer,
                "--node",
                args.node,
                "--operation",
                "status",
                "--policy",
            ])
            .arg(args.policy_ref)
            .args(["--out"])
            .arg(args.grant)
            .output()?,
        args.label,
    );
    Ok(molten::preserves_rail::canonical_hash(&read_preserves(args.grant)?)?)
}

fn ticket_export(root: &std::path::Path, ticket: &std::path::Path, policy_ref: &str, label: &str) -> CliResult<()> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-ticket-export", "--state-root"])
            .arg(root)
            .args(["--policy"])
            .arg(policy_ref)
            .args(["--out"])
            .arg(ticket)
            .output()?,
        label,
    );
    Ok(())
}

struct AdmitArgs<'a> {
    root: &'a std::path::Path,
    receipt: &'a std::path::Path,
    peer: &'a str,
    policy_ref: &'a str,
    ticket: &'a std::path::Path,
    label: &'a str,
}

fn peer_admit(args: AdmitArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args(["test", "node", "live-peer-admit", "--state-root"])
            .arg(args.root)
            .args(["--peer", args.peer, "--policy"])
            .arg(args.policy_ref)
            .args(["--receipt-out"])
            .arg(args.receipt)
            .arg(args.ticket)
            .output()?,
        args.label,
    );
    Ok(molten::preserves_rail::canonical_hash(&read_preserves(args.receipt)?)?)
}

struct SendArgs<'a> {
    root: &'a std::path::Path,
    request: &'a std::path::Path,
    ticket: &'a std::path::Path,
    peer: &'a str,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

fn send_cmd(args: &SendArgs<'_>) -> std::process::Command {
    let mut command = molten_cmd();
    command
        .args(["test", "node", "control-ingress-live-send", "--state-root"])
        .arg(args.root)
        .arg(args.request)
        .arg(args.ticket)
        .args(["--from-peer", args.peer, "--peer-bootstrap"])
        .arg(args.bootstrap_ref)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref);
    command
}

fn expect_no_address(
    args: &SendArgs<'_>,
    transport_receipt: &std::path::Path,
    receipt: &std::path::Path,
) -> CliResult<()> {
    let sent = send_cmd(args)
        .args(["--transport-receipt-out"])
        .arg(transport_receipt)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&sent, "node live send deny no address");
    assert!(stdout(&sent).contains("transport_receipt=none"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(receipt)?), "node-control-live-send-receipt");
    assert!(!transport_receipt.exists());
    let text = molten::preserves_rail::to_text(&read_preserves(receipt)?)?;
    assert!(text.contains("ticket has no endpoint addresses"));
    Ok(())
}

fn expect_mismatch(args: &SendArgs<'_>, operation_ref: &str, receipt: &std::path::Path) -> CliResult<()> {
    let output = send_cmd(args)
        .args(["--operation-id"])
        .arg(operation_ref)
        .args(["--receipt-out"])
        .arg(receipt)
        .output()?;
    assert_success(&output, "node live send deny operation mismatch");
    let text = molten::preserves_rail::to_text(&read_preserves(receipt)?)?;
    assert!(text.contains("operation-id"));
    Ok(())
}

struct BundleArgs<'a> {
    root: &'a std::path::Path,
    ticket: &'a std::path::Path,
    peer_admission: &'a std::path::Path,
    authority_grant: &'a std::path::Path,
    send_receipt: &'a std::path::Path,
    service_receipt: &'a std::path::Path,
    receipt: &'a std::path::Path,
}

fn expect_missing_receive(args: BundleArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "live-workflow-bundle", "--state-root"])
        .arg(args.root)
        .args(["--ticket"])
        .arg(args.ticket)
        .args(["--peer-admission"])
        .arg(args.peer_admission)
        .args(["--authority-grant"])
        .arg(args.authority_grant)
        .args(["--send-receipt"])
        .arg(args.send_receipt)
        .args(["--service-receipt"])
        .arg(args.service_receipt)
        .args(["--receipt-out"])
        .arg(args.receipt)
        .output()?;
    assert_success(&output, "node live workflow bundle deny");
    assert!(stdout(&output).contains("decision=deny"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.receipt)?), "node-control-live-workflow-receipt");
    let text = molten::preserves_rail::to_text(&read_preserves(args.receipt)?)?;
    assert!(text.contains("missing receive receipt"));
    Ok(())
}

struct LoopbackArgs<'a> {
    root: &'a std::path::Path,
    request: &'a std::path::Path,
    publish: &'a std::path::Path,
    receive: &'a std::path::Path,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
}

fn run_loopback(args: LoopbackArgs<'_>) -> CliResult<()> {
    let output = molten_cmd()
        .args(["test", "node", "control-ingress-live-loopback", "--state-root"])
        .arg(args.root)
        .args([
            "--from-peer",
            "peer:cli-live",
            "--to-node",
            "node:cli-live",
            "--peer-bootstrap",
        ])
        .arg(args.bootstrap_ref)
        .args(["--authority"])
        .arg(args.authority_ref)
        .args(["--policy"])
        .arg(args.policy_ref)
        .args(["--resource"])
        .arg(args.resource_ref)
        .args(["--publish-receipt-out"])
        .arg(args.publish)
        .args(["--receive-receipt-out"])
        .arg(args.receive)
        .arg(args.request)
        .output()?;
    assert_success(&output, "node live ingress loopback");
    assert!(stdout(&output).contains("enqueued=yes"));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.publish)?), "node-control-live-transport-receipt");
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(args.receive)?), "node-control-live-transport-receipt");
    Ok(())
}

struct EnvelopeArgs<'a> {
    path: &'a std::path::Path,
    request: &'a std::path::Path,
    bootstrap_ref: &'a str,
    authority_ref: &'a str,
    policy_ref: &'a str,
    resource_ref: &'a str,
    label: &'a str,
}

fn write_envelope(args: EnvelopeArgs<'_>) -> CliResult<String> {
    assert_success(
        &molten_cmd()
            .args([
                "test",
                "node",
                "control-ingress-build",
                "--from-peer",
                "peer:cli",
                "--to-node",
                "node:cli-ingress",
                "--peer-bootstrap",
            ])
            .arg(args.bootstrap_ref)
            .args(["--authority"])
            .arg(args.authority_ref)
            .args(["--policy"])
            .arg(args.policy_ref)
            .args(["--resource"])
            .arg(args.resource_ref)
            .args(["--out"])
            .arg(args.path)
            .arg(args.request)
            .output()?,
        args.label,
    );
    let value = read_preserves(args.path)?;
    assert_eq!(molten::ledger::artifact_kind(&value), "node-control-ingress-envelope");
    Ok(molten::preserves_rail::canonical_hash(&value)?)
}
