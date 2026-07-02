
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

fn protocol_session_gate_receipt_value(input: &ProtocolSessionGateValueInput<'_>) -> Result<IoValue> {
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

fn parse_protocol_global(value: &IoValue) -> Result<ProtocolGlobal> {
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

fn parse_step_sequence(value: &IoValue, label: &str) -> Result<Vec<ProtocolCommInput>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    parse_comm_sequence_value(&fields[0], label)
}

fn parse_comm_sequence_value(value: &Value<IoValue>, label: &str) -> Result<Vec<ProtocolCommInput>> {
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

fn parse_comm_step(value: &Value<IoValue>) -> Result<ProtocolCommInput> {
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
