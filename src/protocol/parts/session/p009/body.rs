
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
