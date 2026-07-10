fn topology_link_values(links: &[TopologyLink]) -> Vec<IoValue> {
    links
        .iter()
        .map(|link_item| {
            record("link", vec![
                record("from", vec![string(&link_item.from)]),
                record("to", vec![string(&link_item.to)]),
                record("topic", vec![string(&link_item.topic)]),
            ])
        })
        .collect()
}

fn node_summary_values(summaries: &[NodeSummary]) -> Vec<IoValue> {
    summaries
        .iter()
        .map(|summary| {
            record("node-summary", vec![
                record("node", vec![string(&summary.node_id)]),
                record("topology", vec![string(&summary.topology_ref)]),
                record("scenario-fixture", vec![string(&summary.scenario_fixture_ref)]),
                record("receipts", vec![refs_sequence(&summary.receipt_refs)]),
                record("queue", vec![string(&summary.queue_ref)]),
                record("ledger", vec![string(&summary.ledger_ref)]),
                record("dispatch", vec![string(&summary.dispatch_ref)]),
                record("ack", vec![string(&summary.ack_ref)]),
                record("protocol", vec![string(&summary.protocol_ref)]),
                record("commits", vec![sequence(semantic_commit_values(&summary.semantic_commits))]),
                record("logs", vec![refs_sequence(&summary.diagnostic_log_refs)]),
            ])
        })
        .collect()
}

fn semantic_commit_values(commits: &[SemanticCommitEvidence]) -> Vec<IoValue> {
    commits
        .iter()
        .map(|commit| {
            record("commit", vec![
                record("operation", vec![string(&commit.operation_id)]),
                record("ref", vec![string(&commit.commit_ref)]),
            ])
        })
        .collect()
}

fn equality_class_values(classes: &[ReconciliationEqualityClass]) -> Vec<IoValue> {
    classes
        .iter()
        .map(|class| {
            record("equality", vec![
                record("name", vec![string(&class.name)]),
                record("refs", vec![refs_sequence(&class.refs)]),
                record("variance", vec![optional_ref_value(class.variance_ref.as_deref())]),
            ])
        })
        .collect()
}

fn local_process_node_values(nodes: &[LocalProcessNodePlan]) -> Vec<IoValue> {
    nodes
        .iter()
        .map(|node| {
            record("node", vec![
                record("id", vec![string(&node.node_id)]),
                record("state-root", vec![string(&node.state_root_handle)]),
                record("transport", vec![string(&node.transport_handle)]),
            ])
        })
        .collect()
}

fn role(node_id: &str, role_name: &str, membership: &str) -> TopologyRole {
    TopologyRole {
        node_id: node_id.to_string(),
        role: role_name.to_string(),
        membership: membership.to_string(),
    }
}

fn link(from: &str, to: &str, topic: &str) -> TopologyLink {
    TopologyLink {
        from: from.to_string(),
        to: to.to_string(),
        topic: topic.to_string(),
    }
}

fn live_transport_refs(input: &LiveTransportVmEvidenceInput) -> Vec<String> {
    vec![
        input.ticket_ref.clone(),
        input.peer_admission_ref.clone(),
        input.authority_ref.clone(),
        input.send_ref.clone(),
        input.receive_ref.clone(),
        input.ingress_ref.clone(),
        input.queue_ref.clone(),
        input.dispatch_ref.clone(),
        input.reconcile_ref.clone(),
        input.ack_ref.clone(),
        input.protocol_gate_ref.clone(),
    ]
}

fn collect_required_text_diagnostic(label: &str, value: &str, diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    if value.trim().is_empty() {
        push_diagnostic(diagnostics, format!("missing-{label}"))?;
    }
    Ok(())
}

fn collect_invalid_ref_diagnostics(label: &str, refs: &[String], diagnostics: &mut impl DiagnosticSink) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_MULTINODE_ITEMS, label)?;
    for reference in refs {
        if crate::preserves_rail::validate_content_ref(reference).is_err() {
            push_diagnostic(diagnostics, format!("invalid-{label}-ref"))?;
        }
    }
    Ok(())
}

fn collect_invalid_optional_ref_diagnostics(
    label: &str,
    reference: Option<&str>,
    diagnostics: &mut impl DiagnosticSink,
) -> Result<()> {
    if let Some(reference) = reference
        && crate::preserves_rail::validate_content_ref(reference).is_err()
    {
        push_diagnostic(diagnostics, format!("invalid-{label}-ref"))?;
    }
    Ok(())
}

fn push_if(diagnostics: &mut impl DiagnosticSink, condition: bool, diagnostic: &'static str) -> Result<()> {
    if condition {
        push_diagnostic(diagnostics, diagnostic.to_string())?;
    }
    Ok(())
}

fn push_diagnostic(diagnostics: &mut impl DiagnosticSink, diagnostic: String) -> Result<()> {
    diagnostics.push_bounded(diagnostic)
}

trait DiagnosticSink {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()>;
}

impl DiagnosticSink for Vec<String> {
    fn push_bounded(&mut self, diagnostic: String) -> Result<()> {
        if self.len() >= MAX_MULTINODE_ITEMS {
            return Err(MoltenError::invalid_harness("multinode diagnostics exceeded bound"));
        }
        self.push(diagnostic);
        Ok(())
    }
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds bound {maximum}")))
    }
}

fn decision_from_diagnostics(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
}

fn status(condition: bool) -> &'static str {
    if condition { PASS_DECISION } else { DENY_DECISION }
}

fn content_ref_from_text(value: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(value.as_bytes())
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(reference) => record("some", vec![string(reference)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

