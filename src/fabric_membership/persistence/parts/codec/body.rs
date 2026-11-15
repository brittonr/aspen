fn assignment_value(assignment: &crate::fabric_membership::RoleAssignment) -> Value {
    crate::preserves_rail::record(ASSIGNMENT_RECORD, vec![
        crate::preserves_rail::string(&assignment.schema),
        crate::preserves_rail::string(&assignment.assignment_id),
        crate::preserves_rail::string(&assignment.extension_id),
        crate::preserves_rail::string(&assignment.service_id),
        crate::preserves_rail::string(&assignment.role_id),
        crate::preserves_rail::string(&assignment.role_kind),
        crate::preserves_rail::string(&assignment.node_id),
        crate::preserves_rail::u64_value(assignment.service_generation),
        crate::preserves_rail::u64_value(assignment.assignment_epoch),
        crate::preserves_rail::u64_value(assignment.fencing_token),
        crate::preserves_rail::string(&assignment.fencing_profile_ref),
        crate::preserves_rail::string(&assignment.resource_reservation_ref),
        crate::preserves_rail::string(&assignment.placement_plan_ref),
        crate::preserves_rail::string(&assignment.authority_ref),
        optional_text(assignment.durable_state_ref.as_deref()),
        optional_text(assignment.predecessor_assignment_ref.as_deref()),
        crate::preserves_rail::sequence(assignment.predecessor_epoch.into_iter().map(crate::preserves_rail::u64_value).collect()),
        crate::preserves_rail::string(assignment.state.as_str()),
        crate::preserves_rail::bool_value(assignment.uncertain_old_owner),
        crate::preserves_rail::sequence(assignment.transition_refs.iter().map(crate::preserves_rail::string).collect()),
    ])
}

fn decode_assignment(value: &Value) -> crate::fabric::FabricPortResult<crate::fabric_membership::RoleAssignment> {
    let values = fields(value, ASSIGNMENT_RECORD, 20)?;
    let predecessor = items(&values[16], 1)?;
    let transitions = items(&values[19], crate::fabric_membership::MAX_MEMBERSHIP_ITEMS)?;
    let assignment = crate::fabric_membership::RoleAssignment {
        schema: text(&values[0])?,
        assignment_id: text(&values[1])?,
        extension_id: text(&values[2])?,
        service_id: text(&values[3])?,
        role_id: text(&values[4])?,
        role_kind: text(&values[5])?,
        node_id: text(&values[6])?,
        service_generation: integer(&values[7])?,
        assignment_epoch: integer(&values[8])?,
        fencing_token: integer(&values[9])?,
        fencing_profile_ref: text(&values[10])?,
        resource_reservation_ref: text(&values[11])?,
        placement_plan_ref: text(&values[12])?,
        authority_ref: text(&values[13])?,
        durable_state_ref: decode_optional_text(&values[14])?,
        predecessor_assignment_ref: decode_optional_text(&values[15])?,
        predecessor_epoch: predecessor.first().map(integer).transpose()?,
        state: state(&text(&values[17])?)?,
        uncertain_old_owner: values[18].as_boolean().ok_or_else(|| invalid("expected assignment boolean"))?,
        transition_refs: transitions.iter().map(text).collect::<crate::fabric::FabricPortResult<_>>()?,
    };
    let issues = crate::fabric_membership::validate_assignment(&assignment);
    if !issues.is_empty() {
        return Err(invalid(format!("invalid stored assignment: {issues:?}")));
    }
    Ok(assignment)
}

fn fields(value: &Value, label: &str, count: usize) -> crate::fabric::FabricPortResult<Vec<Value>> {
    let values = value
        .collect_simple_record(label, Some(count))
        .ok_or_else(|| invalid(format!("expected {label}")))?;
    Ok(values.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn items(value: &Value, maximum: usize) -> crate::fabric::FabricPortResult<Vec<Value>> {
    let values = value.collect_sequence().ok_or_else(|| invalid("expected assignment sequence"))?;
    if values.len() > maximum {
        return Err(invalid("assignment sequence exceeds bound"));
    }
    Ok(values.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn text(value: &Value) -> crate::fabric::FabricPortResult<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| invalid("expected assignment string"))
}

fn integer(value: &Value) -> crate::fabric::FabricPortResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| invalid("expected assignment integer"))?
        .map_err(|_| invalid("assignment integer out of range"))
}

fn optional_text(value: Option<&str>) -> Value {
    crate::preserves_rail::sequence(value.into_iter().map(crate::preserves_rail::string).collect())
}

fn decode_optional_text(value: &Value) -> crate::fabric::FabricPortResult<Option<String>> {
    items(value, 1)?.first().map(text).transpose()
}

fn state(value: &str) -> crate::fabric::FabricPortResult<crate::fabric_membership::AssignmentState> {
    [
        crate::fabric_membership::AssignmentState::Proposed,
        crate::fabric_membership::AssignmentState::Reserved,
        crate::fabric_membership::AssignmentState::Assigned,
        crate::fabric_membership::AssignmentState::Acknowledged,
        crate::fabric_membership::AssignmentState::Active,
        crate::fabric_membership::AssignmentState::Draining,
        crate::fabric_membership::AssignmentState::Replacing,
        crate::fabric_membership::AssignmentState::Released,
        crate::fabric_membership::AssignmentState::Failed,
        crate::fabric_membership::AssignmentState::Quarantined,
    ]
    .into_iter()
    .find(|state| state.as_str() == value)
    .ok_or_else(|| invalid("unknown assignment state"))
}

fn command_kind(value: &str) -> crate::fabric::FabricPortResult<crate::fabric_membership::AssignmentCommandKind> {
    [
        crate::fabric_membership::AssignmentCommandKind::Reserve,
        crate::fabric_membership::AssignmentCommandKind::Assign,
        crate::fabric_membership::AssignmentCommandKind::Acknowledge,
        crate::fabric_membership::AssignmentCommandKind::Activate,
        crate::fabric_membership::AssignmentCommandKind::BeginDrain,
        crate::fabric_membership::AssignmentCommandKind::BeginReplacement,
        crate::fabric_membership::AssignmentCommandKind::Release,
        crate::fabric_membership::AssignmentCommandKind::Fail,
        crate::fabric_membership::AssignmentCommandKind::Quarantine,
    ]
    .into_iter()
    .find(|kind| kind.as_str() == value)
    .ok_or_else(|| invalid("unknown assignment command"))
}

fn valid_ref(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn invalid(message: impl Into<String>) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::malformed(message)
}
