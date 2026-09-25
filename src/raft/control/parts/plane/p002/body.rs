
pub fn parse_control_registry_state(value: &IoValue) -> Result<ControlRegistryState> {
    let fields = value
        .collect_simple_record("control-registry-state-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-state-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::CONTROL_REGISTRY_STATE_SCHEMA, "control registry state schema")?;
    let entries = parse_registry_entries(&fields[1])?;
    let client_sessions = parse_client_sessions(&fields[2])?;
    require_check(&parse_checks(&fields[3])?, "deterministic-map-order", "control registry state")?;
    Ok(ControlRegistryState {
        state_ref: canonical_hash(value)?,
        entries,
        client_sessions,
        value: value.clone(),
    })
}

// r[impl molten.consensus.leaderless_profile_boundary]
// r[impl molten.consensus.runtime_engine_selection]
// r[impl molten.consensus.algorithm_profile_manifest]
/// Constructs the legacy deterministic model runtime.
///
/// This compatibility entry point is intentionally model-only. Production
/// callers must use `new_control_registry_production_runtime`, which fails
/// closed until a live implementation profile has accepted evidence.
pub fn new_control_registry_runtime(manifest_value: &IoValue) -> Result<ControlRegistryRuntime> {
    new_control_registry_model_runtime(manifest_value)
}

pub fn new_control_registry_model_runtime(manifest_value: &IoValue) -> Result<ControlRegistryRuntime> {
    new_control_registry_runtime_for_environment(manifest_value, CONSENSUS_ENVIRONMENT_MODEL)
}

pub fn new_control_registry_production_runtime(
    manifest_value: &IoValue,
) -> Result<ControlRegistryRuntime> {
    new_control_registry_runtime_for_environment(manifest_value, CONSENSUS_ENVIRONMENT_PRODUCTION)
}

fn new_control_registry_runtime_for_environment(
    manifest_value: &IoValue,
    environment: &str,
) -> Result<ControlRegistryRuntime> {
    let manifest = parse_raft_group_manifest(manifest_value)?;
    if manifest.state_machine != CONTROL_REGISTRY_STATE_MACHINE {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported raft state machine {}; expected {CONTROL_REGISTRY_STATE_MACHINE}",
            manifest.state_machine
        )));
    }
    let admission = if environment == CONSENSUS_ENVIRONMENT_MODEL {
        resolve_control_registry_model_engine(&manifest)?
    } else {
        resolve_control_registry_engine(&manifest)?
    };
    if admission.decision != ENGINE_DECISION_PASS {
        return Err(MoltenError::invalid_harness(format!(
            "consensus profile {} is not admitted for {environment} runtime; status {}; diagnostics {}",
            manifest.algorithm_profile,
            manifest.production_status,
            admission.diagnostics.join("; ")
        )));
    }
    initial_control_registry_runtime(manifest)
}

fn initial_control_registry_runtime(manifest: RaftGroupManifest) -> Result<ControlRegistryRuntime> {
    Ok(ControlRegistryRuntime {
        manifest,
        term: 1,
        committed_index: 0,
        last_log_ref: None,
        state: initial_control_registry_state()?,
        log_entries: Vec::new(),
        commit_receipts: Vec::new(),
        registry_receipts: Vec::new(),
        predicate_receipts: Vec::new(),
    })
}

pub fn propose_control_registry_command(
    runtime: &mut ControlRegistryRuntime,
    envelope_value: &IoValue,
) -> Result<ControlRegistryProposal> {
    let transition = propose_control_registry_transition_core(runtime, envelope_value)?;
    apply_control_registry_transition(runtime, &transition);
    Ok(transition.proposal)
}

// r[impl molten.consensus_engine_traits.pure_transition_core]
pub fn propose_control_registry_transition_core(
    runtime: &ControlRegistryRuntime,
    envelope_value: &IoValue,
) -> Result<ControlRegistryTransition> {
    let envelope = parse_raft_command_envelope(envelope_value)?;
    let (command, diagnostics) = admitted_command(&envelope.command);
    if let Some(duplicate) = duplicate_sequence(runtime, &envelope) {
        let proposal = duplicate_proposal(runtime, envelope, duplicate, command.as_ref())?;
        return Ok(denied_transition(runtime, proposal));
    }
    let admission = proposal_diagnostics(ProposalDecisionInput {
        runtime,
        envelope: &envelope,
        command: command.as_ref(),
        diagnostics,
    })?;
    if !admission.is_empty() {
        let commit_receipt = deny_commit_receipt(runtime, &envelope, "proposal-deny", &admission)?;
        let registry_receipt = deny_registry_receipt(runtime, &envelope, command.as_ref(), &admission)?;
        let proposal = ControlRegistryProposal {
            decision: "deny".to_string(),
            duplicate: false,
            envelope,
            predicates: Vec::new(),
            log_entry: None,
            commit_receipt,
            registry_receipt,
        };
        return Ok(denied_transition(runtime, proposal));
    }
    let command =
        command.ok_or_else(|| MoltenError::invalid_harness("missing admitted command after admission pass"))?;
    let draft = pass_draft(runtime, &envelope)?;
    let (state_after, registry_receipt) = apply_admitted_command_core(
        &runtime.state,
        &envelope,
        &command,
        &draft.log_entry,
    )?;
    let next_last_log_ref = Some(draft.log_entry.entry_ref.clone());
    Ok(ControlRegistryTransition {
        proposal: ControlRegistryProposal {
            decision: "pass".to_string(),
            duplicate: false,
            envelope,
            predicates: vec![
                draft.append_predicate,
                draft.commit_predicate,
                draft.advancement_predicate,
            ],
            log_entry: Some(draft.log_entry),
            commit_receipt: draft.commit_receipt,
            registry_receipt,
        },
        state_after: Some(state_after),
        next_committed_index: draft.next_index,
        next_last_log_ref,
    })
}

/// A replayed duplicate returns its prior receipt; a conflicting duplicate is denied with the prior command named.
fn duplicate_proposal(
    runtime: &ControlRegistryRuntime,
    envelope: RaftCommandEnvelope,
    duplicate: DuplicateSequence,
    command: Option<&ControlRegistryCommand>,
) -> Result<ControlRegistryProposal> {
    Ok(match duplicate {
        DuplicateSequence::Replay(existing) => {
            let commit_receipt = deny_commit_receipt(runtime, &envelope, "duplicate-client-sequence", &[])?;
            ControlRegistryProposal {
                decision: existing.decision.clone(),
                duplicate: true,
                envelope,
                predicates: Vec::new(),
                log_entry: None,
                commit_receipt,
                registry_receipt: existing,
            }
        }
        DuplicateSequence::Conflict(session) => {
            let diagnostics = vec![format!(
                "conflicting duplicate client sequence {} for {}; prior command {}",
                envelope.sequence, envelope.client_session, session.result_command_ref
            )];
            let commit_receipt = deny_commit_receipt(runtime, &envelope, "duplicate-client-sequence", &diagnostics)?;
            let registry_receipt = deny_duplicate_registry_receipt(runtime, &envelope, command, &diagnostics)?;
            ControlRegistryProposal {
                decision: "deny".to_string(),
                duplicate: true,
                envelope,
                predicates: Vec::new(),
                log_entry: None,
                commit_receipt,
                registry_receipt,
            }
        }
    })
}

// r[impl molten.consensus_engine_traits.imperative_shell]
fn apply_control_registry_transition(runtime: &mut ControlRegistryRuntime, transition: &ControlRegistryTransition) {
    if transition.proposal.decision != "pass" {
        return;
    }
    let Some(state_after) = transition.state_after.clone() else {
        return;
    };
    runtime.state = state_after;
    runtime.committed_index = transition.next_committed_index;
    runtime.last_log_ref = transition.next_last_log_ref.clone();
    if let Some(log_entry) = &transition.proposal.log_entry {
        runtime.log_entries.push(log_entry.clone());
    }
    runtime.commit_receipts.push(transition.proposal.commit_receipt.clone());
    runtime
        .predicate_receipts
        .extend(transition.proposal.predicates.iter().cloned());
    runtime.registry_receipts.push(transition.proposal.registry_receipt.clone());
}

fn denied_transition(runtime: &ControlRegistryRuntime, proposal: ControlRegistryProposal) -> ControlRegistryTransition {
    ControlRegistryTransition {
        proposal,
        state_after: None,
        next_committed_index: runtime.committed_index,
        next_last_log_ref: runtime.last_log_ref.clone(),
    }
}

// r[impl molten.consensus.read_consistency_modes]
pub fn read_control_registry(input: &ControlRegistryReadInput) -> Result<RaftReadReceipt> {
    validate_read_consistency_mode(&input.read_consistency_mode)?;
    let state = parse_control_registry_state(&input.state)?;
    let mut diagnostics = Vec::new();
    if input.authority_refs.is_empty() {
        diagnostics.push("missing read authority evidence".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("missing read resource evidence".to_string());
    }
    if input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE && input.read_index != input.committed_index {
        diagnostics
            .push(format!("stale read-index {}; expected committed index {}", input.read_index, input.committed_index));
    }
    validate_refs(&input.authority_refs, "raft read authority ref")?;
    validate_refs(&input.resource_refs, "raft read resource ref")?;
    let target = find_entry(&state, &input.namespace, &input.name).map(|entry| entry.target_ref.clone());
    if target.is_none() {
        diagnostics.push("control registry entry not found".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let predicate = if decision == "pass" && input.read_consistency_mode == READ_CONSISTENCY_LINEARIZABLE {
        Some(parse_predicate_receipt(&predicate_receipt_value(&PredicateReceiptInput {
            predicate: "trellis-read-index-freshness",
            decision,
            group_ref: &input.group_ref,
            term: input.committed_term,
            index: input.committed_index,
            subjects: std::slice::from_ref(&state.state_ref),
            diagnostics: &[],
            checks: &[("trellis-predicate", "pass"), ("read-index-current", "pass")],
        })?)?)
    } else {
        None
    };
    let receipt = read_receipt_value(&ReadReceiptValueInput {
        decision,
        group_ref: &input.group_ref,
        state_ref: &state.state_ref,
        committed_term: input.committed_term,
        committed_index: input.committed_index,
        namespace: &input.namespace,
        name: &input.name,
        target_ref: target.as_deref(),
        read_consistency_mode: &input.read_consistency_mode,
        read_index_predicate_ref: predicate.as_ref().map(|value| value.predicate_ref.as_str()),
        authority_refs: &input.authority_refs,
        resource_refs: &input.resource_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(RaftReadReceipt {
        receipt_ref: canonical_hash(&receipt)?,
        decision: decision.to_string(),
        read_consistency_mode: input.read_consistency_mode.clone(),
        target_ref: target,
        diagnostics,
        value: receipt,
    })
}
