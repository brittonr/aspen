
struct RouterReceiptInput<'a> {
    decision: &'a str,
    operation: &'a str,
    outcome: &'a str,
    alpn: &'a str,
    handler_kind: &'a str,
    generation: Option<u64>,
    previous_generation: Option<u64>,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
    shutdown_evidence_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

fn router_receipt_value(input: RouterReceiptInput<'_>) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("iroh-protocol-router-receipt-v1", vec![
        crate::preserves_rail::string(IROH_PROTOCOL_ROUTER_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("outcome", vec![crate::preserves_rail::string(input.outcome)]),
        crate::preserves_rail::record("alpn", vec![crate::preserves_rail::string(input.alpn)]),
        crate::preserves_rail::record("handler", vec![crate::preserves_rail::string(input.handler_kind)]),
        crate::preserves_rail::record("generation", vec![optional_u64_value(input.generation)]),
        crate::preserves_rail::record("previous-generation", vec![optional_u64_value(input.previous_generation)]),
        crate::preserves_rail::record("authority", vec![refs_value(input.authority_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(input.policy_refs)?]),
        crate::preserves_rail::record("resource", vec![refs_value(input.resource_refs)?]),
        crate::preserves_rail::record("evidence", vec![refs_value(input.evidence_refs)?]),
        crate::preserves_rail::record("shutdown-evidence", vec![optional_string_value(input.shutdown_evidence_ref)]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("authority-policy-resource-explicit", pass_fail(input.decision == "pass")),
            ("generationed-router", "pass"),
            ("unsupported-alpn-denies-before-delivery", pass_fail(input.outcome != "unsupported-alpn")),
            ("transport-evidence-only", "pass"),
        ]),
    ]))
}

struct FramedReceiptInput<'a> {
    decision: &'a str,
    alpn: &'a str,
    peer: &'a str,
    node: &'a str,
    stream_id: &'a str,
    sequence: u64,
    declared_length: u64,
    declared_envelope_ref: &'a str,
    actual_envelope_ref: Option<&'a str>,
    limit_profile_ref: &'a str,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

fn framed_receipt_value(input: FramedReceiptInput<'_>) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("iroh-framed-envelope-receipt-v1", vec![
        crate::preserves_rail::string(IROH_FRAMED_ENVELOPE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("alpn", vec![crate::preserves_rail::string(input.alpn)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node)]),
        crate::preserves_rail::record("stream", vec![crate::preserves_rail::string(input.stream_id)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(input.sequence)]),
        crate::preserves_rail::record("length", vec![crate::preserves_rail::u64_value(input.declared_length)]),
        crate::preserves_rail::record("declared-envelope", vec![crate::preserves_rail::string(
            input.declared_envelope_ref,
        )]),
        crate::preserves_rail::record("actual-envelope", vec![optional_string_value(input.actual_envelope_ref)]),
        crate::preserves_rail::record("limit-profile", vec![crate::preserves_rail::string(input.limit_profile_ref)]),
        crate::preserves_rail::record("authority", vec![refs_value(input.authority_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(input.policy_refs)?]),
        crate::preserves_rail::record("resource", vec![refs_value(input.resource_refs)?]),
        crate::preserves_rail::record("evidence", vec![refs_value(input.evidence_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(input.diagnostics)?]),
        checks_value(&[
            ("canonical-preserves-frame", pass_fail(input.decision == "pass")),
            ("frame-limits-bound", pass_fail(input.decision == "pass")),
            ("declared-ref-matches-actual", pass_fail(input.decision == "pass")),
            ("transport-evidence-only", "pass"),
        ]),
    ]))
}

fn service_session_receipt_value(
    input: &ServiceSessionInput,
    decision: &str,
    diagnostics: &[String],
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("iroh-stream-session-receipt-v1", vec![
        crate::preserves_rail::string(IROH_STREAM_SESSION_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("service", vec![crate::preserves_rail::string(&input.service_id)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.operation_id)]),
        crate::preserves_rail::record("interaction", vec![crate::preserves_rail::string(&input.interaction_kind)]),
        crate::preserves_rail::record("path", vec![crate::preserves_rail::string(&input.path_kind)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request_ref)]),
        crate::preserves_rail::record("responses", vec![refs_value(&input.response_refs)?]),
        crate::preserves_rail::record("capability", vec![refs_value(&input.capability_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(&input.policy_refs)?]),
        crate::preserves_rail::record("resource", vec![refs_value(&input.resource_refs)?]),
        crate::preserves_rail::record("alpn", vec![optional_string_value(input.alpn.as_deref())]),
        crate::preserves_rail::record("peer", vec![optional_string_value(input.peer.as_deref())]),
        crate::preserves_rail::record("node", vec![optional_string_value(input.node.as_deref())]),
        crate::preserves_rail::record("stream", vec![optional_string_value(input.stream_id.as_deref())]),
        crate::preserves_rail::record("frames", vec![refs_value(&input.frame_receipt_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(diagnostics)?]),
        checks_value(&[
            ("canonical-local-remote-model", pass_fail(decision == "pass")),
            (
                "remote-frames-bound",
                pass_fail(input.path_kind != "remote" || !input.frame_receipt_refs.is_empty()),
            ),
            ("postcard-not-canonical-boundary", "pass"),
        ]),
    ]))
}

fn descriptor_from_input(input: &RouterOperationInput) -> crate::error::Result<ProtocolHandlerDescriptor> {
    Ok(ProtocolHandlerDescriptor {
        alpn: input.alpn.clone(),
        handler_kind: input.handler_kind.clone(),
        generation: input.generation,
        authority_refs: input.authority_refs.clone(),
        policy_refs: input.policy_refs.clone(),
        resource_refs: input.resource_refs.clone(),
        evidence_refs: input.evidence_refs.clone(),
        drain_policy: DEFAULT_ROUTER_DRAIN_POLICY.to_string(),
    })
}

fn collect_alpn_diagnostic(alpn: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    validate_bounded_text(alpn, "ALPN", MAX_ALPN_BYTES, diagnostics)?;
    if alpn.bytes().all(is_alpn_byte) {
        Ok(())
    } else {
        push_diagnostic(diagnostics, "ALPN must use visible ASCII without spaces")
    }
}

fn collect_handler_diagnostic(handler: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    validate_bounded_text(handler, "handler kind", MAX_HANDLER_KIND_BYTES, diagnostics)
}

fn validate_interaction_kind(kind: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    if matches!(kind, "unary" | "server-streaming" | "client-streaming" | "bidirectional-streaming") {
        Ok(())
    } else {
        push_diagnostic(diagnostics, format!("unsupported service interaction kind {kind}"))
    }
}

fn validate_path_kind(kind: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    if matches!(kind, "local" | "remote") {
        Ok(())
    } else {
        push_diagnostic(diagnostics, format!("unsupported service session path {kind}"))
    }
}

fn validate_status(value: &str, allowed: &[&str], label: &str) -> crate::error::Result<()> {
    if allowed.iter().any(|allowed| allowed == &value) {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("unsupported {label} {value}")))
    }
}

fn validate_port(value: Option<u64>, label: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    match value {
        Some(port) if (MIN_PORT_NUMBER..=MAX_PORT_NUMBER).contains(&port) => Ok(()),
        Some(port) => push_diagnostic(diagnostics, format!("{label} {port} outside valid port range")),
        None => push_diagnostic(diagnostics, format!("{label} is required")),
    }
}

fn collect_required_optional_ref(
    value: Option<&str>,
    label: &str,
    diagnostics: &mut impl DiagnosticSink,
) -> crate::error::Result<()> {
    match value {
        Some(reference) => collect_ref_diagnostics(&[reference.to_string()], label, diagnostics),
        None => push_diagnostic(diagnostics, format!("{label} ref is required")),
    }
}

fn collect_ref_diagnostics(
    refs: &[String],
    label: &str,
    diagnostics: &mut impl DiagnosticSink,
) -> crate::error::Result<()> {
    validate_bounded_value_count(refs.len(), MAX_REF_COUNT, label)?;
    for reference in refs {
        if let Err(error) = crate::preserves_rail::validate_content_ref(reference) {
            push_diagnostic(diagnostics, format!("invalid {label} ref {reference}: {error}"))?;
        }
    }
    Ok(())
}

fn validate_optional_ref(value: Option<&str>, label: &str) -> crate::error::Result<()> {
    if let Some(reference) = value {
        crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_bounded_text(
    value: &str,
    label: &str,
    maximum: usize,
    diagnostics: &mut impl DiagnosticSink,
) -> crate::error::Result<()> {
    if value.trim().is_empty() {
        return push_diagnostic(diagnostics, format!("{label} must not be empty"));
    }
    if value.len() > maximum {
        return push_diagnostic(diagnostics, format!("{label} length {} exceeds bound {maximum}", value.len()));
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, diagnostics: &mut impl DiagnosticSink) -> crate::error::Result<()> {
    validate_bounded_text(value, label, MAX_SERVICE_ID_BYTES, diagnostics)
}

fn validate_bounded_value_count(actual: usize, maximum: usize, label: &str) -> crate::error::Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} count {actual} exceeds bound {maximum}"
        )))
    }
}

fn ensure_string_count(values: &[String], maximum: usize, label: &str) -> crate::error::Result<()> {
    validate_bounded_value_count(values.len(), maximum, label)
}

fn push_diagnostic(diagnostics: &mut impl DiagnosticSink, diagnostic: impl Into<String>) -> crate::error::Result<()> {
    diagnostics.push_bounded(diagnostic.into())
}

fn is_alpn_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')
}

fn refs_value(refs: &[String]) -> crate::error::Result<preserves::IOValue> {
    validate_bounded_value_count(refs.len(), MAX_REF_COUNT, "ref")?;
    Ok(crate::preserves_rail::sequence(refs.iter().map(crate::preserves_rail::string).collect()))
}

fn strings_value(values: &[String]) -> crate::error::Result<preserves::IOValue> {
    ensure_string_count(values, MAX_DIAGNOSTICS, "string")?;
    Ok(crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect()))
}

fn optional_string_value(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_u64_value(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&'static str, &'static str)]) -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|(name, status)| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(name),
                    crate::preserves_rail::string(status),
                ])
            })
            .collect(),
    )])
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}
