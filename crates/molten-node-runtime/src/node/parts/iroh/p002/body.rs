
impl<'a> FrameEvaluator<'a> {
    fn new(registry: &'a ProtocolRegistry, input: &'a FramedEnvelopeInput) -> Self {
        Self {
            registry,
            input,
            diagnostics: DiagnosticLog::new(),
        }
    }

    fn evaluate(mut self) -> crate::error::Result<FrameEvaluation> {
        self.collect_admission_diagnostics()?;
        let actual_ref = if self.should_parse_payload() {
            Some(self.evaluate_payload()?)
        } else {
            None
        };
        Ok(FrameEvaluation {
            actual_ref,
            diagnostics: self.diagnostics.into_values(),
        })
    }

    fn collect_admission_diagnostics(&mut self) -> crate::error::Result<()> {
        collect_alpn_diagnostic(&self.input.alpn, &mut self.diagnostics).ok();
        validate_text(&self.input.peer, "frame peer", &mut self.diagnostics)?;
        validate_text(&self.input.node, "frame node", &mut self.diagnostics)?;
        validate_text(&self.input.stream_id, "frame stream", &mut self.diagnostics)?;
        collect_ref_diagnostics(
            std::slice::from_ref(&self.input.limit_profile_ref),
            "limit profile",
            &mut self.diagnostics,
        )?;
        collect_ref_diagnostics(
            std::slice::from_ref(&self.input.declared_envelope_ref),
            "declared envelope",
            &mut self.diagnostics,
        )?;
        collect_ref_diagnostics(&self.input.authority_refs, "authority", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.policy_refs, "policy", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.resource_refs, "resource", &mut self.diagnostics)?;
        collect_ref_diagnostics(&self.input.evidence_refs, "evidence", &mut self.diagnostics)?;
        self.collect_registry_diagnostics()?;
        self.collect_limit_diagnostics()
    }

    fn collect_registry_diagnostics(&mut self) -> crate::error::Result<()> {
        if !self.registry.handlers.contains_key(&self.input.alpn) {
            push_diagnostic(&mut self.diagnostics, "unsupported ALPN denied before payload delivery")?;
        }
        Ok(())
    }

    fn collect_limit_diagnostics(&mut self) -> crate::error::Result<()> {
        if self.input.declared_length > self.input.limits.max_frame_bytes {
            push_diagnostic(&mut self.diagnostics, "oversized frame denied before parsing payload")?;
        }
        if self.input.sequence >= self.input.limits.max_frames_per_session {
            push_diagnostic(&mut self.diagnostics, "frame sequence exceeds per-session limit")?;
        }
        if self.input.limits.max_frame_bytes < MIN_FRAME_BYTES || self.input.limits.max_frame_bytes > MAX_FRAME_BYTES {
            push_diagnostic(&mut self.diagnostics, "frame byte limit is outside supported bounds")?;
        }
        if self.input.limits.max_frames_per_session == 0
            || self.input.limits.max_frames_per_session > MAX_SESSION_FRAMES
        {
            push_diagnostic(&mut self.diagnostics, "frame count limit is outside supported bounds")?;
        }
        Ok(())
    }

    fn should_parse_payload(&self) -> bool {
        !self.diagnostics.iter().any(|diagnostic| diagnostic.contains("oversized frame"))
    }

    fn evaluate_payload(&mut self) -> crate::error::Result<String> {
        let byte_len = self.input.envelope_bytes.len() as u64;
        if byte_len != self.input.declared_length {
            push_diagnostic(
                &mut self.diagnostics,
                format!("declared frame length {} does not match bytes {byte_len}", self.input.declared_length),
            )?;
        }
        let parsed = match crate::preserves_rail::parse_canonical_bytes(&self.input.envelope_bytes) {
            Ok(value) => value,
            Err(error) => {
                push_diagnostic(&mut self.diagnostics, format!("malformed Preserves frame: {error}"))?;
                crate::preserves_rail::record("invalid-frame", Vec::new())
            }
        };
        let encoded = crate::preserves_rail::canonical_bytes(&parsed).unwrap_or_default();
        if encoded != self.input.envelope_bytes && !self.input.envelope_bytes.is_empty() {
            push_diagnostic(&mut self.diagnostics, "frame payload is not canonical Preserves bytes")?;
        }
        let computed = crate::preserves_rail::content_ref_from_bytes(&self.input.envelope_bytes);
        if computed != self.input.declared_envelope_ref {
            push_diagnostic(
                &mut self.diagnostics,
                format!(
                    "declared envelope ref mismatch: got {}, expected {}",
                    computed, self.input.declared_envelope_ref
                ),
            )?;
        }
        Ok(computed)
    }
}

pub fn evaluate_framed_envelope(
    registry: &ProtocolRegistry,
    input: &FramedEnvelopeInput,
) -> crate::error::Result<FramedEnvelopeDecision> {
    let evaluation = FrameEvaluator::new(registry, input).evaluate()?;
    let decision = if evaluation.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = framed_receipt_value(FramedReceiptInput {
        decision: &decision,
        alpn: &input.alpn,
        peer: &input.peer,
        node: &input.node,
        stream_id: &input.stream_id,
        sequence: input.sequence,
        declared_length: input.declared_length,
        declared_envelope_ref: &input.declared_envelope_ref,
        actual_envelope_ref: evaluation.actual_ref.as_deref(),
        limit_profile_ref: &input.limit_profile_ref,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        diagnostics: &evaluation.diagnostics,
    })?;
    Ok(FramedEnvelopeDecision {
        decision,
        alpn: input.alpn.clone(),
        peer: input.peer.clone(),
        node: input.node.clone(),
        stream_id: input.stream_id.clone(),
        sequence: input.sequence,
        declared_envelope_ref: input.declared_envelope_ref.clone(),
        actual_envelope_ref: evaluation.actual_ref,
        diagnostics: evaluation.diagnostics,
        receipt_value,
    })
}

pub fn evaluate_service_session(input: &ServiceSessionInput) -> crate::error::Result<ServiceSessionDecision> {
    let mut diagnostics = Vec::new();
    validate_bounded_text(&input.service_id, "service id", MAX_SERVICE_ID_BYTES, &mut diagnostics)?;
    validate_bounded_text(&input.operation_id, "operation id", MAX_OPERATION_ID_BYTES, &mut diagnostics)?;
    validate_interaction_kind(&input.interaction_kind, &mut diagnostics)?;
    validate_path_kind(&input.path_kind, &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.request_ref), "request", &mut diagnostics)?;
    collect_ref_diagnostics(&input.response_refs, "response", &mut diagnostics)?;
    collect_ref_diagnostics(&input.capability_refs, "capability", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    if input.path_kind == "remote" {
        if input.alpn.as_deref().is_none() || input.peer.as_deref().is_none() || input.node.as_deref().is_none() {
            push_diagnostic(&mut diagnostics, "remote session requires ALPN, peer, and node ids")?;
        }
        if input.frame_receipt_refs.is_empty() {
            push_diagnostic(&mut diagnostics, "remote session requires frame receipt refs")?;
        }
    }
    collect_ref_diagnostics(&input.frame_receipt_refs, "frame receipt", &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let receipt_value = service_session_receipt_value(input, &decision, &diagnostics)?;
    Ok(ServiceSessionDecision {
        decision,
        service_id: input.service_id.clone(),
        operation_id: input.operation_id.clone(),
        interaction_kind: input.interaction_kind.clone(),
        path_kind: input.path_kind.clone(),
        diagnostics,
        receipt_value,
    })
}

pub fn network_diagnostics_report(input: &NetworkDiagnosticsInput) -> crate::error::Result<DiagnosticDecision> {
    let mut diagnostics = input.diagnostics.clone();
    ensure_string_count(&diagnostics, MAX_DIAGNOSTICS, "network diagnostics")?;
    validate_status(&input.udp_status, &["pass", "deny", "degraded", "unavailable"], "UDP status")?;
    validate_status(
        &input.direct_path_status,
        &["pass", "deny", "degraded", "relay-only", "unavailable"],
        "direct path status",
    )?;
    validate_bounded_value_count(input.port_map_protocols.len(), MAX_NETWORK_OBSERVATIONS, "port map protocol")?;
    collect_ref_diagnostics(&input.interface_refs, "interface snapshot", &mut diagnostics)?;
    collect_ref_diagnostics(&input.route_refs, "route snapshot", &mut diagnostics)?;
    if !input.live_observations_recorded {
        push_diagnostic(&mut diagnostics, "live-only observations are non-replayable diagnostics")?;
    }
    let decision = if input.udp_status == "deny" || input.direct_path_status == "deny" {
        "deny"
    } else if diagnostics.is_empty() && input.udp_status == "pass" && input.direct_path_status == "pass" {
        "pass"
    } else {
        "degraded"
    }
    .to_string();
    let receipt_value = crate::preserves_rail::record("network-diagnostics-report-v1", vec![
        crate::preserves_rail::string(NETWORK_DIAGNOSTICS_REPORT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("nat", vec![crate::preserves_rail::string(&input.nat_class)]),
        crate::preserves_rail::record("udp", vec![crate::preserves_rail::string(&input.udp_status)]),
        crate::preserves_rail::record("direct-path", vec![crate::preserves_rail::string(&input.direct_path_status)]),
        crate::preserves_rail::record("relay-latency-ms", vec![optional_u64_value(input.relay_latency_ms)]),
        crate::preserves_rail::record("port-map-protocols", vec![crate::preserves_rail::sequence(
            input.port_map_protocols.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("interfaces", vec![refs_value(&input.interface_refs)?]),
        crate::preserves_rail::record("routes", vec![refs_value(&input.route_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("diagnostics-evidence-only", "pass"),
            ("live-observations-recorded", pass_fail(input.live_observations_recorded)),
            ("no-transport-derived-authority", "pass"),
        ]),
        crate::preserves_rail::record("caveat", vec![crate::preserves_rail::string(EVIDENCE_ONLY_CAVEAT)]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}
