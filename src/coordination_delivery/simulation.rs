use molten_core::coordination_delivery::*;

#[derive(Clone, Debug)]
pub enum DeliverySimulationAction {
    Request(DeliveryRequest),
    FaultedRequest {
        fault: molten_core::fabric_simulation::SimulationFaultKind,
        request: DeliveryRequest,
    },
    CrashRestart,
}

#[derive(Clone, Debug)]
pub struct DeliverySimulationTrace {
    pub transitions: Vec<DeliveryTransition>,
    pub state_refs: Vec<String>,
    pub fault_classes: Vec<String>,
    pub final_state: DeliveryState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliverySimulationError {
    StateCodec,
    UnsupportedFault(molten_core::fabric_simulation::SimulationFaultKind),
}

/// A crash-restart action records both a crash and a restart fault class; other actions record at
/// most one.
const MAX_FAULT_CLASSES_PER_ACTION: usize = 2;

// r[impl molten.coordination_delivery.final_validation]
pub fn run_delivery_simulation(
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    time_profile: &molten_core::fabric_time::AdmittedTimeProfile,
    initial: DeliveryState,
    actions: &[DeliverySimulationAction],
) -> Result<DeliverySimulationTrace, DeliverySimulationError> {
    let mut state = initial;
    let mut transitions = Vec::new();
    let mut state_refs = vec![identify_delivery_state(&state)];
    let mut fault_classes = Vec::with_capacity(actions.len().saturating_mul(MAX_FAULT_CLASSES_PER_ACTION));
    for action in actions {
        match action {
            DeliverySimulationAction::Request(request) => {
                apply_simulated_request(
                    manifest,
                    policy,
                    time_profile,
                    request,
                    &mut state,
                    &mut transitions,
                    &mut state_refs,
                );
            }
            DeliverySimulationAction::FaultedRequest { fault, request } => {
                let mut faulted = request.clone();
                match fault {
                    molten_core::fabric_simulation::SimulationFaultKind::Partition
                    | molten_core::fabric_simulation::SimulationFaultKind::ConsistencyQuorumLoss => {
                        faulted.currentness = DeliveryCurrentness::LocalStale;
                    }
                    molten_core::fabric_simulation::SimulationFaultKind::AuthorityRevocation => {
                        faulted.authority_refs.clear();
                    }
                    molten_core::fabric_simulation::SimulationFaultKind::CapacityExhaustion => {
                        faulted.resource_refs.clear();
                    }
                    molten_core::fabric_simulation::SimulationFaultKind::Duplicate => {}
                    other => {
                        return Err(DeliverySimulationError::UnsupportedFault(*other));
                    }
                }
                apply_simulated_request(
                    manifest,
                    policy,
                    time_profile,
                    &faulted,
                    &mut state,
                    &mut transitions,
                    &mut state_refs,
                );
                if *fault == molten_core::fabric_simulation::SimulationFaultKind::Duplicate {
                    apply_simulated_request(
                        manifest,
                        policy,
                        time_profile,
                        &faulted,
                        &mut state,
                        &mut transitions,
                        &mut state_refs,
                    );
                }
                fault_classes.push(fault.as_str().to_string());
            }
            DeliverySimulationAction::CrashRestart => {
                let bytes = serde_json::to_vec(&state).map_err(|_| DeliverySimulationError::StateCodec)?;
                state = serde_json::from_slice(&bytes).map_err(|_| DeliverySimulationError::StateCodec)?;
                state_refs.push(identify_delivery_state(&state));
                fault_classes.push(molten_core::fabric_simulation::SimulationFaultKind::Crash.as_str().to_string());
                fault_classes.push(molten_core::fabric_simulation::SimulationFaultKind::Restart.as_str().to_string());
            }
        }
    }
    Ok(DeliverySimulationTrace {
        transitions,
        state_refs,
        fault_classes,
        final_state: state,
    })
}

fn apply_simulated_request(
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    time_profile: &molten_core::fabric_time::AdmittedTimeProfile,
    request: &DeliveryRequest,
    state: &mut DeliveryState,
    transitions: &mut Vec<DeliveryTransition>,
    state_refs: &mut Vec<String>,
) {
    let transition = plan_delivery_transition(&DeliveryTransitionInput {
        manifest,
        policy,
        time_profile,
        state,
        request,
    });
    *state = transition.next_state.clone();
    state_refs.push(transition.after_state_ref.clone());
    transitions.push(transition);
}
