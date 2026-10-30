// Outbound envelope construction and reference-introduction rechecks.

pub enum EnvelopeBuild {
    Built(Box<Envelope>),
    Denied { receipt_value: IoValue, diagnostics: Vec<String> },
}

/// Build using the exact current receiver-owned view. Without a view only
/// reference-free messages (or introducing non-message operations) can build.
pub fn build_envelope(input: EnvelopeInput, session: Option<&SenderSessionView<'_>>) -> Result<Envelope> {
    match build_envelope_with_evidence(input, session)? {
        EnvelopeBuild::Built(envelope) => Ok(*envelope),
        EnvelopeBuild::Denied { receipt_value, .. } => Err(MoltenError::invalid_harness(format!(
            "remote envelope build denied: {}", crate::preserves_rail::to_text(&receipt_value)?
        ))),
    }
}

// r[impl molten.runtime_spine.reference_introduction_rule]
pub fn build_envelope_with_evidence(input: EnvelopeInput, session: Option<&SenderSessionView<'_>>) -> Result<EnvelopeBuild> {
    validate_name(&input.from_peer, "from peer")?;
    validate_name(&input.from_actor, "from actor")?;
    validate_name(&input.to_peer, "to peer")?;
    validate_name(&input.topic, "topic")?;
    validate_refs(&input.content_refs, "content ref")?;
    validate_refs(&input.capability_refs, "capability ref")?;
    validate_refs(&input.evidence_refs, "evidence ref")?;
    let diagnostics = outbound_reference_diagnostics(OutboundReferenceInput {
        from_peer: &input.from_peer, to_peer: &input.to_peer, topic: &input.topic, operation: input.operation,
        payload: &input.payload, content_refs: &input.content_refs, session,
    })?;
    if !diagnostics.is_empty() {
        let receipt_value = outbound_denial_value(&input.from_peer, &input.to_peer, &input.topic,
            &canonical_hash(&envelope_value(&input)?)?, &diagnostics);
        return Ok(EnvelopeBuild::Denied { receipt_value, diagnostics });
    }
    let value = envelope_value(&input)?;
    Ok(EnvelopeBuild::Built(Box::new(parse_envelope(&value)?)))
}

/// The routed message fields and live sender view an outbound reference check reads.
struct OutboundReferenceInput<'a> {
    from_peer: &'a str,
    to_peer: &'a str,
    topic: &'a str,
    operation: Operation,
    payload: &'a IoValue,
    content_refs: &'a [String],
    session: Option<&'a SenderSessionView<'a>>,
}

fn outbound_reference_diagnostics(input: OutboundReferenceInput<'_>) -> Result<Vec<String>> {
    let OutboundReferenceInput { from_peer, to_peer, topic, operation, payload, content_refs, session } = input;
    let mut diagnostics = Vec::new();
    if operation == Operation::Message {
        let carried = message_references(payload, content_refs)?;
        let introduced = match session {
            Some(view) => match view.introduced_for(from_peer, to_peer, topic) {
                Ok(introduced) => introduced,
                Err(error) => {
                    diagnostics.push(error.to_string());
                    IntroducedReferences::default()
                }
            },
            None => IntroducedReferences::default(),
        };
        diagnostics.extend(introduced.unknown(&carried).into_iter()
            .map(|reference| format!("message carries unintroduced reference {reference}")));
    }
    Ok(diagnostics)
}

fn outbound_denial_value(from_peer: &str, to_peer: &str, topic: &str, envelope_ref: &str, diagnostics: &[String]) -> IoValue {
    record("remote-dataspace-envelope-build-denial-v1", vec![
        string("molten.remote-dataspace.envelope-build-denial.v1"), string("deny"),
        string(from_peer), string(to_peer), string(topic), string(envelope_ref),
        sequence(diagnostics.iter().map(string).collect()),
    ])
}

/// Recheck imported or previously built artifacts immediately before publication.
/// A successful earlier build is not a source-currentness or publication receipt.
pub fn outbound_envelope_denial(envelope: &Envelope, session: Option<&SenderSessionView<'_>>) -> Result<Option<IoValue>> {
    let parsed = parse_envelope(&envelope.value)?;
    let diagnostics = if parsed != *envelope {
        vec!["envelope fields differ from canonical wire value".to_owned()]
    } else {
        outbound_reference_diagnostics(OutboundReferenceInput {
            from_peer: &parsed.from_peer, to_peer: &parsed.to_peer, topic: &parsed.topic, operation: parsed.operation,
            payload: &parsed.payload, content_refs: &parsed.content_refs, session,
        })?
    };
    Ok((!diagnostics.is_empty()).then(|| outbound_denial_value(&parsed.from_peer, &parsed.to_peer,
        &parsed.topic, &parsed.envelope_ref, &diagnostics)))
}

fn require_outbound_envelope(envelope: &Envelope, session: Option<&SenderSessionView<'_>>) -> Result<()> {
    if let Some(receipt) = outbound_envelope_denial(envelope, session)? {
        return Err(MoltenError::invalid_harness(format!("remote envelope publication denied: {}",
            crate::preserves_rail::to_text(&receipt)?)));
    }
    Ok(())
}

pub fn assert_envelope(input: AssertEnvelopeInput<'_>) -> Result<Envelope> {
    build_envelope(EnvelopeInput {
        from_peer: input.from_peer.to_owned(),
        from_actor: input.from_actor.to_owned(),
        to_peer: input.to_peer.to_owned(),
        topic: input.topic.to_owned(),
        operation: Operation::Assert,
        payload: input.payload,
        content_refs: Vec::new(),
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
    }, None)
}

/// Construct untrusted incoming wire data for receiver tests. Never a sender
/// admission path; deliberately absent from production builds.
#[cfg(test)]
pub(crate) fn incoming_envelope_fixture(input: EnvelopeInput) -> Result<Envelope> {
    parse_envelope(&envelope_value(&input)?)
}
