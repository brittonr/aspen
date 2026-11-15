// Fixed finite identities and acknowledgement/promise hazards used by the replay driver.
fn operation(command: &str) -> molten::fabric_consistency::fastpath::OperationIdentity {
    molten::fabric_consistency::fastpath::OperationIdentity {
        command_ref: command.to_owned(),
        session_ref: "model-session".to_owned(),
        session_sequence: if command == "c1" { 1 } else { 2 },
        group_ref: "model-group".to_owned(),
        extension_generation: 1,
        application_schema_ref: "model-schema".to_owned(),
        policy_ref: "model-policy".to_owned(),
        authority_ref: "model-authority".to_owned(),
        resource_ref: "model-resource".to_owned(),
        engine_epoch: 1,
    }
}

fn attempt(
    command: &str,
    ack_mode: &str,
    promise_mode: &str,
    conflict_free: bool,
    state: &molten::fabric_consistency::fastpath::RecoveryState,
) -> molten::fabric_consistency::fastpath::StableViewAttempt {
    let operation = operation(command);
    let mut original_operation = operation.clone();
    if ack_mode == "identity" {
        original_operation.engine_epoch += 1;
    }
    let acknowledgements = (0..if ack_mode == "short" { 2 } else { 3 })
        .map(|index| molten::fabric_consistency::fastpath::FastAcknowledgement {
            replica_id: format!("r{index}"),
            acceleration_view: if ack_mode == "mixed" && index == 2 {
                0
            } else {
                state.acceleration_view
            },
            base_view: state.base_view,
            operation: operation.clone(),
        })
        .collect();
    let promises = if promise_mode == "missing" {
        Vec::new()
    } else {
        vec![molten::fabric_consistency::fastpath::ProposerPromise {
            proposer_id: "proposer".to_owned(),
            acceleration_view: if promise_mode == "mixed" {
                0
            } else {
                state.acceleration_view
            },
            base_view: state.base_view,
            proposal_order_preserved: promise_mode != "reorder",
        }]
    };
    molten::fabric_consistency::fastpath::StableViewAttempt {
        operation,
        original_operation,
        acceleration_view: state.acceleration_view,
        base_view: state.base_view,
        conflict_free,
        acknowledgements,
        promises,
        active_proposers: std::collections::BTreeSet::from(["proposer".to_owned()]),
        original_path_available: true,
    }
}
