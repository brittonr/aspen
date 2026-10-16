use crate::fabric_execution::ExecutionFabricPort;
use crate::fabric_execution::ExecutionOutputPublisher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionFabricSelectionError {
    LiveProfileHasSimulationScripts,
    AdapterProfileMismatch,
}

pub enum ExecutionFabric<P: ExecutionOutputPublisher> {
    Live(crate::fabric_execution::Adapter<P>),
    Simulation(crate::fabric_execution::SimulatedAdapter<P>),
}

// r[impl molten.fabric_execution.port_contract]
pub fn compose_execution_fabric<P: ExecutionOutputPublisher>(
    profile: crate::fabric_execution::Profile,
    publisher: P,
    scripts: std::collections::BTreeMap<String, crate::fabric_execution::ScriptedObservation>,
) -> Result<ExecutionFabric<P>, ExecutionFabricSelectionError> {
    match profile.profile.descriptor.kind {
        crate::fabric_execution::ExecutionProfileKind::LiveBoundedProcess => {
            if !scripts.is_empty() {
                return Err(ExecutionFabricSelectionError::LiveProfileHasSimulationScripts);
            }
            let adapter = crate::fabric_execution::Adapter::new(profile, publisher)
                .map_err(|_| ExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(ExecutionFabric::Live(adapter))
        }
        crate::fabric_execution::ExecutionProfileKind::DeterministicSimulation => {
            let adapter = crate::fabric_execution::SimulatedAdapter::new(profile, publisher, scripts)
                .map_err(|_| ExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(ExecutionFabric::Simulation(adapter))
        }
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for ExecutionFabric<P> {
    fn profile(&self) -> &crate::fabric_execution::Profile {
        match self {
            Self::Live(adapter) => adapter.profile(),
            Self::Simulation(adapter) => adapter.profile(),
        }
    }

    fn execute(
        &mut self,
        request: &crate::fabric_execution::Request,
        resolved: &crate::fabric_execution::ResolvedContext,
        cancellation: Option<&std::sync::atomic::AtomicBool>,
    ) -> crate::fabric_execution::PortResult<crate::fabric_execution::Receipt> {
        match self {
            Self::Live(adapter) => adapter.execute(request, resolved, cancellation),
            Self::Simulation(adapter) => adapter.execute(request, resolved, cancellation),
        }
    }

    fn reconcile(&self, operation_ref: &str, generation: u64) -> crate::fabric_execution::ReconciliationStatus {
        match self {
            Self::Live(adapter) => adapter.reconcile(operation_ref, generation),
            Self::Simulation(adapter) => adapter.reconcile(operation_ref, generation),
        }
    }
}
