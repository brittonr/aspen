use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::PROTOCOL_ENDPOINT_SCHEMA;
use crate::preserves_rail::PROTOCOL_INSTALL_RECEIPT_SCHEMA;
use crate::preserves_rail::PROTOCOL_LOCAL_STATE_SCHEMA;
use crate::preserves_rail::PROTOCOL_MANIFEST_SCHEMA;
use crate::preserves_rail::PROTOCOL_MESSAGE_SCHEMA;
use crate::preserves_rail::PROTOCOL_OPERATION_RECEIPT_SCHEMA;
use crate::preserves_rail::PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::PROTOCOL_SESSION_STATE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;
use crate::remote_dataspace;
use crate::remote_dataspace::RemoteDataspaceEnvelope;
use crate::remote_dataspace::RemoteDataspaceEnvelopeInput;
use crate::remote_dataspace::RemoteDataspaceOperation;

const MAX_PROTOCOL_ITEMS: usize = 1024;
const MAX_PROTOCOL_STEPS: usize = 256;

const _: () = assert!(MAX_PROTOCOL_ITEMS > 0);
const _: () = assert!(MAX_PROTOCOL_STEPS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPayloadInput {
    pub tag: String,
    pub schema_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolManifestInput {
    pub protocol_id: String,
    pub roles: Vec<String>,
    pub labels: Vec<String>,
    pub payloads: Vec<ProtocolPayloadInput>,
    pub global: IOValue,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCommInput {
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolBranchInput {
    pub label: String,
    pub steps: Vec<ProtocolCommInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolChoiceInput {
    pub decider: String,
    pub branches: Vec<ProtocolBranchInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPayload {
    pub tag: String,
    pub schema_ref: String,
    pub payload_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolGlobal {
    Script(Vec<ProtocolCommInput>),
    Choice(ProtocolChoiceInput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolManifest {
    pub manifest_ref: String,
    pub protocol_id: String,
    pub roles: Vec<String>,
    pub labels: Vec<String>,
    pub payloads: Vec<ProtocolPayload>,
    pub global: ProtocolGlobal,
    pub global_value: IOValue,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub name: String,
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRegistries {
    pub roles: Vec<RegistryEntry>,
    pub labels: Vec<RegistryEntry>,
    pub payloads: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalAction {
    pub direction: String,
    pub peer: String,
    pub label: String,
    pub payload_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalBranch {
    pub label: String,
    pub actions: Vec<ProtocolLocalAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolLocalTerminal {
    End,
    InternalChoice(Vec<ProtocolLocalBranch>),
    Offer {
        from_role: String,
        branches: Vec<ProtocolLocalBranch>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalState {
    pub actions: Vec<ProtocolLocalAction>,
    pub terminal: ProtocolLocalTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolEndpoint {
    pub endpoint_ref: String,
    pub protocol_ref: String,
    pub role: String,
    pub role_id: u32,
    pub local_state: ProtocolLocalState,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolInstallReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest: ProtocolManifest,
    pub registries: ProtocolRegistries,
    pub endpoints: Vec<ProtocolEndpoint>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionStateInput {
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub sequence: u64,
    pub endpoint: IOValue,
    pub local_state: IOValue,
    pub seen_message_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionState {
    pub state_ref: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub sequence: u64,
    pub endpoint: ProtocolEndpoint,
    pub local_state: ProtocolLocalState,
    pub seen_message_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessageInput {
    pub protocol_ref: String,
    pub session_id: String,
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IOValue,
    pub sequence: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessage {
    pub message_ref: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IOValue,
    pub sequence: u64,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOperationReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub prior_state_ref: String,
    pub message_ref: Option<String>,
    pub next_state_ref: Option<String>,
    pub sequence: u64,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub carrier_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGateInput {
    pub install_receipt: IOValue,
    pub initial_states: Vec<IOValue>,
    pub operation_receipts: Vec<IOValue>,
    pub messages: Vec<IOValue>,
    pub next_states: Vec<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGate {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_count: usize,
    pub operation_count: usize,
    pub message_count: usize,
    pub final_state_count: usize,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_refs: Vec<String>,
    pub operation_refs: Vec<String>,
    pub message_refs: Vec<String>,
    pub final_state_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOperationRun {
    pub decision: String,
    pub message: Option<ProtocolMessage>,
    pub next_state: Option<ProtocolSessionState>,
    pub receipt: ProtocolOperationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSendInput {
    pub state: IOValue,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IOValue,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolReceiveInput {
    pub state: IOValue,
    pub message: IOValue,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub carrier_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolBranchOperationInput {
    pub state: IOValue,
    pub label: String,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub carrier_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRemoteEnvelopeInput {
    pub from_peer: String,
    pub from_actor: String,
    pub to_peer: String,
    pub topic: String,
    pub message: IOValue,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

struct OperationReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    protocol_ref: &'a str,
    session_id: &'a str,
    role: &'a str,
    prior_state_ref: &'a str,
    message_ref: Option<&'a str>,
    next_state_ref: Option<&'a str>,
    sequence: u64,
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    carrier_refs: &'a [String],
    diagnostics: &'a [String],
}

struct ProtocolSessionGateValueInput<'a> {
    decision: &'a str,
    install_ref: &'a str,
    protocol_ref: &'a str,
    session_ids: &'a [String],
    initial_state_refs: &'a [String],
    operation_refs: &'a [String],
    message_refs: &'a [String],
    final_state_refs: &'a [String],
    diagnostics: &'a [String],
}

struct ProtocolSessionGateParsed {
    install: ProtocolInstallReceipt,
    initial_states: Vec<ProtocolSessionState>,
    operation_receipts: Vec<ProtocolOperationReceipt>,
    messages: Vec<ProtocolMessage>,
    next_states: Vec<ProtocolSessionState>,
}

pub fn protocol_comm_value(input: &ProtocolCommInput) -> Result<IOValue> {
    validate_name(&input.from_role, "protocol comm from role")?;
    validate_name(&input.to_role, "protocol comm to role")?;
    validate_name(&input.label, "protocol comm label")?;
    validate_name(&input.payload_tag, "protocol comm payload tag")?;
    Ok(record("comm", vec![
        record("from", vec![string(&input.from_role)]),
        record("to", vec![string(&input.to_role)]),
        record("label", vec![string(&input.label)]),
        record("payload", vec![string(&input.payload_tag)]),
    ]))
}

pub fn protocol_global_script_value(steps: &[ProtocolCommInput]) -> Result<IOValue> {
    ensure_count_at_most(steps.len(), MAX_PROTOCOL_STEPS, "protocol script steps")?;
    let mut values = Vec::with_capacity(steps.len());
    for step in steps {
        values.push(protocol_comm_value(step)?);
    }
    Ok(record("global-script", vec![sequence(values)]))
}

pub fn protocol_global_choice_value(input: &ProtocolChoiceInput) -> Result<IOValue> {
    validate_name(&input.decider, "protocol choice decider")?;
    ensure_count_at_most(input.branches.len(), MAX_PROTOCOL_ITEMS, "protocol choice branches")?;
    let mut branches = Vec::with_capacity(input.branches.len());
    for branch in &input.branches {
        validate_name(&branch.label, "protocol branch label")?;
        ensure_count_at_most(branch.steps.len(), MAX_PROTOCOL_STEPS, "protocol branch steps")?;
        let mut steps = Vec::with_capacity(branch.steps.len());
        for step in &branch.steps {
            steps.push(protocol_comm_value(step)?);
        }
        branches.push(record("branch", vec![string(&branch.label), sequence(steps)]));
    }
    Ok(record("global-choice", vec![
        record("decider", vec![string(&input.decider)]),
        record("branches", vec![sequence(branches)]),
    ]))
}

pub fn protocol_manifest_value(input: &ProtocolManifestInput) -> Result<IOValue> {
    validate_protocol_manifest_input(input)?;
    let mut payloads = Vec::with_capacity(input.payloads.len());
    for payload in &input.payloads {
        payloads.push(record("payload", vec![string(&payload.tag), string(&payload.schema_ref)]));
    }
    Ok(record("protocol-manifest-v1", vec![
        string(PROTOCOL_MANIFEST_SCHEMA),
        record("protocol-id", vec![string(&input.protocol_id)]),
        record("roles", vec![strings_sequence(&input.roles)]),
        record("labels", vec![strings_sequence(&input.labels)]),
        record("payloads", vec![sequence(payloads)]),
        record("global", vec![input.global.clone()]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("capability", vec![refs_sequence(&input.capability_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        checks_value(&["finite-protocol", "canonical-protocol-manifest", "transport-neutral"]),
    ]))
}

pub fn parse_protocol_manifest(value: &IOValue) -> Result<ProtocolManifest> {
    let fields = value
        .collect_simple_record("protocol-manifest-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-manifest-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_MANIFEST_SCHEMA, "protocol manifest schema")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "finite-protocol", "protocol manifest")?;
    let protocol_id = record_string(&fields[1], "protocol-id")?;
    validate_protocol_id(&protocol_id)?;
    let roles = parse_string_sequence(&fields[2], "roles")?;
    let labels = parse_string_sequence(&fields[3], "labels")?;
    let payloads = parse_payloads(&fields[4])?;
    let global_value = record_iovalue(&fields[5], "global")?;
    let global = parse_protocol_global(&global_value)?;
    let policy_refs = parse_ref_sequence(&fields[6], "policy")?;
    let capability_refs = parse_ref_sequence(&fields[7], "capability")?;
    let resource_refs = parse_ref_sequence(&fields[8], "resource")?;
    let manifest = ProtocolManifest {
        manifest_ref: canonical_hash(value)?,
        protocol_id,
        roles,
        labels,
        payloads,
        global,
        global_value,
        policy_refs,
        capability_refs,
        resource_refs,
        value: value.clone(),
    };
    validate_protocol_manifest(&manifest)?;
    Ok(manifest)
}

pub fn install_protocol_manifest_value(value: &IOValue) -> Result<ProtocolInstallReceipt> {
    let manifest = parse_protocol_manifest(value)?;
    install_protocol_manifest(&manifest)
}

pub fn install_protocol_manifest(manifest: &ProtocolManifest) -> Result<ProtocolInstallReceipt> {
    let registries = build_registries(manifest)?;
    let trellis_global = compile_global(&manifest.global, &registries)?;
    let is_projectable = trellis::choreography_projection::projectable(&trellis_global);
    if !is_projectable {
        return install_receipt(manifest, &registries, Vec::new(), "deny", vec![
            "trellis projectability rejected protocol".to_string(),
        ]);
    }
    let mut endpoints = Vec::with_capacity(registries.roles.len());
    for role in &registries.roles {
        let projected = trellis::choreography_projection::project_endpoint(role.id, &trellis_global);
        let local_state = match local_state_from_trellis(&projected, &registries) {
            Ok(value) => value,
            Err(error) => {
                return install_receipt(manifest, &registries, Vec::new(), "deny", vec![format!(
                    "unsupported projected endpoint shape: {error}"
                )]);
            }
        };
        endpoints.push(protocol_endpoint(manifest, role, local_state)?);
    }
    install_receipt(manifest, &registries, endpoints, "pass", Vec::new())
}

pub fn protocol_session_state_value(input: &ProtocolSessionStateInput) -> Result<IOValue> {
    validate_protocol_ref(&input.protocol_ref, "protocol session protocol ref")?;
    validate_session_id(&input.session_id)?;
    validate_name(&input.role, "protocol session role")?;
    let endpoint = parse_protocol_endpoint(&input.endpoint)?;
    let _local_state = parse_protocol_local_state(&input.local_state)?;
    if endpoint.protocol_ref != input.protocol_ref {
        return Err(MoltenError::invalid_harness("protocol session endpoint protocol mismatch"));
    }
    validate_refs(&input.seen_message_refs, "protocol session seen message ref")?;
    validate_refs(&input.authority_refs, "protocol session authority ref")?;
    validate_refs(&input.resource_refs, "protocol session resource ref")?;
    if endpoint.role != input.role {
        return Err(MoltenError::invalid_harness("protocol session endpoint role mismatch"));
    }
    Ok(record("protocol-session-state-v1", vec![
        string(PROTOCOL_SESSION_STATE_SCHEMA),
        record("protocol", vec![string(&input.protocol_ref)]),
        record("session", vec![string(&input.session_id)]),
        record("role", vec![string(&input.role)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("endpoint", vec![input.endpoint.clone()]),
        record("state", vec![input.local_state.clone()]),
        record("seen", vec![refs_sequence(&input.seen_message_refs)]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        checks_value(&[
            "canonical-session-state",
            "projected-local-state",
            "bounded-replay-window",
        ]),
    ]))
}

pub fn parse_protocol_session_state(value: &IOValue) -> Result<ProtocolSessionState> {
    let fields = value
        .collect_simple_record("protocol-session-state-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-session-state-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_SESSION_STATE_SCHEMA, "protocol session state schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "projected-local-state", "protocol session state")?;
    let protocol_ref = record_ref(&fields[1], "protocol")?;
    let session_id = record_string(&fields[2], "session")?;
    validate_session_id(&session_id)?;
    let role = record_string(&fields[3], "role")?;
    let sequence_value = record_u64(&fields[4], "sequence")?;
    let endpoint_value = record_iovalue(&fields[5], "endpoint")?;
    let endpoint = parse_protocol_endpoint(&endpoint_value)?;
    let local_value = record_iovalue(&fields[6], "state")?;
    let local_state = parse_protocol_local_state(&local_value)?;
    let seen_message_refs = parse_ref_sequence(&fields[7], "seen")?;
    let authority_refs = parse_ref_sequence(&fields[8], "authority")?;
    let resource_refs = parse_ref_sequence(&fields[9], "resource")?;
    Ok(ProtocolSessionState {
        state_ref: canonical_hash(value)?,
        protocol_ref,
        session_id,
        role,
        sequence: sequence_value,
        endpoint,
        local_state,
        seen_message_refs,
        authority_refs,
        resource_refs,
        value: value.clone(),
    })
}

pub fn start_protocol_session(
    install: &ProtocolInstallReceipt,
    role: &str,
    session_id: &str,
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
) -> Result<ProtocolSessionState> {
    if install.decision != "pass" {
        return Err(MoltenError::invalid_harness("cannot start session from denied protocol install"));
    }
    validate_session_id(session_id)?;
    validate_refs(&authority_refs, "protocol session authority ref")?;
    validate_refs(&resource_refs, "protocol session resource ref")?;
    let endpoint = endpoint_for_role(&install.endpoints, role)?;
    let local_value = protocol_local_state_value(&endpoint.local_state)?;
    let state_value = protocol_session_state_value(&ProtocolSessionStateInput {
        protocol_ref: install.manifest.manifest_ref.clone(),
        session_id: session_id.to_string(),
        role: role.to_string(),
        sequence: 0,
        endpoint: endpoint.value.clone(),
        local_state: local_value,
        seen_message_refs: Vec::new(),
        authority_refs,
        resource_refs,
    })?;
    parse_protocol_session_state(&state_value)
}

pub fn protocol_message_value(input: &ProtocolMessageInput) -> Result<IOValue> {
    validate_protocol_ref(&input.protocol_ref, "protocol message protocol ref")?;
    validate_session_id(&input.session_id)?;
    validate_name(&input.from_role, "protocol message from role")?;
    validate_name(&input.to_role, "protocol message to role")?;
    validate_name(&input.label, "protocol message label")?;
    validate_name(&input.payload_tag, "protocol message payload tag")?;
    validate_refs(&input.evidence_refs, "protocol message evidence ref")?;
    Ok(record("protocol-message-v1", vec![
        string(PROTOCOL_MESSAGE_SCHEMA),
        record("protocol", vec![string(&input.protocol_ref)]),
        record("session", vec![string(&input.session_id)]),
        record("from-role", vec![string(&input.from_role)]),
        record("to-role", vec![string(&input.to_role)]),
        record("label", vec![string(&input.label)]),
        record("payload-tag", vec![string(&input.payload_tag)]),
        record("body-or-ref", vec![input.body_or_ref.clone()]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&["projected-action", "payload-schema-tag", "transport-neutral-payload"]),
    ]))
}

pub fn parse_protocol_message(value: &IOValue) -> Result<ProtocolMessage> {
    let fields = value
        .collect_simple_record("protocol-message-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-message-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_MESSAGE_SCHEMA, "protocol message schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "projected-action", "protocol message")?;
    let protocol_ref = record_ref(&fields[1], "protocol")?;
    let session_id = record_string(&fields[2], "session")?;
    validate_session_id(&session_id)?;
    Ok(ProtocolMessage {
        message_ref: canonical_hash(value)?,
        protocol_ref,
        session_id,
        from_role: record_string(&fields[3], "from-role")?,
        to_role: record_string(&fields[4], "to-role")?,
        label: record_string(&fields[5], "label")?,
        payload_tag: record_string(&fields[6], "payload-tag")?,
        body_or_ref: record_iovalue(&fields[7], "body-or-ref")?,
        sequence: record_u64(&fields[8], "sequence")?,
        evidence_refs: parse_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    })
}

pub fn send_protocol_message(input: ProtocolSendInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &[]);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("send", &state, None, gates, diagnostics);
    }
    let Some(action) = state.local_state.actions.first() else {
        return deny_operation("send", &state, None, gates, vec!["endpoint does not expect send".to_string()]);
    };
    if action.direction != "send" {
        return deny_operation("send", &state, None, gates, vec!["endpoint does not expect send".to_string()]);
    }
    if action.peer != input.to_role || action.label != input.label || action.payload_tag != input.payload_tag {
        return deny_operation("send", &state, None, gates, vec![format!(
            "send does not match projected action label={}",
            action.label
        )]);
    }
    let mut evidence_refs =
        Vec::with_capacity(input.evidence_refs.len() + input.authority_refs.len() + input.resource_refs.len());
    evidence_refs.extend(input.evidence_refs.iter().cloned());
    evidence_refs.extend(input.authority_refs.iter().cloned());
    evidence_refs.extend(input.resource_refs.iter().cloned());
    let message_value = protocol_message_value(&ProtocolMessageInput {
        protocol_ref: state.protocol_ref.clone(),
        session_id: state.session_id.clone(),
        from_role: state.role.clone(),
        to_role: input.to_role,
        label: input.label,
        payload_tag: input.payload_tag,
        body_or_ref: input.body_or_ref,
        sequence: state.sequence,
        evidence_refs,
    })?;
    let message = parse_protocol_message(&message_value)?;
    let next_state = advance_state(
        &state,
        consume_first_action(&state.local_state)?,
        state.sequence + 1,
        state.seen_message_refs.clone(),
    )?;
    pass_operation("send", &state, Some(&message), &next_state, gates)
}

pub fn receive_protocol_message(input: ProtocolReceiveInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let message = parse_protocol_message(&input.message)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("receive", &state, Some(&message), gates, diagnostics);
    }
    if state.seen_message_refs.iter().any(|reference| reference == &message.message_ref) {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "duplicate protocol message replay".to_string(),
        ]);
    }
    let Some(action) = state.local_state.actions.first() else {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "endpoint does not expect receive".to_string(),
        ]);
    };
    let expected = ExpectedReceive {
        peer: &action.peer,
        label: &action.label,
        payload_tag: &action.payload_tag,
    };
    if action.direction != "recv" || !message_matches(&message, &state, expected) {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "message does not match projected receive action".to_string(),
        ]);
    }
    let mut seen = Vec::with_capacity(state.seen_message_refs.len() + 1);
    seen.extend(state.seen_message_refs.iter().cloned());
    seen.push(message.message_ref.clone());
    let next_state = advance_state(&state, consume_first_action(&state.local_state)?, state.sequence + 1, seen)?;
    pass_operation("receive", &state, Some(&message), &next_state, gates)
}

pub fn choose_protocol_branch(input: ProtocolBranchOperationInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("branch", &state, None, gates, diagnostics);
    }
    let ProtocolLocalTerminal::InternalChoice(branches) = &state.local_state.terminal else {
        return deny_operation("branch", &state, None, gates, vec![
            "endpoint does not expect internal choice".to_string(),
        ]);
    };
    let Some(branch) = branch_for_label(branches, &input.label) else {
        return deny_operation("branch", &state, None, gates, vec![
            "branch label is not offered by projected state".to_string(),
        ]);
    };
    let next_local = ProtocolLocalState {
        actions: branch.actions.clone(),
        terminal: ProtocolLocalTerminal::End,
    };
    let next_state = advance_state(&state, next_local, state.sequence + 1, state.seen_message_refs.clone())?;
    pass_operation("branch", &state, None, &next_state, gates)
}

pub fn offer_protocol_branch(input: ProtocolBranchOperationInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("offer", &state, None, gates, diagnostics);
    }
    let ProtocolLocalTerminal::Offer { branches, .. } = &state.local_state.terminal else {
        return deny_operation("offer", &state, None, gates, vec!["endpoint does not expect offer".to_string()]);
    };
    let Some(branch) = branch_for_label(branches, &input.label) else {
        return deny_operation("offer", &state, None, gates, vec!["offer label is not projected".to_string()]);
    };
    let next_local = ProtocolLocalState {
        actions: branch.actions.clone(),
        terminal: ProtocolLocalTerminal::End,
    };
    let next_state = advance_state(&state, next_local, state.sequence + 1, state.seen_message_refs.clone())?;
    pass_operation("offer", &state, None, &next_state, gates)
}

pub fn protocol_message_remote_envelope(input: ProtocolRemoteEnvelopeInput) -> Result<RemoteDataspaceEnvelope> {
    let message = parse_protocol_message(&input.message)?;
    remote_dataspace::build_envelope(RemoteDataspaceEnvelopeInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: input.to_peer,
        topic: input.topic,
        operation: RemoteDataspaceOperation::Message,
        payload: message.value,
        content_refs: Vec::new(),
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
    })
}

pub fn parse_protocol_install_receipt(value: &IOValue) -> Result<ProtocolInstallReceipt> {
    let fields = value
        .collect_simple_record("protocol-install-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-install-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_INSTALL_RECEIPT_SCHEMA, "protocol install receipt schema")?;
    let manifest_value = record_iovalue(&fields[2], "manifest")?;
    let endpoints = parse_endpoint_sequence(&fields[6])?;
    Ok(ProtocolInstallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        manifest: parse_protocol_manifest(&manifest_value)?,
        registries: ProtocolRegistries {
            roles: parse_registry(&fields[3], "role-registry")?,
            labels: parse_registry(&fields[4], "label-registry")?,
            payloads: parse_registry(&fields[5], "payload-registry")?,
        },
        endpoints,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_protocol_operation_receipt(value: &IOValue) -> Result<ProtocolOperationReceipt> {
    let fields = value
        .collect_simple_record("protocol-operation-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-operation-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_OPERATION_RECEIPT_SCHEMA, "protocol operation receipt schema")?;
    Ok(ProtocolOperationReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        protocol_ref: record_ref(&fields[3], "protocol")?,
        session_id: record_string(&fields[4], "session")?,
        role: record_string(&fields[5], "role")?,
        prior_state_ref: record_ref(&fields[6], "prior-state")?,
        message_ref: record_optional_ref(&fields[7], "message")?,
        next_state_ref: record_optional_ref(&fields[8], "next-state")?,
        sequence: record_u64(&fields[9], "sequence")?,
        authority_refs: parse_ref_sequence(&fields[10], "authority")?,
        resource_refs: parse_ref_sequence(&fields[11], "resource")?,
        carrier_refs: parse_ref_sequence(&fields[12], "carrier")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn gate_protocol_session_lifecycle(input: ProtocolSessionGateInput) -> Result<ProtocolSessionGate> {
    let parsed = parse_protocol_session_gate_input(input)?;
    let diagnostics = protocol_session_gate_diagnostics(&parsed)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let initial_state_refs = state_refs(&parsed.initial_states);
    let operation_refs = operation_refs(&parsed.operation_receipts);
    let message_refs = message_refs(&parsed.messages);
    let final_state_refs = terminal_state_refs(&parsed.next_states);
    let session_ids = session_ids(&parsed.initial_states)?;
    let receipt_value = protocol_session_gate_receipt_value(&ProtocolSessionGateValueInput {
        decision,
        install_ref: &parsed.install.receipt_ref,
        protocol_ref: &parsed.install.manifest.manifest_ref,
        session_ids: &session_ids,
        initial_state_refs: &initial_state_refs,
        operation_refs: &operation_refs,
        message_refs: &message_refs,
        final_state_refs: &final_state_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(ProtocolSessionGate {
        receipt_ref: canonical_hash(&receipt_value)?,
        decision: decision.to_string(),
        install_ref: parsed.install.receipt_ref,
        protocol_ref: parsed.install.manifest.manifest_ref,
        session_ids,
        initial_state_count: parsed.initial_states.len(),
        operation_count: parsed.operation_receipts.len(),
        message_count: parsed.messages.len(),
        final_state_count: final_state_refs.len(),
        diagnostics,
        value: receipt_value,
    })
}

pub fn parse_protocol_session_gate_receipt(value: &IOValue) -> Result<ProtocolSessionGateReceipt> {
    let fields = value
        .collect_simple_record("protocol-session-gate-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-session-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA, "protocol session gate receipt schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "protocol-session-gate-is-not-authority", "protocol session gate receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_gate_decision(&decision, "protocol session gate decision")?;
    let session_ids = parse_string_sequence(&fields[4], "sessions")?;
    for session_id in &session_ids {
        validate_session_id(session_id)?;
    }
    Ok(ProtocolSessionGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        install_ref: record_ref(&fields[2], "install")?,
        protocol_ref: record_ref(&fields[3], "protocol")?,
        session_ids,
        initial_state_refs: parse_ref_sequence(&fields[5], "initial-states")?,
        operation_refs: parse_ref_sequence(&fields[6], "operations")?,
        message_refs: parse_ref_sequence(&fields[7], "messages")?,
        final_state_refs: parse_ref_sequence(&fields[8], "final-states")?,
        diagnostics: parse_string_sequence(&fields[9], "diagnostics")?,
    })
}

pub fn protocol_summary(value: &IOValue) -> Result<String> {
    if value.collect_simple_record("protocol-install-receipt-v1", Some(12)).is_some() {
        let install = parse_protocol_install_receipt(value)?;
        return Ok(format!(
            "protocol install receipt ref={} decision={} protocol={} endpoints={} diagnostics={}",
            install.receipt_ref,
            install.decision,
            install.manifest.protocol_id,
            install.endpoints.len(),
            install.diagnostics.len()
        ));
    }
    if value.collect_simple_record("protocol-operation-receipt-v1", Some(15)).is_some() {
        let receipt = parse_protocol_operation_receipt(value)?;
        return Ok(format!(
            "protocol operation receipt ref={} decision={} operation={} session={} role={} sequence={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.operation,
            receipt.session_id,
            receipt.role,
            receipt.sequence
        ));
    }
    if value.collect_simple_record("protocol-session-gate-receipt-v1", Some(11)).is_some() {
        let receipt = parse_protocol_session_gate_receipt(value)?;
        return Ok(format!(
            "protocol session gate receipt ref={} decision={} protocol={} sessions={} operations={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.protocol_ref,
            receipt.session_ids.len(),
            receipt.operation_refs.len(),
            receipt.diagnostics.len()
        ));
    }
    Err(MoltenError::invalid_harness("unsupported protocol summary record"))
}

pub fn request_response_manifest_value() -> Result<IOValue> {
    let request_schema = synthetic_ref("request-schema")?;
    let response_schema = synthetic_ref("response-schema")?;
    let policy_ref = synthetic_ref("policy")?;
    let capability_ref = synthetic_ref("capability")?;
    let resource_ref = synthetic_ref("resource")?;
    let global = protocol_global_script_value(&[
        ProtocolCommInput {
            from_role: "client".to_string(),
            to_role: "server".to_string(),
            label: "request".to_string(),
            payload_tag: "request".to_string(),
        },
        ProtocolCommInput {
            from_role: "server".to_string(),
            to_role: "client".to_string(),
            label: "response".to_string(),
            payload_tag: "response".to_string(),
        },
    ])?;
    protocol_manifest_value(&ProtocolManifestInput {
        protocol_id: "proto:request-response".to_string(),
        roles: vec!["client".to_string(), "server".to_string()],
        labels: vec!["request".to_string(), "response".to_string()],
        payloads: vec![
            ProtocolPayloadInput {
                tag: "request".to_string(),
                schema_ref: request_schema,
            },
            ProtocolPayloadInput {
                tag: "response".to_string(),
                schema_ref: response_schema,
            },
        ],
        global,
        policy_refs: vec![policy_ref],
        capability_refs: vec![capability_ref],
        resource_refs: vec![resource_ref],
    })
}

pub fn request_response_lifecycle() -> Result<RequestResponseLifecycle> {
    let manifest_value = request_response_manifest_value()?;
    let install = install_protocol_manifest_value(&manifest_value)?;
    let authority_ref = synthetic_ref("authority")?;
    let resource_ref = synthetic_ref("resource-run")?;
    let client0 =
        start_protocol_session(&install, "client", "session:request-response:1", vec![authority_ref.clone()], vec![
            resource_ref.clone(),
        ])?;
    let server0 =
        start_protocol_session(&install, "server", "session:request-response:1", vec![authority_ref.clone()], vec![
            resource_ref.clone(),
        ])?;
    let send_request = send_protocol_message(ProtocolSendInput {
        state: client0.value.clone(),
        to_role: "server".to_string(),
        label: "request".to_string(),
        payload_tag: "request".to_string(),
        body_or_ref: record("body", vec![string("hello")]),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        evidence_refs: vec![install.receipt_ref.clone()],
    })?;
    let request_message = required_message(&send_request)?;
    let receive_request = receive_protocol_message(ProtocolReceiveInput {
        state: server0.value.clone(),
        message: request_message.value.clone(),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        carrier_refs: Vec::new(),
    })?;
    let server1 = required_next_state(&receive_request)?;
    let send_response = send_protocol_message(ProtocolSendInput {
        state: server1.value.clone(),
        to_role: "client".to_string(),
        label: "response".to_string(),
        payload_tag: "response".to_string(),
        body_or_ref: record("body", vec![string("ok")]),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        evidence_refs: vec![receive_request.receipt.receipt_ref.clone()],
    })?;
    let response_message = required_message(&send_response)?;
    let client1 = required_next_state(&send_request)?;
    let receive_response = receive_protocol_message(ProtocolReceiveInput {
        state: client1.value.clone(),
        message: response_message.value.clone(),
        authority_refs: vec![authority_ref],
        resource_refs: vec![resource_ref],
        carrier_refs: Vec::new(),
    })?;
    Ok(RequestResponseLifecycle {
        manifest_value,
        install,
        initial_states: vec![client0, server0],
        operations: vec![send_request, receive_request, send_response, receive_response],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestResponseLifecycle {
    pub manifest_value: IOValue,
    pub install: ProtocolInstallReceipt,
    pub initial_states: Vec<ProtocolSessionState>,
    pub operations: Vec<ProtocolOperationRun>,
}

fn parse_protocol_session_gate_input(input: ProtocolSessionGateInput) -> Result<ProtocolSessionGateParsed> {
    ensure_count_at_most(input.initial_states.len(), MAX_PROTOCOL_ITEMS, "protocol gate initial states")?;
    ensure_count_at_most(input.operation_receipts.len(), MAX_PROTOCOL_STEPS, "protocol gate operations")?;
    ensure_count_at_most(input.messages.len(), MAX_PROTOCOL_STEPS, "protocol gate messages")?;
    ensure_count_at_most(input.next_states.len(), MAX_PROTOCOL_STEPS, "protocol gate next states")?;
    Ok(ProtocolSessionGateParsed {
        install: parse_protocol_install_receipt(&input.install_receipt)?,
        initial_states: parse_protocol_states(&input.initial_states)?,
        operation_receipts: parse_protocol_operation_receipts(&input.operation_receipts)?,
        messages: parse_protocol_messages(&input.messages)?,
        next_states: parse_protocol_states(&input.next_states)?,
    })
}

fn parse_protocol_states(values: &[IOValue]) -> Result<Vec<ProtocolSessionState>> {
    let mut states = Vec::with_capacity(values.len());
    for value in values {
        states.push(parse_protocol_session_state(value)?);
    }
    Ok(states)
}

fn parse_protocol_operation_receipts(values: &[IOValue]) -> Result<Vec<ProtocolOperationReceipt>> {
    let mut receipts = Vec::with_capacity(values.len());
    for value in values {
        receipts.push(parse_protocol_operation_receipt(value)?);
    }
    Ok(receipts)
}

fn parse_protocol_messages(values: &[IOValue]) -> Result<Vec<ProtocolMessage>> {
    let mut messages = Vec::with_capacity(values.len());
    for value in values {
        messages.push(parse_protocol_message(value)?);
    }
    Ok(messages)
}

fn protocol_session_gate_diagnostics(parsed: &ProtocolSessionGateParsed) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if parsed.install.decision != "pass" {
        diagnostics.push("protocol session gate requires a passing install receipt".to_string());
    }
    match install_protocol_manifest(&parsed.install.manifest) {
        Ok(recomputed) => {
            if recomputed.receipt_ref != parsed.install.receipt_ref {
                diagnostics.push("protocol install receipt does not replay from manifest".to_string());
            }
        }
        Err(error) => diagnostics.push(format!("protocol install replay failed: {error}")),
    }
    if parsed.initial_states.is_empty() {
        diagnostics.push("protocol session gate requires initial state evidence".to_string());
    }
    if parsed.operation_receipts.is_empty() {
        diagnostics.push("protocol session gate requires operation receipt evidence".to_string());
    }
    for state in &parsed.initial_states {
        diagnostics.extend(initial_state_gate_diagnostics(parsed, state));
    }
    for message in &parsed.messages {
        diagnostics.extend(message_gate_diagnostics(parsed, message));
    }
    for receipt in &parsed.operation_receipts {
        diagnostics.extend(operation_gate_diagnostics(parsed, receipt)?);
    }
    diagnostics.extend(terminal_role_diagnostics(parsed));
    Ok(diagnostics)
}

fn initial_state_gate_diagnostics(parsed: &ProtocolSessionGateParsed, state: &ProtocolSessionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if state.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("initial state {} protocol does not match install", state.state_ref));
    }
    if state.endpoint.protocol_ref != state.protocol_ref {
        diagnostics.push(format!("initial state {} endpoint protocol mismatch", state.state_ref));
    }
    if !parsed.install.endpoints.iter().any(|endpoint| endpoint.endpoint_ref == state.endpoint.endpoint_ref) {
        diagnostics.push(format!("initial state {} endpoint is not installed", state.state_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &state.role) {
        diagnostics.push(format!("initial state {} role is not in manifest", state.state_ref));
    }
    diagnostics
}

fn message_gate_diagnostics(parsed: &ProtocolSessionGateParsed, message: &ProtocolMessage) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if message.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("protocol message {} protocol does not match install", message.message_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &message.from_role) {
        diagnostics.push(format!("protocol message {} sender role is not in manifest", message.message_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &message.to_role) {
        diagnostics.push(format!("protocol message {} receiver role is not in manifest", message.message_ref));
    }
    if !parsed.install.manifest.payloads.iter().any(|payload| payload.tag == message.payload_tag) {
        diagnostics.push(format!("protocol message {} payload tag is not declared", message.message_ref));
    }
    diagnostics
}

fn operation_gate_diagnostics(
    parsed: &ProtocolSessionGateParsed,
    receipt: &ProtocolOperationReceipt,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if !matches!(receipt.decision.as_str(), "pass" | "deny") {
        diagnostics.push(format!("protocol operation {} has unsupported decision", receipt.receipt_ref));
    }
    if !matches!(receipt.operation.as_str(), "send" | "receive" | "branch" | "offer") {
        diagnostics.push(format!("protocol operation {} has unsupported operation", receipt.receipt_ref));
    }
    if receipt.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("protocol operation {} protocol does not match install", receipt.receipt_ref));
    }
    let Some(prior) = find_state(parsed, &receipt.prior_state_ref) else {
        diagnostics.push(format!("protocol operation {} prior state is missing", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    diagnostics.extend(operation_prior_diagnostics(receipt, prior));
    let message = match &receipt.message_ref {
        Some(reference) => match find_message(parsed, reference) {
            Some(message) => Some(message),
            None => {
                diagnostics.push(format!("protocol operation {} message is missing", receipt.receipt_ref));
                None
            }
        },
        None => None,
    };
    if let Some(message) = message {
        diagnostics.extend(operation_message_diagnostics(receipt, prior, message));
    }
    match receipt.decision.as_str() {
        "pass" => diagnostics.extend(pass_operation_gate_diagnostics(parsed, receipt, prior, message)?),
        "deny" => diagnostics.extend(deny_operation_gate_diagnostics(receipt)),
        _ => {}
    }
    Ok(diagnostics)
}

fn operation_prior_diagnostics(receipt: &ProtocolOperationReceipt, prior: &ProtocolSessionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if receipt.session_id != prior.session_id {
        diagnostics.push(format!("protocol operation {} session does not match prior state", receipt.receipt_ref));
    }
    if receipt.role != prior.role {
        diagnostics.push(format!("protocol operation {} role does not match prior state", receipt.receipt_ref));
    }
    if receipt.sequence != prior.sequence {
        diagnostics.push(format!("protocol operation {} sequence does not match prior state", receipt.receipt_ref));
    }
    diagnostics
}

fn operation_message_diagnostics(
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: &ProtocolMessage,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if message.protocol_ref != prior.protocol_ref || message.session_id != prior.session_id {
        diagnostics.push(format!("protocol operation {} message session binding mismatch", receipt.receipt_ref));
    }
    if receipt.operation == "send" && message.from_role != prior.role {
        diagnostics.push(format!("protocol operation {} send message sender mismatch", receipt.receipt_ref));
    }
    if receipt.operation == "receive" && message.to_role != prior.role {
        diagnostics.push(format!("protocol operation {} receive message receiver mismatch", receipt.receipt_ref));
    }
    diagnostics
}

fn pass_operation_gate_diagnostics(
    parsed: &ProtocolSessionGateParsed,
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    let Some(next_ref) = &receipt.next_state_ref else {
        diagnostics.push(format!("protocol operation {} pass is missing next state", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    let Some(next) = find_state(parsed, next_ref) else {
        diagnostics.push(format!("protocol operation {} next state is missing", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    if next.protocol_ref != prior.protocol_ref || next.session_id != prior.session_id || next.role != prior.role {
        diagnostics.push(format!("protocol operation {} next state binding mismatch", receipt.receipt_ref));
    }
    if next.sequence != prior.sequence.saturating_add(1) {
        diagnostics.push(format!("protocol operation {} next sequence is not prior+1", receipt.receipt_ref));
    }
    match replay_protocol_operation(receipt, prior, message, next) {
        Ok(replayed) => diagnostics.extend(replayed_operation_diagnostics(receipt, &replayed)),
        Err(error) => diagnostics.push(format!("protocol operation {} replay failed: {error}", receipt.receipt_ref)),
    }
    Ok(diagnostics)
}

fn deny_operation_gate_diagnostics(receipt: &ProtocolOperationReceipt) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(2);
    if receipt.next_state_ref.is_some() {
        diagnostics.push(format!("protocol operation {} deny unexpectedly has next state", receipt.receipt_ref));
    }
    if receipt.diagnostics.is_empty() {
        diagnostics.push(format!("protocol operation {} deny is missing diagnostics", receipt.receipt_ref));
    }
    diagnostics
}

fn replay_protocol_operation(
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    next: &ProtocolSessionState,
) -> Result<ProtocolOperationRun> {
    match receipt.operation.as_str() {
        "send" => {
            let message = message.ok_or_else(|| MoltenError::invalid_harness("send replay requires message"))?;
            let evidence_refs =
                send_evidence_prefix(&message.evidence_refs, &receipt.authority_refs, &receipt.resource_refs)?;
            send_protocol_message(ProtocolSendInput {
                state: prior.value.clone(),
                to_role: message.to_role.clone(),
                label: message.label.clone(),
                payload_tag: message.payload_tag.clone(),
                body_or_ref: message.body_or_ref.clone(),
                authority_refs: receipt.authority_refs.clone(),
                resource_refs: receipt.resource_refs.clone(),
                evidence_refs,
            })
        }
        "receive" => {
            let message = message.ok_or_else(|| MoltenError::invalid_harness("receive replay requires message"))?;
            receive_protocol_message(ProtocolReceiveInput {
                state: prior.value.clone(),
                message: message.value.clone(),
                authority_refs: receipt.authority_refs.clone(),
                resource_refs: receipt.resource_refs.clone(),
                carrier_refs: receipt.carrier_refs.clone(),
            })
        }
        "branch" => choose_protocol_branch(ProtocolBranchOperationInput {
            state: prior.value.clone(),
            label: transition_branch_label(prior, next, "branch")?,
            authority_refs: receipt.authority_refs.clone(),
            resource_refs: receipt.resource_refs.clone(),
            carrier_refs: receipt.carrier_refs.clone(),
        }),
        "offer" => offer_protocol_branch(ProtocolBranchOperationInput {
            state: prior.value.clone(),
            label: transition_branch_label(prior, next, "offer")?,
            authority_refs: receipt.authority_refs.clone(),
            resource_refs: receipt.resource_refs.clone(),
            carrier_refs: receipt.carrier_refs.clone(),
        }),
        value => Err(MoltenError::invalid_harness(format!("unsupported protocol operation replay {value}"))),
    }
}

fn replayed_operation_diagnostics(receipt: &ProtocolOperationReceipt, replayed: &ProtocolOperationRun) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if replayed.receipt.receipt_ref != receipt.receipt_ref {
        diagnostics.push(format!("protocol operation {} receipt does not replay", receipt.receipt_ref));
    }
    if replayed.receipt.message_ref != receipt.message_ref {
        diagnostics.push(format!("protocol operation {} message ref does not replay", receipt.receipt_ref));
    }
    if replayed.receipt.next_state_ref != receipt.next_state_ref {
        diagnostics.push(format!("protocol operation {} next state ref does not replay", receipt.receipt_ref));
    }
    diagnostics
}

fn send_evidence_prefix(
    evidence_refs: &[String],
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<Vec<String>> {
    let suffix_count = authority_refs
        .len()
        .checked_add(resource_refs.len())
        .ok_or_else(|| MoltenError::invalid_harness("protocol evidence suffix overflow"))?;
    if evidence_refs.len() < suffix_count {
        return Err(MoltenError::invalid_harness("protocol message evidence is missing gate refs"));
    }
    let prefix_count = evidence_refs.len() - suffix_count;
    let authority_end = prefix_count + authority_refs.len();
    if &evidence_refs[prefix_count..authority_end] != authority_refs {
        return Err(MoltenError::invalid_harness("protocol message evidence authority suffix mismatch"));
    }
    if &evidence_refs[authority_end..] != resource_refs {
        return Err(MoltenError::invalid_harness("protocol message evidence resource suffix mismatch"));
    }
    Ok(evidence_refs[..prefix_count].to_vec())
}

fn transition_branch_label(
    prior: &ProtocolSessionState,
    next: &ProtocolSessionState,
    operation: &str,
) -> Result<String> {
    let branches = match (operation, &prior.local_state.terminal) {
        ("branch", ProtocolLocalTerminal::InternalChoice(branches)) => branches,
        ("offer", ProtocolLocalTerminal::Offer { branches, .. }) => branches,
        _ => return Err(MoltenError::invalid_harness("protocol state does not contain requested branch shape")),
    };
    let mut matched = Vec::with_capacity(branches.len());
    for branch in branches {
        let candidate = ProtocolLocalState {
            actions: branch.actions.clone(),
            terminal: ProtocolLocalTerminal::End,
        };
        if candidate == next.local_state {
            matched.push(branch.label.clone());
        }
    }
    if matched.len() == 1 {
        return Ok(matched.remove(0));
    }
    Err(MoltenError::invalid_harness("protocol branch transition is ambiguous or missing"))
}

fn terminal_role_diagnostics(parsed: &ProtocolSessionGateParsed) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(parsed.initial_states.len());
    for state in &parsed.initial_states {
        diagnostics.extend(terminal_trace_diagnostics(parsed, state));
    }
    diagnostics
}

fn terminal_trace_diagnostics(parsed: &ProtocolSessionGateParsed, state: &ProtocolSessionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(2);
    let mut current_ref = state.state_ref.as_str();
    for _ in 0..MAX_PROTOCOL_STEPS {
        let Some(current) = find_state(parsed, current_ref) else {
            diagnostics.push(format!("protocol role {} in {} reaches missing state", state.role, state.session_id));
            return diagnostics;
        };
        if is_terminal_local_state(&current.local_state) {
            return diagnostics;
        }
        let mut next_ref: Option<&str> = None;
        let mut successor_count = 0usize;
        for receipt in &parsed.operation_receipts {
            if receipt.decision == "pass" && receipt.prior_state_ref == current_ref {
                successor_count += 1;
                next_ref = receipt.next_state_ref.as_deref();
            }
        }
        if successor_count == 1 {
            if let Some(reference) = next_ref {
                current_ref = reference;
            } else {
                diagnostics.push(format!(
                    "protocol role {} in {} has pass operation without next state",
                    state.role, state.session_id
                ));
                return diagnostics;
            }
        } else if successor_count == 0 {
            diagnostics
                .push(format!("protocol role {} in {} does not reach a terminal state", state.role, state.session_id));
            return diagnostics;
        } else {
            diagnostics
                .push(format!("protocol role {} in {} has ambiguous state successors", state.role, state.session_id));
            return diagnostics;
        }
    }
    diagnostics.push(format!("protocol role {} in {} exceeds replay step bound", state.role, state.session_id));
    diagnostics
}

fn find_state<'a>(parsed: &'a ProtocolSessionGateParsed, reference: &str) -> Option<&'a ProtocolSessionState> {
    parsed
        .initial_states
        .iter()
        .chain(parsed.next_states.iter())
        .find(|state| state.state_ref == reference)
}

fn find_message<'a>(parsed: &'a ProtocolSessionGateParsed, reference: &str) -> Option<&'a ProtocolMessage> {
    parsed.messages.iter().find(|message| message.message_ref == reference)
}

fn is_terminal_local_state(state: &ProtocolLocalState) -> bool {
    state.actions.is_empty() && matches!(state.terminal, ProtocolLocalTerminal::End)
}

fn session_ids(states: &[ProtocolSessionState]) -> Result<Vec<String>> {
    let mut sessions = Vec::with_capacity(states.len());
    for state in states {
        if !sessions.iter().any(|session| session == &state.session_id) {
            sessions.push(state.session_id.clone());
        }
    }
    ensure_count_at_most(sessions.len(), MAX_PROTOCOL_ITEMS, "protocol gate sessions")?;
    Ok(sessions)
}

fn state_refs(states: &[ProtocolSessionState]) -> Vec<String> {
    states.iter().map(|state| state.state_ref.clone()).collect()
}

fn operation_refs(receipts: &[ProtocolOperationReceipt]) -> Vec<String> {
    receipts.iter().map(|receipt| receipt.receipt_ref.clone()).collect()
}

fn message_refs(messages: &[ProtocolMessage]) -> Vec<String> {
    messages.iter().map(|message| message.message_ref.clone()).collect()
}

fn terminal_state_refs(states: &[ProtocolSessionState]) -> Vec<String> {
    states
        .iter()
        .filter(|state| is_terminal_local_state(&state.local_state))
        .map(|state| state.state_ref.clone())
        .collect()
}

fn protocol_session_gate_receipt_value(input: &ProtocolSessionGateValueInput<'_>) -> Result<IOValue> {
    validate_gate_decision(input.decision, "protocol session gate receipt decision")?;
    validate_refs(input.initial_state_refs, "protocol gate initial state ref")?;
    validate_refs(input.operation_refs, "protocol gate operation ref")?;
    validate_refs(input.message_refs, "protocol gate message ref")?;
    validate_refs(input.final_state_refs, "protocol gate final state ref")?;
    for session_id in input.session_ids {
        validate_session_id(session_id)?;
    }
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(record("protocol-session-gate-receipt-v1", vec![
        string(PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("install", vec![string(input.install_ref)]),
        record("protocol", vec![string(input.protocol_ref)]),
        record("sessions", vec![strings_sequence(input.session_ids)]),
        record("initial-states", vec![refs_sequence(input.initial_state_refs)]),
        record("operations", vec![refs_sequence(input.operation_refs)]),
        record("messages", vec![refs_sequence(input.message_refs)]),
        record("final-states", vec![refs_sequence(input.final_state_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("install-replay"), string(gate_status)]),
            record("check", vec![string("projected-operation-replay"), string(gate_status)]),
            record("check", vec![string("terminal-session-state"), string(gate_status)]),
            record("check", vec![string("transport-neutral-message"), string(gate_status)]),
            record("check", vec![string("protocol-session-gate-is-not-authority"), string("pass")]),
        ])]),
    ]))
}

fn validate_protocol_manifest_input(input: &ProtocolManifestInput) -> Result<()> {
    validate_protocol_id(&input.protocol_id)?;
    validate_unique_names(&input.roles, "protocol roles")?;
    validate_unique_names(&input.labels, "protocol labels")?;
    ensure_count_at_most(input.payloads.len(), MAX_PROTOCOL_ITEMS, "protocol payloads")?;
    for payload in &input.payloads {
        validate_name(&payload.tag, "protocol payload tag")?;
        require_ref(&payload.schema_ref, "protocol payload schema ref")?;
    }
    validate_refs(&input.policy_refs, "protocol policy ref")?;
    validate_refs(&input.capability_refs, "protocol capability ref")?;
    validate_refs(&input.resource_refs, "protocol resource ref")?;
    parse_protocol_global(&input.global)?;
    Ok(())
}

fn validate_protocol_manifest(manifest: &ProtocolManifest) -> Result<()> {
    validate_protocol_id(&manifest.protocol_id)?;
    validate_unique_names(&manifest.roles, "protocol roles")?;
    validate_unique_names(&manifest.labels, "protocol labels")?;
    let mut payload_names = Vec::with_capacity(manifest.payloads.len());
    for payload in &manifest.payloads {
        payload_names.push(payload.tag.clone());
        require_ref(&payload.schema_ref, "protocol payload schema ref")?;
    }
    validate_unique_names(&payload_names, "protocol payloads")?;
    validate_global_names(&manifest.global, manifest)
}

fn validate_global_names(global: &ProtocolGlobal, manifest: &ProtocolManifest) -> Result<()> {
    match global {
        ProtocolGlobal::Script(steps) => validate_steps(steps, manifest),
        ProtocolGlobal::Choice(choice) => {
            require_member(&choice.decider, &manifest.roles, "protocol choice decider")?;
            validate_unique_branch_labels(&choice.branches)?;
            for branch in &choice.branches {
                require_member(&branch.label, &manifest.labels, "protocol branch label")?;
                validate_steps(&branch.steps, manifest)?;
            }
            Ok(())
        }
    }
}

fn validate_steps(steps: &[ProtocolCommInput], manifest: &ProtocolManifest) -> Result<()> {
    ensure_count_at_most(steps.len(), MAX_PROTOCOL_STEPS, "protocol steps")?;
    for step in steps {
        require_member(&step.from_role, &manifest.roles, "protocol step from role")?;
        require_member(&step.to_role, &manifest.roles, "protocol step to role")?;
        require_member(&step.label, &manifest.labels, "protocol step label")?;
        require_payload(&step.payload_tag, manifest)?;
    }
    Ok(())
}

fn parse_protocol_global(value: &IOValue) -> Result<ProtocolGlobal> {
    if value.collect_simple_record("global-script", Some(1)).is_some() {
        return Ok(ProtocolGlobal::Script(parse_step_sequence(value, "global-script")?));
    }
    let fields = value
        .collect_simple_record("global-choice", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol global script or choice"))?;
    let decider = record_string(&fields[0], "decider")?;
    let branch_fields = fields[1]
        .collect_simple_record("branches", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol choice branches"))?;
    let branch_values = branch_fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol choice branch sequence"))?;
    ensure_count_at_most(branch_values.len(), MAX_PROTOCOL_ITEMS, "protocol choice branches")?;
    let mut branches = Vec::with_capacity(branch_values.len());
    for branch in branch_values.iter() {
        let branch_fields = branch
            .collect_simple_record("branch", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol branch"))?;
        let label = required_string(&branch_fields[0], "protocol branch label")?;
        let steps = parse_comm_sequence_value(&branch_fields[1], "protocol branch steps")?;
        branches.push(ProtocolBranchInput { label, steps });
    }
    Ok(ProtocolGlobal::Choice(ProtocolChoiceInput { decider, branches }))
}

fn parse_step_sequence(value: &IOValue, label: &str) -> Result<Vec<ProtocolCommInput>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    parse_comm_sequence_value(&fields[0], label)
}

fn parse_comm_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<ProtocolCommInput>> {
    let steps = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected protocol comm sequence for {label}")))?;
    ensure_count_at_most(steps.len(), MAX_PROTOCOL_STEPS, label)?;
    let mut parsed = Vec::with_capacity(steps.len());
    for step in steps.iter() {
        parsed.push(parse_comm_step(step)?);
    }
    Ok(parsed)
}

fn parse_comm_step(value: &Value<IOValue>) -> Result<ProtocolCommInput> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("comm", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol comm step"))?;
    Ok(ProtocolCommInput {
        from_role: record_string(&fields[0], "from")?,
        to_role: record_string(&fields[1], "to")?,
        label: record_string(&fields[2], "label")?,
        payload_tag: record_string(&fields[3], "payload")?,
    })
}

fn build_registries(manifest: &ProtocolManifest) -> Result<ProtocolRegistries> {
    Ok(ProtocolRegistries {
        roles: registry_entries(&manifest.roles, "protocol roles")?,
        labels: registry_entries(&manifest.labels, "protocol labels")?,
        payloads: registry_entries(
            &manifest.payloads.iter().map(|payload| payload.tag.clone()).collect::<Vec<_>>(),
            "protocol payloads",
        )?,
    })
}

fn registry_entries(names: &[String], label: &str) -> Result<Vec<RegistryEntry>> {
    ensure_count_at_most(names.len(), MAX_PROTOCOL_ITEMS, label)?;
    let mut entries = Vec::with_capacity(names.len());
    for (index, name) in names.iter().enumerate() {
        entries.push(RegistryEntry {
            name: name.clone(),
            id: u32::try_from(index)
                .map_err(|error| MoltenError::invalid_harness(format!("protocol registry id overflow: {error}")))?,
        });
    }
    Ok(entries)
}

fn compile_global(
    global: &ProtocolGlobal,
    registries: &ProtocolRegistries,
) -> Result<trellis::choreography_global::GlobalChoreo> {
    match global {
        ProtocolGlobal::Script(steps) => compile_script(steps, registries),
        ProtocolGlobal::Choice(choice) => compile_choice(choice, registries),
    }
}

fn compile_choice(
    choice: &ProtocolChoiceInput,
    registries: &ProtocolRegistries,
) -> Result<trellis::choreography_global::GlobalChoreo> {
    let mut branches = Vec::with_capacity(choice.branches.len());
    for branch in &choice.branches {
        branches.push(trellis::choreography_global::GlobalBranch {
            label: registry_id(&registries.labels, &branch.label, "branch label")?,
            body: compile_script(&branch.steps, registries)?,
        });
    }
    Ok(trellis::choreography_global::GlobalChoreo::Choice {
        decider: registry_id(&registries.roles, &choice.decider, "choice decider")?,
        branches,
    })
}

fn compile_script(
    steps: &[ProtocolCommInput],
    registries: &ProtocolRegistries,
) -> Result<trellis::choreography_global::GlobalChoreo> {
    let mut global = trellis::choreography_global::GlobalChoreo::End;
    for step in steps.iter().rev() {
        global = trellis::choreography_global::GlobalChoreo::Comm {
            from: registry_id(&registries.roles, &step.from_role, "comm from role")?,
            to: registry_id(&registries.roles, &step.to_role, "comm to role")?,
            label: registry_id(&registries.labels, &step.label, "comm label")?,
            payload_tag: registry_id(&registries.payloads, &step.payload_tag, "comm payload tag")?,
            next: Box::new(global),
        };
    }
    Ok(global)
}

fn local_state_from_trellis(
    local: &trellis::choreography_local::LocalChoreo,
    registries: &ProtocolRegistries,
) -> Result<ProtocolLocalState> {
    let mut actions = Vec::with_capacity(MAX_PROTOCOL_STEPS.min(16));
    let mut current = local;
    for _step in 0..=MAX_PROTOCOL_STEPS {
        ensure_count_at_most(actions.len(), MAX_PROTOCOL_STEPS, "projected local actions")?;
        match current {
            trellis::choreography_local::LocalChoreo::End => {
                return Ok(ProtocolLocalState {
                    actions,
                    terminal: ProtocolLocalTerminal::End,
                });
            }
            trellis::choreography_local::LocalChoreo::Send {
                peer,
                label,
                payload_tag,
                next,
            } => {
                actions.push(local_action("send", *peer, *label, *payload_tag, registries)?);
                current = next;
            }
            trellis::choreography_local::LocalChoreo::Recv {
                peer,
                label,
                payload_tag,
                next,
            } => {
                actions.push(local_action("recv", *peer, *label, *payload_tag, registries)?);
                current = next;
            }
            trellis::choreography_local::LocalChoreo::InternalChoice { branches } => {
                return Ok(ProtocolLocalState {
                    actions,
                    terminal: ProtocolLocalTerminal::InternalChoice(local_branches_from_trellis(branches, registries)?),
                });
            }
            trellis::choreography_local::LocalChoreo::Offer { from, branches } => {
                return Ok(ProtocolLocalState {
                    actions,
                    terminal: ProtocolLocalTerminal::Offer {
                        from_role: registry_name(&registries.roles, *from, "offer from role")?,
                        branches: local_branches_from_trellis(branches, registries)?,
                    },
                });
            }
        }
    }
    Err(MoltenError::invalid_harness("projected local state exceeds protocol step bound"))
}

fn local_branches_from_trellis(
    branches: &[trellis::choreography_local::LocalBranch],
    registries: &ProtocolRegistries,
) -> Result<Vec<ProtocolLocalBranch>> {
    ensure_count_at_most(branches.len(), MAX_PROTOCOL_ITEMS, "projected local branches")?;
    let mut local_branches = Vec::with_capacity(branches.len());
    for branch in branches {
        local_branches.push(ProtocolLocalBranch {
            label: registry_name(&registries.labels, branch.label, "local branch label")?,
            actions: linear_actions_from_trellis(&branch.body, registries)?,
        });
    }
    Ok(local_branches)
}

fn linear_actions_from_trellis(
    local: &trellis::choreography_local::LocalChoreo,
    registries: &ProtocolRegistries,
) -> Result<Vec<ProtocolLocalAction>> {
    let mut actions = Vec::with_capacity(MAX_PROTOCOL_STEPS.min(16));
    let mut current = local;
    for _step in 0..=MAX_PROTOCOL_STEPS {
        ensure_count_at_most(actions.len(), MAX_PROTOCOL_STEPS, "projected branch actions")?;
        match current {
            trellis::choreography_local::LocalChoreo::End => return Ok(actions),
            trellis::choreography_local::LocalChoreo::Send {
                peer,
                label,
                payload_tag,
                next,
            } => {
                actions.push(local_action("send", *peer, *label, *payload_tag, registries)?);
                current = next;
            }
            trellis::choreography_local::LocalChoreo::Recv {
                peer,
                label,
                payload_tag,
                next,
            } => {
                actions.push(local_action("recv", *peer, *label, *payload_tag, registries)?);
                current = next;
            }
            trellis::choreography_local::LocalChoreo::InternalChoice { branches: _ } => {
                return Err(MoltenError::invalid_harness("nested internal choice projection is unsupported"));
            }
            trellis::choreography_local::LocalChoreo::Offer { from: _, branches: _ } => {
                return Err(MoltenError::invalid_harness("nested offer projection is unsupported"));
            }
        }
    }
    Err(MoltenError::invalid_harness("projected branch actions exceed protocol step bound"))
}

fn local_action(
    direction: &str,
    peer: u32,
    label: u32,
    payload_tag: u32,
    registries: &ProtocolRegistries,
) -> Result<ProtocolLocalAction> {
    Ok(ProtocolLocalAction {
        direction: direction.to_string(),
        peer: registry_name(&registries.roles, peer, "local action peer")?,
        label: registry_name(&registries.labels, label, "local action label")?,
        payload_tag: registry_name(&registries.payloads, payload_tag, "local action payload")?,
    })
}

fn protocol_endpoint(
    manifest: &ProtocolManifest,
    role: &RegistryEntry,
    local_state: ProtocolLocalState,
) -> Result<ProtocolEndpoint> {
    let local_value = protocol_local_state_value(&local_state)?;
    let endpoint_value = record("protocol-endpoint-v1", vec![
        string(PROTOCOL_ENDPOINT_SCHEMA),
        record("protocol", vec![string(&manifest.manifest_ref)]),
        record("role", vec![string(&role.name)]),
        record("role-id", vec![u64_value(u64::from(role.id))]),
        record("state", vec![local_value]),
        checks_value(&["canonical-protocol-endpoint", "trellis-projection", "transport-neutral"]),
    ]);
    parse_protocol_endpoint(&endpoint_value)
}

fn parse_protocol_endpoint(value: &IOValue) -> Result<ProtocolEndpoint> {
    let fields = value
        .collect_simple_record("protocol-endpoint-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-endpoint-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_ENDPOINT_SCHEMA, "protocol endpoint schema")?;
    let protocol_ref = record_ref(&fields[1], "protocol")?;
    let role = record_string(&fields[2], "role")?;
    let role_id = u32::try_from(record_u64(&fields[3], "role-id")?)
        .map_err(|error| MoltenError::invalid_harness(format!("protocol role id out of range: {error}")))?;
    let local_value = record_iovalue(&fields[4], "state")?;
    let local_state = parse_protocol_local_state(&local_value)?;
    Ok(ProtocolEndpoint {
        endpoint_ref: canonical_hash(value)?,
        protocol_ref,
        role,
        role_id,
        local_state,
        value: value.clone(),
    })
}

fn protocol_local_state_value(state: &ProtocolLocalState) -> Result<IOValue> {
    ensure_count_at_most(state.actions.len(), MAX_PROTOCOL_STEPS, "protocol local actions")?;
    let mut actions = Vec::with_capacity(state.actions.len());
    for action in &state.actions {
        actions.push(local_action_value(action)?);
    }
    Ok(record("protocol-local-state-v1", vec![
        string(PROTOCOL_LOCAL_STATE_SCHEMA),
        record("actions", vec![sequence(actions)]),
        record("terminal", vec![local_terminal_value(&state.terminal)?]),
        checks_value(&["canonical-protocol-local-state", "bounded-projection"]),
    ]))
}

fn parse_protocol_local_state(value: &IOValue) -> Result<ProtocolLocalState> {
    let fields = value
        .collect_simple_record("protocol-local-state-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-local-state-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_LOCAL_STATE_SCHEMA, "protocol local state schema")?;
    Ok(ProtocolLocalState {
        actions: parse_local_actions(&fields[1])?,
        terminal: parse_local_terminal(&fields[2])?,
    })
}

fn local_action_value(action: &ProtocolLocalAction) -> Result<IOValue> {
    validate_direction(&action.direction)?;
    validate_name(&action.peer, "protocol local action peer")?;
    validate_name(&action.label, "protocol local action label")?;
    validate_name(&action.payload_tag, "protocol local action payload")?;
    let record_label = if action.direction == "send" { "send" } else { "recv" };
    Ok(record(record_label, vec![string(&action.peer), string(&action.label), string(&action.payload_tag)]))
}

fn local_terminal_value(terminal: &ProtocolLocalTerminal) -> Result<IOValue> {
    match terminal {
        ProtocolLocalTerminal::End => Ok(record("end", Vec::new())),
        ProtocolLocalTerminal::InternalChoice(branches) => {
            Ok(record("internal-choice", vec![local_branch_sequence(branches)?]))
        }
        ProtocolLocalTerminal::Offer { from_role, branches } => Ok(record("offer", vec![
            record("from", vec![string(from_role)]),
            record("branches", vec![local_branch_sequence(branches)?]),
        ])),
    }
}

fn local_branch_sequence(branches: &[ProtocolLocalBranch]) -> Result<IOValue> {
    ensure_count_at_most(branches.len(), MAX_PROTOCOL_ITEMS, "protocol local branches")?;
    let mut values = Vec::with_capacity(branches.len());
    for branch in branches {
        let mut actions = Vec::with_capacity(branch.actions.len());
        for action in &branch.actions {
            actions.push(local_action_value(action)?);
        }
        values.push(record("branch", vec![string(&branch.label), sequence(actions)]));
    }
    Ok(sequence(values))
}

fn parse_local_actions(value: &Value<IOValue>) -> Result<Vec<ProtocolLocalAction>> {
    let fields = value
        .collect_simple_record("actions", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local actions"))?;
    parse_local_action_sequence(&fields[0])
}

fn parse_local_action_sequence(value: &Value<IOValue>) -> Result<Vec<ProtocolLocalAction>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local action sequence"))?;
    ensure_count_at_most(values.len(), MAX_PROTOCOL_STEPS, "protocol local actions")?;
    let mut actions = Vec::with_capacity(values.len());
    for action in values.iter() {
        actions.push(parse_local_action(action)?);
    }
    Ok(actions)
}

fn parse_local_action(value: &Value<IOValue>) -> Result<ProtocolLocalAction> {
    if let Some(fields) = value.collect_simple_record("send", Some(3)) {
        return Ok(ProtocolLocalAction {
            direction: "send".to_string(),
            peer: required_string(&fields[0], "send peer")?,
            label: required_string(&fields[1], "send label")?,
            payload_tag: required_string(&fields[2], "send payload")?,
        });
    }
    let fields = value
        .collect_simple_record("recv", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local send or recv"))?;
    Ok(ProtocolLocalAction {
        direction: "recv".to_string(),
        peer: required_string(&fields[0], "recv peer")?,
        label: required_string(&fields[1], "recv label")?,
        payload_tag: required_string(&fields[2], "recv payload")?,
    })
}

fn parse_local_terminal(value: &Value<IOValue>) -> Result<ProtocolLocalTerminal> {
    let fields = value
        .collect_simple_record("terminal", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local terminal"))?;
    if fields[0].collect_simple_record("end", Some(0)).is_some() {
        return Ok(ProtocolLocalTerminal::End);
    }
    if let Some(choice) = fields[0].collect_simple_record("internal-choice", Some(1)) {
        return Ok(ProtocolLocalTerminal::InternalChoice(parse_local_branches(&choice[0])?));
    }
    let offer = fields[0]
        .collect_simple_record("offer", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local terminal value"))?;
    Ok(ProtocolLocalTerminal::Offer {
        from_role: record_string(&offer[0], "from")?,
        branches: parse_local_branches_record(&offer[1])?,
    })
}

fn parse_local_branches_record(value: &Value<IOValue>) -> Result<Vec<ProtocolLocalBranch>> {
    let fields = value
        .collect_simple_record("branches", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch record"))?;
    parse_local_branches(&fields[0])
}

fn parse_local_branches(value: &Value<IOValue>) -> Result<Vec<ProtocolLocalBranch>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch sequence"))?;
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, "protocol local branches")?;
    let mut branches = Vec::with_capacity(values.len());
    for branch in values.iter() {
        let fields = branch
            .collect_simple_record("branch", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch"))?;
        branches.push(ProtocolLocalBranch {
            label: required_string(&fields[0], "protocol local branch label")?,
            actions: parse_local_action_sequence(&fields[1])?,
        });
    }
    Ok(branches)
}

fn install_receipt(
    manifest: &ProtocolManifest,
    registries: &ProtocolRegistries,
    endpoints: Vec<ProtocolEndpoint>,
    decision: &str,
    diagnostics: Vec<String>,
) -> Result<ProtocolInstallReceipt> {
    let mut endpoint_values = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        endpoint_values.push(endpoint.value.clone());
    }
    let value = record("protocol-install-receipt-v1", vec![
        string(PROTOCOL_INSTALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![manifest.value.clone()]),
        registry_value("role-registry", &registries.roles),
        registry_value("label-registry", &registries.labels),
        registry_value("payload-registry", &registries.payloads),
        record("endpoints", vec![sequence(endpoint_values)]),
        record("policy", vec![refs_sequence(&manifest.policy_refs)]),
        record("capability", vec![refs_sequence(&manifest.capability_refs)]),
        record("resource", vec![refs_sequence(&manifest.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            "trellis-projectability",
            "endpoint-projection",
            "install-receipt-binding",
        ]),
    ]);
    parse_protocol_install_receipt(&value)
}

fn registry_value(label: &str, entries: &[RegistryEntry]) -> IOValue {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(record("entry", vec![string(&entry.name), u64_value(u64::from(entry.id))]));
    }
    if label == "role-registry" {
        return record("role-registry", vec![sequence(values)]);
    }
    if label == "label-registry" {
        return record("label-registry", vec![sequence(values)]);
    }
    record("payload-registry", vec![sequence(values)])
}

fn parse_registry(value: &Value<IOValue>, label: &str) -> Result<Vec<RegistryEntry>> {
    let values = field_sequence(value, label)?;
    let mut entries = Vec::with_capacity(values.len());
    for entry in values.iter() {
        let fields = entry
            .collect_simple_record("entry", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol registry entry"))?;
        entries.push(RegistryEntry {
            name: required_string(&fields[0], "registry entry name")?,
            id: u32::try_from(required_u64(&fields[1], "registry entry id")?)
                .map_err(|error| MoltenError::invalid_harness(format!("registry id out of range: {error}")))?,
        });
    }
    Ok(entries)
}

fn parse_endpoint_sequence(value: &Value<IOValue>) -> Result<Vec<ProtocolEndpoint>> {
    let values = field_sequence(value, "endpoints")?;
    let mut endpoints = Vec::with_capacity(values.len());
    for endpoint in values.iter() {
        endpoints.push(parse_protocol_endpoint(&value_to_iovalue(endpoint))?);
    }
    Ok(endpoints)
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IOValue> {
    validate_refs(input.authority_refs, "protocol operation authority ref")?;
    validate_refs(input.resource_refs, "protocol operation resource ref")?;
    validate_refs(input.carrier_refs, "protocol operation carrier ref")?;
    Ok(record("protocol-operation-receipt-v1", vec![
        string(PROTOCOL_OPERATION_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("protocol", vec![string(input.protocol_ref)]),
        record("session", vec![string(input.session_id)]),
        record("role", vec![string(input.role)]),
        record("prior-state", vec![string(input.prior_state_ref)]),
        record("message", vec![optional_ref_value(input.message_ref)]),
        record("next-state", vec![optional_ref_value(input.next_state_ref)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("carrier", vec![refs_sequence(input.carrier_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            "projected-local-state",
            "sequence-window",
            "decision-before-side-effects",
        ]),
    ]))
}

#[derive(Clone, Copy)]
struct OperationGates<'a> {
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    carrier_refs: &'a [String],
}

fn operation_gates<'a>(
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    carrier_refs: &'a [String],
) -> OperationGates<'a> {
    OperationGates {
        authority_refs,
        resource_refs,
        carrier_refs,
    }
}

fn pass_operation(
    operation: &str,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    next: &ProtocolSessionState,
    gates: OperationGates<'_>,
) -> Result<ProtocolOperationRun> {
    let receipt_value = operation_receipt_value(&OperationReceiptValueInput {
        operation,
        decision: "pass",
        protocol_ref: &prior.protocol_ref,
        session_id: &prior.session_id,
        role: &prior.role,
        prior_state_ref: &prior.state_ref,
        message_ref: message.map(|value| value.message_ref.as_str()),
        next_state_ref: Some(&next.state_ref),
        sequence: prior.sequence,
        authority_refs: gates.authority_refs,
        resource_refs: gates.resource_refs,
        carrier_refs: gates.carrier_refs,
        diagnostics: &[],
    })?;
    Ok(ProtocolOperationRun {
        decision: "pass".to_string(),
        message: message.cloned(),
        next_state: Some(next.clone()),
        receipt: parse_protocol_operation_receipt(&receipt_value)?,
    })
}

fn deny_operation(
    operation: &str,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    gates: OperationGates<'_>,
    diagnostics: Vec<String>,
) -> Result<ProtocolOperationRun> {
    let receipt_value = operation_receipt_value(&OperationReceiptValueInput {
        operation,
        decision: "deny",
        protocol_ref: &prior.protocol_ref,
        session_id: &prior.session_id,
        role: &prior.role,
        prior_state_ref: &prior.state_ref,
        message_ref: message.map(|value| value.message_ref.as_str()),
        next_state_ref: None,
        sequence: prior.sequence,
        authority_refs: gates.authority_refs,
        resource_refs: gates.resource_refs,
        carrier_refs: gates.carrier_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(ProtocolOperationRun {
        decision: "deny".to_string(),
        message: None,
        next_state: None,
        receipt: parse_protocol_operation_receipt(&receipt_value)?,
    })
}

fn advance_state(
    prior: &ProtocolSessionState,
    local_state: ProtocolLocalState,
    sequence_value: u64,
    seen_message_refs: Vec<String>,
) -> Result<ProtocolSessionState> {
    let local_value = protocol_local_state_value(&local_state)?;
    let state_value = protocol_session_state_value(&ProtocolSessionStateInput {
        protocol_ref: prior.protocol_ref.clone(),
        session_id: prior.session_id.clone(),
        role: prior.role.clone(),
        sequence: sequence_value,
        endpoint: prior.endpoint.value.clone(),
        local_state: local_value,
        seen_message_refs,
        authority_refs: prior.authority_refs.clone(),
        resource_refs: prior.resource_refs.clone(),
    })?;
    parse_protocol_session_state(&state_value)
}

fn consume_first_action(local_state: &ProtocolLocalState) -> Result<ProtocolLocalState> {
    if local_state.actions.is_empty() {
        return Err(MoltenError::invalid_harness("cannot advance local state with no actions"));
    }
    let mut actions = Vec::with_capacity(local_state.actions.len().saturating_sub(1));
    for action in local_state.actions.iter().skip(1) {
        actions.push(action.clone());
    }
    Ok(ProtocolLocalState {
        actions,
        terminal: local_state.terminal.clone(),
    })
}

struct ExpectedReceive<'a> {
    peer: &'a str,
    label: &'a str,
    payload_tag: &'a str,
}

fn message_matches(message: &ProtocolMessage, state: &ProtocolSessionState, expected: ExpectedReceive<'_>) -> bool {
    message.protocol_ref == state.protocol_ref
        && message.session_id == state.session_id
        && message.from_role == expected.peer
        && message.to_role == state.role
        && message.label == expected.label
        && message.payload_tag == expected.payload_tag
        && message.sequence == state.sequence
}

fn admission_diagnostics(authority_refs: &[String], resource_refs: &[String]) -> Result<Vec<String>> {
    validate_refs(authority_refs, "protocol operation authority ref")?;
    validate_refs(resource_refs, "protocol operation resource ref")?;
    if authority_refs.is_empty() {
        return Ok(vec!["missing protocol authority evidence".to_string()]);
    }
    if resource_refs.is_empty() {
        return Ok(vec!["missing protocol resource evidence".to_string()]);
    }
    Ok(Vec::new())
}

fn required_message(run: &ProtocolOperationRun) -> Result<ProtocolMessage> {
    run.message
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol message in pass operation"))
}

fn required_next_state(run: &ProtocolOperationRun) -> Result<ProtocolSessionState> {
    run.next_state
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("expected next protocol state in pass operation"))
}

fn endpoint_for_role(endpoints: &[ProtocolEndpoint], role: &str) -> Result<ProtocolEndpoint> {
    for endpoint in endpoints {
        if endpoint.role == role {
            return Ok(endpoint.clone());
        }
    }
    Err(MoltenError::invalid_harness(format!("missing endpoint for role {role}")))
}

fn branch_for_label<'a>(branches: &'a [ProtocolLocalBranch], label: &str) -> Option<&'a ProtocolLocalBranch> {
    branches.iter().find(|branch| branch.label == label)
}

fn parse_payloads(value: &Value<IOValue>) -> Result<Vec<ProtocolPayload>> {
    let values = field_sequence(value, "payloads")?;
    let mut payloads = Vec::with_capacity(values.len());
    for (index, payload) in values.iter().enumerate() {
        let fields = payload
            .collect_simple_record("payload", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol payload"))?;
        let tag = required_string(&fields[0], "payload tag")?;
        let schema_ref = required_ref(&fields[1], "payload schema ref")?;
        payloads.push(ProtocolPayload {
            tag,
            schema_ref,
            payload_id: u32::try_from(index)
                .map_err(|error| MoltenError::invalid_harness(format!("payload id out of range: {error}")))?,
        });
    }
    Ok(payloads)
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, label)?;
    Ok(values.iter().cloned().collect())
}

fn parse_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let mut strings = Vec::with_capacity(values.len());
    for value in &values {
        strings.push(required_string(value, label)?);
    }
    Ok(strings)
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let mut refs = Vec::with_capacity(values.len());
    for value in &values {
        refs.push(required_ref(value, label)?);
    }
    Ok(refs)
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    let mut checks = Vec::with_capacity(values.len());
    for value in &values {
        let fields = value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol check"))?;
        checks.push((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?));
    }
    Ok(checks)
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("missing passing check {name} for {label}")))
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn refs_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(names: &[&str]) -> IOValue {
    let mut checks = Vec::with_capacity(names.len());
    for name in names {
        checks.push(record("check", vec![string(name), string("pass")]));
    }
    record("checks", vec![sequence(checks)])
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    match reference {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} STRING>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    Ok(Some(required_ref(&some[0], label)?))
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} U64>")))?;
    required_u64(&fields[0], label)
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn required_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn required_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = required_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn require_schema(value: &Value<IOValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected {expected} for {label}, got {actual}")))
}

fn validate_protocol_id(value: &str) -> Result<()> {
    if value.starts_with("proto:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected proto: protocol id, got {value}")))
}

fn validate_protocol_ref(value: &str, label: &str) -> Result<()> {
    require_ref(value, label)
}

fn validate_session_id(value: &str) -> Result<()> {
    if value.starts_with("session:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected session: protocol session id, got {value}")))
}

fn validate_direction(value: &str) -> Result<()> {
    if value == "send" || value == "recv" {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unsupported protocol local action direction {value}")))
}

fn validate_gate_decision(value: &str, label: &str) -> Result<()> {
    if matches!(value, "pass" | "deny") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unsupported {label} {value}")))
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    let has_valid_chars = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == ':' || ch == '-' || ch == '_' || ch == '.');
    if !value.is_empty() && has_valid_chars {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("invalid {label}: {value}")))
}

fn validate_unique_names(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, label)?;
    for value in values {
        validate_name(value, label)?;
    }
    for (index, left) in values.iter().enumerate() {
        for right in values.iter().skip(index + 1) {
            if left == right {
                return Err(MoltenError::invalid_harness(format!("duplicate {label} entry {left}")));
            }
        }
    }
    Ok(())
}

fn validate_unique_branch_labels(branches: &[ProtocolBranchInput]) -> Result<()> {
    for (index, left) in branches.iter().enumerate() {
        for right in branches.iter().skip(index + 1) {
            if left.label == right.label {
                return Err(MoltenError::invalid_harness(format!("duplicate protocol branch label {}", left.label)));
            }
        }
    }
    Ok(())
}

fn require_member(value: &str, values: &[String], label: &str) -> Result<()> {
    if values.iter().any(|item| item == value) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unknown {label}: {value}")))
}

fn require_payload(value: &str, manifest: &ProtocolManifest) -> Result<()> {
    if manifest.payloads.iter().any(|payload| payload.tag == value) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("unknown protocol payload tag {value}")))
}

fn registry_id(entries: &[RegistryEntry], name: &str, label: &str) -> Result<u32> {
    for entry in entries {
        if entry.name == name {
            return Ok(entry.id);
        }
    }
    Err(MoltenError::invalid_harness(format!("missing {label} registry entry {name}")))
}

fn registry_name(entries: &[RegistryEntry], id: u32, label: &str) -> Result<String> {
    for entry in entries {
        if entry.id == id {
            return Ok(entry.name.clone());
        }
    }
    Err(MoltenError::invalid_harness(format!("missing {label} registry id {id}")))
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_PROTOCOL_ITEMS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    if reference.starts_with("blake3:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected blake3 ref for {label}, got {reference}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("protocol-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog::CatalogListInput;
    use crate::catalog::CatalogVisibilityInput;
    use crate::catalog_mcp;
    use crate::ledger;
    use crate::preserves_rail::to_text;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("protocol-test-ref", vec![string(label)])).expect("test ref")
    }

    fn auth() -> Vec<String> {
        vec![test_ref("authority")]
    }

    fn resources() -> Vec<String> {
        vec![test_ref("resource")]
    }

    fn gate_input(lifecycle: &RequestResponseLifecycle) -> ProtocolSessionGateInput {
        ProtocolSessionGateInput {
            install_receipt: lifecycle.install.value.clone(),
            initial_states: lifecycle.initial_states.iter().map(|state| state.value.clone()).collect(),
            operation_receipts: lifecycle.operations.iter().map(|operation| operation.receipt.value.clone()).collect(),
            messages: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.message.as_ref().map(|message| message.value.clone()))
                .collect(),
            next_states: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.next_state.as_ref().map(|state| state.value.clone()))
                .collect(),
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-protocol-{label}-{}-{id}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn request_response_installs_and_interprets() {
        let lifecycle = request_response_lifecycle().expect("request response lifecycle");
        assert_eq!(lifecycle.install.decision, "pass");
        assert_eq!(lifecycle.install.endpoints.len(), 2);
        assert_eq!(lifecycle.operations.len(), 4);
        for operation in &lifecycle.operations {
            assert_eq!(operation.decision, "pass");
        }
        let gate = gate_protocol_session_lifecycle(gate_input(&lifecycle)).expect("protocol session gate");
        assert_eq!(gate.decision, "pass");
        assert_eq!(gate.operation_count, 4);
        let gate_receipt = parse_protocol_session_gate_receipt(&gate.value).expect("parse protocol gate receipt");
        assert_eq!(gate_receipt.decision, "pass");
        assert_eq!(gate_receipt.operation_refs.len(), 4);
        assert!(matches!(
            lifecycle
                .operations
                .last()
                .expect("last op")
                .next_state
                .as_ref()
                .expect("next")
                .local_state
                .terminal,
            ProtocolLocalTerminal::End
        ));
    }

    #[test]
    fn non_projectable_protocol_denies_install() {
        let global = protocol_global_script_value(&[ProtocolCommInput {
            from_role: "client".to_string(),
            to_role: "client".to_string(),
            label: "loop".to_string(),
            payload_tag: "loop".to_string(),
        }])
        .expect("global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:bad".to_string(),
            roles: vec!["client".to_string()],
            labels: vec!["loop".to_string()],
            payloads: vec![ProtocolPayloadInput {
                tag: "loop".to_string(),
                schema_ref: test_ref("schema"),
            }],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install receipt");
        assert_eq!(install.decision, "deny");
        assert!(install.endpoints.is_empty());
    }

    #[test]
    fn wrong_label_and_missing_authority_deny_before_message() {
        let manifest_value = request_response_manifest_value().expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install");
        let client = start_protocol_session(&install, "client", "session:deny", auth(), resources()).expect("client");
        let wrong = send_protocol_message(ProtocolSendInput {
            state: client.value.clone(),
            to_role: "server".to_string(),
            label: "response".to_string(),
            payload_tag: "response".to_string(),
            body_or_ref: record("body", vec![string("bad")]),
            authority_refs: auth(),
            resource_refs: resources(),
            evidence_refs: Vec::new(),
        })
        .expect("wrong label denial");
        assert_eq!(wrong.decision, "deny");
        assert!(wrong.message.is_none());
        let missing_auth = send_protocol_message(ProtocolSendInput {
            state: client.value,
            to_role: "server".to_string(),
            label: "request".to_string(),
            payload_tag: "request".to_string(),
            body_or_ref: record("body", vec![string("bad")]),
            authority_refs: Vec::new(),
            resource_refs: resources(),
            evidence_refs: Vec::new(),
        })
        .expect("missing auth denial");
        assert_eq!(missing_auth.decision, "deny");
        assert!(missing_auth.receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    #[test]
    fn protocol_session_gate_denies_missing_next_state() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let mut input = gate_input(&lifecycle);
        input.next_states.pop();
        let gate = gate_protocol_session_lifecycle(input).expect("gate missing next state");
        assert_eq!(gate.decision, "deny");
        assert!(
            gate.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("next state") || diagnostic.contains("terminal"))
        );
    }

    #[test]
    fn bad_payload_tag_and_replay_deny() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let server = lifecycle.initial_states[1].clone();
        let request = lifecycle.operations[0].message.as_ref().expect("request").clone();
        let bad_message = protocol_message_value(&ProtocolMessageInput {
            protocol_ref: request.protocol_ref,
            session_id: request.session_id,
            from_role: request.from_role,
            to_role: request.to_role,
            label: request.label,
            payload_tag: "response".to_string(),
            body_or_ref: request.body_or_ref,
            sequence: request.sequence,
            evidence_refs: Vec::new(),
        })
        .expect("bad tag message");
        let bad = receive_protocol_message(ProtocolReceiveInput {
            state: server.value,
            message: bad_message,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("bad tag deny");
        assert_eq!(bad.decision, "deny");

        let after_receive = lifecycle.operations[1].next_state.as_ref().expect("next state").clone();
        let replay = receive_protocol_message(ProtocolReceiveInput {
            state: after_receive.value,
            message: request.value,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("replay deny");
        assert_eq!(replay.decision, "deny");
    }

    #[test]
    fn branch_choice_and_offer_follow_projected_state() {
        let left = ProtocolBranchInput {
            label: "left".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "left".to_string(),
                payload_tag: "left".to_string(),
            }],
        };
        let right = ProtocolBranchInput {
            label: "right".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "right".to_string(),
                payload_tag: "right".to_string(),
            }],
        };
        let global = protocol_global_choice_value(&ProtocolChoiceInput {
            decider: "client".to_string(),
            branches: vec![left, right],
        })
        .expect("choice global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:choice".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels: vec!["left".to_string(), "right".to_string()],
            payloads: vec![
                ProtocolPayloadInput {
                    tag: "left".to_string(),
                    schema_ref: test_ref("left-schema"),
                },
                ProtocolPayloadInput {
                    tag: "right".to_string(),
                    schema_ref: test_ref("right-schema"),
                },
            ],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("choice manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install choice");
        assert_eq!(install.decision, "pass");
        let client = start_protocol_session(&install, "client", "session:choice", auth(), resources()).expect("client");
        let server = start_protocol_session(&install, "server", "session:choice", auth(), resources()).expect("server");
        let branch = choose_protocol_branch(ProtocolBranchOperationInput {
            state: client.value,
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("choose branch");
        assert_eq!(branch.decision, "pass");
        let offer = offer_protocol_branch(ProtocolBranchOperationInput {
            state: server.value,
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("offer branch");
        assert_eq!(offer.decision, "pass");
    }

    #[test]
    fn protocol_message_semantics_are_transport_neutral() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let server = lifecycle.initial_states[1].clone();
        let request = lifecycle.operations[0].message.as_ref().expect("request").clone();
        let local = receive_protocol_message(ProtocolReceiveInput {
            state: server.value.clone(),
            message: request.value.clone(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("local receive");
        let envelope = protocol_message_remote_envelope(ProtocolRemoteEnvelopeInput {
            from_peer: "peer:a".to_string(),
            from_actor: "client".to_string(),
            to_peer: "peer:b".to_string(),
            topic: "protocols".to_string(),
            message: request.value,
            capability_refs: vec![test_ref("capability")],
            evidence_refs: vec![test_ref("carrier-evidence")],
        })
        .expect("remote envelope");
        let remote = receive_protocol_message(ProtocolReceiveInput {
            state: server.value,
            message: envelope.payload,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: vec![envelope.envelope_ref],
        })
        .expect("remote receive");
        assert_eq!(
            local.next_state.expect("local next").local_state,
            remote.next_state.expect("remote next").local_state
        );
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_protocol_records() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let gate = gate_protocol_session_lifecycle(gate_input(&lifecycle)).expect("protocol session gate");
        assert_eq!(ledger::artifact_kind(&lifecycle.manifest_value), "protocol-manifest");
        assert_eq!(ledger::artifact_kind(&lifecycle.install.value), "protocol-install-receipt");
        assert_eq!(ledger::artifact_kind(&gate.value), "protocol-session-gate-receipt");
        assert_eq!(ledger::artifact_kind(&lifecycle.install.endpoints[0].value), "protocol-endpoint");
        assert_eq!(ledger::artifact_kind(&lifecycle.initial_states[0].value), "protocol-session-state");
        assert_eq!(
            ledger::artifact_kind(&lifecycle.operations[0].message.as_ref().expect("message").value),
            "protocol-message"
        );
        assert_eq!(ledger::artifact_kind(&lifecycle.operations[0].receipt.value), "protocol-operation-receipt");
        let dir = temp_dir("catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let imported = ledger::import_artifact(&ledger_root, &lifecycle.install.value).expect("ledger import");
        assert_eq!(imported.artifact_kind, "protocol-install-receipt");
        let listed = catalog::list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("protocol-install-receipt".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let request = catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "protocol-install-receipt",
        )])])
        .expect("mcp request");
        let mcp = catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render mcp").contains("protocol-install-receipt"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_generated_linear_protocols_install_and_roundtrip(tc: TestCase) {
        let step_count = usize::try_from(tc.draw(generators::integers::<u64>().min_value(1).max_value(3)))
            .expect("usize step count");
        let mut steps = Vec::with_capacity(step_count);
        let mut labels = Vec::with_capacity(step_count);
        let mut payloads = Vec::with_capacity(step_count);
        for index in 0..step_count {
            let label = format!("l{index}");
            labels.push(label.clone());
            payloads.push(ProtocolPayloadInput {
                tag: label.clone(),
                schema_ref: test_ref(&format!("schema-{index}")),
            });
            let is_even = index % 2 == 0;
            let (from_role, to_role) = if is_even {
                ("client", "server")
            } else {
                ("server", "client")
            };
            steps.push(ProtocolCommInput {
                from_role: from_role.to_string(),
                to_role: to_role.to_string(),
                label: label.clone(),
                payload_tag: label,
            });
        }
        let global = protocol_global_script_value(&steps).expect("generated global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:generated".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels,
            payloads,
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("generated manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("generated install");
        assert_eq!(install.decision, "pass");
        assert_eq!(install.endpoints.len(), 2);
        let parsed = parse_protocol_install_receipt(&install.value).expect("parse install receipt");
        assert_eq!(parsed.receipt_ref, install.receipt_ref);
    }
}
