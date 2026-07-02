
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOperationRun {
    pub decision: String,
    pub message: Option<ProtocolMessage>,
    pub next_state: Option<ProtocolSessionState>,
    pub receipt: ProtocolOperationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSendInput {
    pub state: IoValue,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IoValue,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolReceiveInput {
    pub state: IoValue,
    pub message: IoValue,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub carrier_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolBranchOperationInput {
    pub state: IoValue,
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
    pub message: IoValue,
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

pub fn protocol_comm_value(input: &ProtocolCommInput) -> Result<IoValue> {
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

pub fn protocol_global_script_value(steps: &[ProtocolCommInput]) -> Result<IoValue> {
    ensure_count_at_most(steps.len(), MAX_PROTOCOL_STEPS, "protocol script steps")?;
    let mut values = Vec::with_capacity(steps.len());
    for step in steps {
        values.push(protocol_comm_value(step)?);
    }
    Ok(record("global-script", vec![sequence(values)]))
}

pub fn protocol_global_choice_value(input: &ProtocolChoiceInput) -> Result<IoValue> {
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

pub fn protocol_manifest_value(input: &ProtocolManifestInput) -> Result<IoValue> {
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

pub fn parse_protocol_manifest(value: &IoValue) -> Result<ProtocolManifest> {
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

pub fn install_protocol_manifest_value(value: &IoValue) -> Result<ProtocolInstallReceipt> {
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

pub fn protocol_session_state_value(input: &ProtocolSessionStateInput) -> Result<IoValue> {
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

pub fn parse_protocol_session_state(value: &IoValue) -> Result<ProtocolSessionState> {
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
