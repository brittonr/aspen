use molten_core::coordination_delivery::*;
use molten_core::fabric_simulation::SimulationFaultKind;
use molten_core::fabric_time::AdmittedTimeProfile;

#[derive(Clone, Debug)]
pub enum DeliverySimulationAction {
    Request(DeliveryRequest),
    FaultedRequest {
        fault: SimulationFaultKind,
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
    UnsupportedFault(SimulationFaultKind),
}

// r[impl molten.coordination_delivery.final_validation]
pub fn run_delivery_simulation(
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    time_profile: &AdmittedTimeProfile,
    initial: DeliveryState,
    actions: &[DeliverySimulationAction],
) -> Result<DeliverySimulationTrace, DeliverySimulationError> {
    let mut state = initial;
    let mut transitions = Vec::new();
    let mut state_refs = vec![identify_delivery_state(&state)];
    let mut fault_classes = Vec::new();
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
                    SimulationFaultKind::Partition | SimulationFaultKind::ConsistencyQuorumLoss => {
                        faulted.currentness = DeliveryCurrentness::LocalStale;
                    }
                    SimulationFaultKind::AuthorityRevocation => {
                        faulted.authority_refs.clear();
                    }
                    SimulationFaultKind::CapacityExhaustion => {
                        faulted.resource_refs.clear();
                    }
                    SimulationFaultKind::Duplicate => {}
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
                if *fault == SimulationFaultKind::Duplicate {
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
                fault_classes.push(SimulationFaultKind::Crash.as_str().to_string());
                fault_classes.push(SimulationFaultKind::Restart.as_str().to_string());
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
    time_profile: &AdmittedTimeProfile,
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
