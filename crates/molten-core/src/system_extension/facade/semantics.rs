use super::super::LifecycleEvent;
use super::super::LifecycleEventKind;
use super::AdmittedFacadeManifest;
use super::FacadeProfile;
use super::FacadeRuntimeState;

/// The phase a completed task reports. A task that completes successfully
/// stops; it does not consume restart budget.
pub const TASK_COMPLETED_PHASE: &str = "stopped";

/// The event a task facade presents to complete its work.
pub const TASK_COMPLETE_EVENT: &str = "complete";

/// Whether a facade profile admits a phase table at all.
pub const fn facade_uses_phase_table(profile: FacadeProfile) -> bool {
    matches!(profile, FacadeProfile::StateMachine)
}

/// Whether a facade profile ends in a completed, stopped state.
pub const fn facade_completes(profile: FacadeProfile) -> bool {
    matches!(profile, FacadeProfile::Task)
}

/// Whether a facade profile postpones events instead of rejecting them.
pub const fn facade_postpones(profile: FacadeProfile) -> bool {
    matches!(profile, FacadeProfile::StateMachine)
}

/// The task completion decision for one admitted turn.
///
/// A successful completion moves the task to the stopped phase without
/// consuming the restart budget, and any further event is rejected because the
/// task is already complete.
pub fn task_completion_decision(facade: &AdmittedFacadeManifest, completed: bool) -> Option<&'static str> {
    if facade.profile != FacadeProfile::Task {
        return None;
    }
    if completed { Some(TASK_COMPLETED_PHASE) } else { None }
}

/// The lifecycle events that stop a completed task.
///
/// A task that completes successfully maps to the existing stopped transition.
/// It is not a failure, so it consumes no restart budget.
pub fn task_completion_events(state: &FacadeRuntimeState) -> Vec<LifecycleEvent> {
    vec![
        LifecycleEvent::simple(LifecycleEventKind::BeginShutdown, state.generation, state.incarnation),
        LifecycleEvent::simple(LifecycleEventKind::ShutdownSucceeded, state.generation, state.incarnation),
    ]
}
