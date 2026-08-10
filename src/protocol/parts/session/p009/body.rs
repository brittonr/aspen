
const PROTOCOL_FACADE_GENERATION_RECEIPT_SCHEMA: &str = "molten.protocol.facade-generation-receipt.v1";
const PROTOCOL_FACADE_TRANSITION_SCHEMA: &str = "molten.protocol.facade-transition.v1";
const PROTOCOL_FACADE_GENERATION_RECEIPT_FIELDS: usize = 13;
const FACADE_REF_CAPACITY_OVERFLOW: &str = "protocol facade receipt input ref capacity overflow";

const FACADE_NON_CLAIM_AUTHORITY: &str = "no-authority-grant";
const FACADE_NON_CLAIM_POLICY: &str = "no-policy-admission";
const FACADE_NON_CLAIM_RESOURCE: &str = "no-resource-grant";
const FACADE_NON_CLAIM_PROVENANCE: &str = "no-provenance-approval";
const FACADE_NON_CLAIM_TRANSPORT: &str = "no-transport-trust";
const FACADE_NON_CLAIM_CHORUS: &str = "no-chorus-compatibility";
const FACADE_NON_CLAIM_JSON: &str = "no-serde-json-protocol-identity";

const PROTOCOL_FACADE_FORBIDDEN_DEPENDENCY_MARKERS: &[&str] = &[
    "chorus_lib",
    "chorus-http",
    "chorus_http",
    "chorus-local",
    "chorus_local",
    "chorus_transport",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFacadeGenerationInput {
    pub install_receipt: IoValue,
    pub generator_ref: String,
    pub artifact_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFacadeGenerationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub install_ref: String,
    pub role_registry_ref: String,
    pub label_registry_ref: String,
    pub payload_registry_ref: String,
    pub endpoint_refs: Vec<String>,
    pub generator_ref: String,
    pub artifact_ref: String,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFacadeTransitionInput {
    pub operation: String,
    pub state: IoValue,
    pub peer: Option<String>,
    pub label: String,
    pub payload_tag: Option<String>,
    pub body_or_ref: Option<IoValue>,
    pub message: Option<IoValue>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFacadeTransition {
    pub decision: String,
    pub operation: String,
    pub message_descriptor: Option<ProtocolMessage>,
    pub next_state: Option<ProtocolSessionState>,
    pub receipt_input_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocatedPayload {
    pub owner_role: String,
    pub payload_tag: String,
    pub payload_ref: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ProtocolFacadePayloadAccessInput<'a> {
    pub payload: &'a ProtocolLocatedPayload,
    pub local_role: &'a str,
    pub expected_payload_tag: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolFacadePayloadAccessDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub payload_ref: Option<String>,
}

struct ProtocolFacadeGenerationValueInput<'a> {
    decision: &'a str,
    manifest_ref: &'a str,
    install_ref: &'a str,
    role_registry_ref: &'a str,
    label_registry_ref: &'a str,
    payload_registry_ref: &'a str,
    endpoint_refs: &'a [String],
    generator_ref: &'a str,
    artifact_ref: &'a str,
    diagnostics: &'a [String],
    non_claims: &'a [String],
}

struct ProtocolFacadeTransitionValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    protocol_ref: &'a str,
    session_id: &'a str,
    role: &'a str,
    prior_state_ref: &'a str,
    message_ref: Option<&'a str>,
    next_state_ref: Option<&'a str>,
    receipt_input_refs: &'a [String],
    diagnostics: &'a [String],
}

pub fn generate_protocol_facade_receipt(
    input: ProtocolFacadeGenerationInput,
) -> Result<ProtocolFacadeGenerationReceipt> {
    require_ref(&input.generator_ref, "protocol facade generator ref")?;
    require_ref(&input.artifact_ref, "protocol facade artifact ref")?;
    let install = parse_protocol_install_receipt(&input.install_receipt)?;
    let role_registry_ref = canonical_hash(&registry_value("role-registry", &install.registries.roles))?;
    let label_registry_ref = canonical_hash(&registry_value("label-registry", &install.registries.labels))?;
    let payload_registry_ref = canonical_hash(&registry_value("payload-registry", &install.registries.payloads))?;
    let endpoint_refs = install_endpoint_refs(&install);
    let mut diagnostics = Vec::new();
    if install.decision != "pass" {
        diagnostics.push("protocol facade generation requires a passing projectability install receipt".to_string());
    }
    if endpoint_refs.is_empty() {
        diagnostics.push("protocol facade generation requires projected endpoint refs".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let non_claims = protocol_facade_non_claims();
    let value = protocol_facade_generation_receipt_value(&ProtocolFacadeGenerationValueInput {
        decision,
        manifest_ref: &install.manifest.manifest_ref,
        install_ref: &install.receipt_ref,
        role_registry_ref: &role_registry_ref,
        label_registry_ref: &label_registry_ref,
        payload_registry_ref: &payload_registry_ref,
        endpoint_refs: &endpoint_refs,
        generator_ref: &input.generator_ref,
        artifact_ref: &input.artifact_ref,
        diagnostics: &diagnostics,
        non_claims: &non_claims,
    })?;
    Ok(ProtocolFacadeGenerationReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        manifest_ref: install.manifest.manifest_ref,
        install_ref: install.receipt_ref,
        role_registry_ref,
        label_registry_ref,
        payload_registry_ref,
        endpoint_refs,
        generator_ref: input.generator_ref,
        artifact_ref: input.artifact_ref,
        diagnostics,
        non_claims,
        value,
    })
}

pub fn parse_protocol_facade_generation_receipt(value: &IoValue) -> Result<ProtocolFacadeGenerationReceipt> {
    let fields = value
        .collect_simple_record(
            "protocol-facade-generation-receipt-v1",
            Some(PROTOCOL_FACADE_GENERATION_RECEIPT_FIELDS),
        )
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-facade-generation-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_FACADE_GENERATION_RECEIPT_SCHEMA, "protocol facade generation schema")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "facade-non-authority", "protocol facade generation receipt")?;
    require_check(&checks, "no-chorus-compatibility", "protocol facade generation receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_gate_decision(&decision, "protocol facade generation decision")?;
    let endpoint_refs = parse_ref_sequence(&fields[7], "endpoints")?;
    Ok(ProtocolFacadeGenerationReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        manifest_ref: record_ref(&fields[2], "manifest")?,
        install_ref: record_ref(&fields[3], "install")?,
        role_registry_ref: record_ref(&fields[4], "role-registry")?,
        label_registry_ref: record_ref(&fields[5], "label-registry")?,
        payload_registry_ref: record_ref(&fields[6], "payload-registry")?,
        endpoint_refs,
        generator_ref: record_ref(&fields[8], "generator")?,
        artifact_ref: record_ref(&fields[9], "artifact")?,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        non_claims: parse_string_sequence(&fields[11], "non-claims")?,
        value: value.clone(),
    })
}

pub fn evaluate_protocol_facade_transition(input: ProtocolFacadeTransitionInput) -> Result<ProtocolFacadeTransition> {
    validate_name(&input.operation, "protocol facade operation")?;
    validate_name(&input.label, "protocol facade label")?;
    if let Some(peer) = &input.peer {
        validate_name(peer, "protocol facade peer")?;
    }
    if let Some(payload_tag) = &input.payload_tag {
        validate_name(payload_tag, "protocol facade payload tag")?;
    }
    validate_refs(&input.evidence_refs, "protocol facade evidence ref")?;
    let state = parse_protocol_session_state(&input.state)?;
    let mut diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    let mut message_descriptor = None;
    let mut next_state = None;
    if diagnostics.is_empty() {
        match input.operation.as_str() {
            "send" => {
                let (message, next) = evaluate_facade_send_transition(&input, &state, &mut diagnostics)?;
                message_descriptor = message;
                next_state = next;
            }
            "receive" => {
                let (message, next) = evaluate_facade_receive_transition(&input, &state, &mut diagnostics)?;
                message_descriptor = message;
                next_state = next;
            }
            "branch" | "offer" => {
                next_state = evaluate_facade_branch_transition(&input, &state, &mut diagnostics)?;
            }
            _ => diagnostics.push(PROTOCOL_TRANSITION_UNSUPPORTED_OPERATION.to_string()),
        }
    }
    let decision = if diagnostics.is_empty() && next_state.is_some() {
        "pass"
    } else {
        "deny"
    };
    let receipt_input_refs = protocol_facade_receipt_input_refs(&input)?;
    let value = protocol_facade_transition_value(&ProtocolFacadeTransitionValueInput {
        decision,
        operation: &input.operation,
        protocol_ref: &state.protocol_ref,
        session_id: &state.session_id,
        role: &state.role,
        prior_state_ref: &state.state_ref,
        message_ref: message_descriptor.as_ref().map(|message| message.message_ref.as_str()),
        next_state_ref: next_state.as_ref().map(|state| state.state_ref.as_str()),
        receipt_input_refs: &receipt_input_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(ProtocolFacadeTransition {
        decision: decision.to_string(),
        operation: input.operation,
        message_descriptor,
        next_state,
        receipt_input_refs,
        diagnostics,
        value,
    })
}

pub fn evaluate_protocol_facade_payload_access(
    input: ProtocolFacadePayloadAccessInput<'_>,
) -> Result<ProtocolFacadePayloadAccessDecision> {
    validate_name(&input.payload.owner_role, "protocol facade payload owner role")?;
    validate_name(&input.payload.payload_tag, "protocol facade payload tag")?;
    validate_name(input.local_role, "protocol facade local role")?;
    validate_name(input.expected_payload_tag, "protocol facade expected payload tag")?;
    require_ref(&input.payload.payload_ref, "protocol facade payload ref")?;
    validate_refs(input.evidence_refs, "protocol facade payload evidence ref")?;
    let mut diagnostics = Vec::new();
    if input.payload.owner_role != input.local_role {
        diagnostics.push("protocol facade payload owner role mismatch".to_string());
    }
    if input.payload.payload_tag != input.expected_payload_tag {
        diagnostics.push("protocol facade payload tag mismatch".to_string());
    }
    if input.evidence_refs.is_empty() {
        diagnostics.push("protocol facade payload access requires evidence".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let payload_ref = if diagnostics.is_empty() {
        Some(input.payload.payload_ref.clone())
    } else {
        None
    };
    Ok(ProtocolFacadePayloadAccessDecision {
        decision: decision.to_string(),
        diagnostics,
        payload_ref,
    })
}

// r[impl molten.choreography.chorus_design_reference]
pub fn protocol_facade_dependency_boundary_diagnostics(cargo_manifest: &str, cargo_lock: &str) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(PROTOCOL_FACADE_FORBIDDEN_DEPENDENCY_MARKERS.len());
    for marker in PROTOCOL_FACADE_FORBIDDEN_DEPENDENCY_MARKERS {
        if contains_dependency_marker(cargo_manifest, marker) || contains_dependency_marker(cargo_lock, marker) {
            diagnostics.push(format!(
                "protocol facade dependency boundary forbids ChoRus dependency marker {marker}"
            ));
        }
    }
    diagnostics
}

fn install_endpoint_refs(install: &ProtocolInstallReceipt) -> Vec<String> {
    install.endpoints.iter().map(|endpoint| endpoint.endpoint_ref.clone()).collect()
}

fn protocol_facade_generation_receipt_value(input: &ProtocolFacadeGenerationValueInput<'_>) -> Result<IoValue> {
    validate_gate_decision(input.decision, "protocol facade generation receipt decision")?;
    require_ref(input.manifest_ref, "protocol facade manifest ref")?;
    require_ref(input.install_ref, "protocol facade install ref")?;
    require_ref(input.role_registry_ref, "protocol facade role registry ref")?;
    require_ref(input.label_registry_ref, "protocol facade label registry ref")?;
    require_ref(input.payload_registry_ref, "protocol facade payload registry ref")?;
    validate_refs(input.endpoint_refs, "protocol facade endpoint ref")?;
    require_ref(input.generator_ref, "protocol facade generator ref")?;
    require_ref(input.artifact_ref, "protocol facade artifact ref")?;
    validate_non_claims(input.non_claims)?;
    Ok(record("protocol-facade-generation-receipt-v1", vec![
        string(PROTOCOL_FACADE_GENERATION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("install", vec![string(input.install_ref)]),
        record("role-registry", vec![string(input.role_registry_ref)]),
        record("label-registry", vec![string(input.label_registry_ref)]),
        record("payload-registry", vec![string(input.payload_registry_ref)]),
        record("endpoints", vec![refs_sequence(input.endpoint_refs)]),
        record("generator", vec![string(input.generator_ref)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("non-claims", vec![strings_sequence(input.non_claims)]),
        record("checks", vec![sequence(protocol_facade_generation_checks(input.decision))]),
    ]))
}

fn protocol_facade_transition_value(input: &ProtocolFacadeTransitionValueInput<'_>) -> Result<IoValue> {
    validate_gate_decision(input.decision, "protocol facade transition decision")?;
    validate_name(input.operation, "protocol facade transition operation")?;
    require_ref(input.protocol_ref, "protocol facade transition protocol ref")?;
    validate_session_id(input.session_id)?;
    validate_name(input.role, "protocol facade transition role")?;
    require_ref(input.prior_state_ref, "protocol facade transition prior state ref")?;
    if let Some(message_ref) = input.message_ref {
        require_ref(message_ref, "protocol facade transition message ref")?;
    }
    if let Some(next_state_ref) = input.next_state_ref {
        require_ref(next_state_ref, "protocol facade transition next state ref")?;
    }
    validate_refs(input.receipt_input_refs, "protocol facade transition receipt input ref")?;
    Ok(record("protocol-facade-transition-v1", vec![
        string(PROTOCOL_FACADE_TRANSITION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("protocol", vec![string(input.protocol_ref)]),
        record("session", vec![string(input.session_id)]),
        record("role", vec![string(input.role)]),
        record("prior-state", vec![string(input.prior_state_ref)]),
        record("message", vec![optional_ref_value(input.message_ref)]),
        record("next-state", vec![optional_ref_value(input.next_state_ref)]),
        record("receipt-inputs", vec![refs_sequence(input.receipt_input_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("checks", vec![sequence(protocol_facade_transition_checks(input.decision))]),
    ]))
}

fn evaluate_facade_send_transition(
    input: &ProtocolFacadeTransitionInput,
    state: &ProtocolSessionState,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolMessage>, Option<ProtocolSessionState>)> {
    let Some(peer) = input.peer.as_deref() else {
        diagnostics.push("protocol facade send requires a peer".to_string());
        return Ok((None, None));
    };
    let Some(payload_tag) = input.payload_tag.as_deref() else {
        diagnostics.push("protocol facade send requires a payload tag".to_string());
        return Ok((None, None));
    };
    let Some(body_or_ref) = input.body_or_ref.clone() else {
        diagnostics.push("protocol facade send requires a body or content ref".to_string());
        return Ok((None, None));
    };
    let evidence_refs = protocol_facade_message_evidence_refs(input)?;
    let message_value = protocol_message_value(&ProtocolMessageInput {
        protocol_ref: state.protocol_ref.clone(),
        session_id: state.session_id.clone(),
        from_role: state.role.clone(),
        to_role: peer.to_string(),
        label: input.label.clone(),
        payload_tag: payload_tag.to_string(),
        body_or_ref,
        sequence: state.sequence,
        evidence_refs,
    })?;
    let candidate = parse_protocol_message(&message_value)?;
    let transition = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
        operation: "send",
        prior: state,
        peer: Some(peer),
        label: &input.label,
        payload_tag: Some(payload_tag),
        message: Some(&candidate),
        next: None,
    })?;
    if transition.decision != "pass" {
        diagnostics.extend(transition.diagnostics);
        return Ok((None, None));
    }
    let next_state = next_facade_state(state, transition)?;
    Ok((Some(candidate), Some(next_state)))
}

fn evaluate_facade_receive_transition(
    input: &ProtocolFacadeTransitionInput,
    state: &ProtocolSessionState,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolMessage>, Option<ProtocolSessionState>)> {
    let Some(message_value) = input.message.as_ref() else {
        diagnostics.push(PROTOCOL_TRANSITION_MESSAGE_MISSING.to_string());
        return Ok((None, None));
    };
    let candidate = parse_protocol_message(message_value)?;
    let transition = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
        operation: "receive",
        prior: state,
        peer: Some(&candidate.from_role),
        label: &candidate.label,
        payload_tag: Some(&candidate.payload_tag),
        message: Some(&candidate),
        next: None,
    })?;
    if transition.decision != "pass" {
        diagnostics.extend(transition.diagnostics);
        return Ok((None, None));
    }
    let next_state = next_facade_state(state, transition)?;
    Ok((Some(candidate), Some(next_state)))
}

fn evaluate_facade_branch_transition(
    input: &ProtocolFacadeTransitionInput,
    state: &ProtocolSessionState,
    diagnostics: &mut Vec<String>,
) -> Result<Option<ProtocolSessionState>> {
    let transition = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
        operation: &input.operation,
        prior: state,
        peer: input.peer.as_deref(),
        label: &input.label,
        payload_tag: input.payload_tag.as_deref(),
        message: None,
        next: None,
    })?;
    if transition.decision != "pass" {
        diagnostics.extend(transition.diagnostics);
        return Ok(None);
    }
    Ok(Some(next_facade_state(state, transition)?))
}

fn next_facade_state(
    state: &ProtocolSessionState,
    transition: ProtocolEndpointTransitionDecision,
) -> Result<ProtocolSessionState> {
    let next_local_state = transition
        .next_local_state
        .ok_or_else(|| MoltenError::invalid_harness("protocol facade transition missing next local state"))?;
    advance_state(
        state,
        next_local_state,
        state.sequence.saturating_add(1),
        transition.seen_message_refs,
    )
}

fn protocol_facade_message_evidence_refs(input: &ProtocolFacadeTransitionInput) -> Result<Vec<String>> {
    let capacity = protocol_facade_receipt_ref_capacity(input)?;
    let mut refs = Vec::with_capacity(capacity);
    refs.extend(input.evidence_refs.iter().cloned());
    refs.extend(input.authority_refs.iter().cloned());
    refs.extend(input.resource_refs.iter().cloned());
    Ok(refs)
}

fn protocol_facade_receipt_input_refs(input: &ProtocolFacadeTransitionInput) -> Result<Vec<String>> {
    protocol_facade_message_evidence_refs(input)
}

fn protocol_facade_receipt_ref_capacity(input: &ProtocolFacadeTransitionInput) -> Result<usize> {
    input
        .evidence_refs
        .len()
        .checked_add(input.authority_refs.len())
        .and_then(|count| count.checked_add(input.resource_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness(FACADE_REF_CAPACITY_OVERFLOW))
}

fn protocol_facade_generation_checks(decision: &str) -> Vec<IoValue> {
    let admitted_status = pass_fail(decision == "pass");
    vec![
        record("check", vec![string("admitted-manifest"), string(admitted_status)]),
        record("check", vec![string("projected-endpoint-binding"), string(admitted_status)]),
        record("check", vec![string("install-receipt-binding"), string(admitted_status)]),
        record("check", vec![string("facade-non-authority"), string("pass")]),
        record("check", vec![string("no-transport-trust"), string("pass")]),
        record("check", vec![string("no-serde-json-protocol-identity"), string("pass")]),
        record("check", vec![string("no-chorus-compatibility"), string("pass")]),
    ]
}

fn protocol_facade_transition_checks(decision: &str) -> Vec<IoValue> {
    let transition_status = pass_fail(decision == "pass");
    vec![
        record("check", vec![string("sans-io-transition-core"), string(transition_status)]),
        record("check", vec![string("explicit-receipt-inputs"), string(transition_status)]),
        record("check", vec![string("no-shell-effects"), string("pass")]),
        record("check", vec![string("transport-neutral-message"), string("pass")]),
        record("check", vec![string("no-chorus-runtime"), string("pass")]),
    ]
}

fn protocol_facade_non_claims() -> Vec<String> {
    vec![
        FACADE_NON_CLAIM_AUTHORITY.to_string(),
        FACADE_NON_CLAIM_POLICY.to_string(),
        FACADE_NON_CLAIM_RESOURCE.to_string(),
        FACADE_NON_CLAIM_PROVENANCE.to_string(),
        FACADE_NON_CLAIM_TRANSPORT.to_string(),
        FACADE_NON_CLAIM_CHORUS.to_string(),
        FACADE_NON_CLAIM_JSON.to_string(),
    ]
}

fn validate_non_claims(non_claims: &[String]) -> Result<()> {
    ensure_count_at_most(non_claims.len(), MAX_PROTOCOL_ITEMS, "protocol facade non-claims")?;
    for non_claim in non_claims {
        validate_name(non_claim, "protocol facade non-claim")?;
    }
    Ok(())
}

fn pass_fail(passed: bool) -> &'static str {
    if passed { "pass" } else { "fail" }
}

fn contains_dependency_marker(text: &str, marker: &str) -> bool {
    text.to_ascii_lowercase().contains(&marker.to_ascii_lowercase())
}
