// Fixed finite identities and acknowledgement/promise hazards used by the replay driver.
// Five admitted replicas r0..r4 with the derived 4-of-5 superquorum; r0 is the active proposer.
const REPLICAS: usize = 5;
const PROPOSER: &str = "r0";

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
    // "short" leaves 3 of 5 acknowledgements, below the superquorum. "unacknowledged" omits the
    // admitted proposer r0, so the remaining replicas still form a 4-of-5 superquorum without it:
    // the `ConsensusFastPath.tla` missing-promise counterexample shape.
    let first = if promise_mode == "unacknowledged" { 1 } else { 0 };
    let end = if ack_mode == "short" { 3 } else { REPLICAS };
    let acknowledgements = (first..end)
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
            proposer_id: PROPOSER.to_owned(),
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
        replicas: (0..REPLICAS).map(|index| format!("r{index}")).collect(),
        acknowledgements,
        promises,
        active_proposers: std::collections::BTreeSet::from([PROPOSER.to_owned()]),
        original_path_available: true,
    }
}
