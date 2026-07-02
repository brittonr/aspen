
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

fn parse_protocol_endpoint(value: &IoValue) -> Result<ProtocolEndpoint> {
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

fn protocol_local_state_value(state: &ProtocolLocalState) -> Result<IoValue> {
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

fn parse_protocol_local_state(value: &IoValue) -> Result<ProtocolLocalState> {
    let fields = value
        .collect_simple_record("protocol-local-state-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-local-state-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_LOCAL_STATE_SCHEMA, "protocol local state schema")?;
    Ok(ProtocolLocalState {
        actions: parse_local_actions(&fields[1])?,
        terminal: parse_local_terminal(&fields[2])?,
    })
}

fn local_action_value(action: &ProtocolLocalAction) -> Result<IoValue> {
    validate_direction(&action.direction)?;
    validate_name(&action.peer, "protocol local action peer")?;
    validate_name(&action.label, "protocol local action label")?;
    validate_name(&action.payload_tag, "protocol local action payload")?;
    let record_label = if action.direction == "send" { "send" } else { "recv" };
    Ok(record(record_label, vec![string(&action.peer), string(&action.label), string(&action.payload_tag)]))
}

fn local_terminal_value(terminal: &ProtocolLocalTerminal) -> Result<IoValue> {
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

fn local_branch_sequence(branches: &[ProtocolLocalBranch]) -> Result<IoValue> {
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

fn parse_local_actions(value: &Value<IoValue>) -> Result<Vec<ProtocolLocalAction>> {
    let fields = value
        .collect_simple_record("actions", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local actions"))?;
    parse_local_action_sequence(&fields[0])
}

fn parse_local_action_sequence(value: &Value<IoValue>) -> Result<Vec<ProtocolLocalAction>> {
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

fn parse_local_action(value: &Value<IoValue>) -> Result<ProtocolLocalAction> {
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

fn parse_local_terminal(value: &Value<IoValue>) -> Result<ProtocolLocalTerminal> {
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
