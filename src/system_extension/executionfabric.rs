use crate::fabric_execution::ExecutionFabricPort;
use crate::fabric_execution::ExecutionOutputPublisher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemExtensionExecutionFabricSelectionError {
    LiveProfileHasSimulationScripts,
    AdapterProfileMismatch,
}

pub enum SystemExtensionExecutionFabric<P: ExecutionOutputPublisher> {
    Live(crate::fabric_execution::LiveExecutionAdapter<P>),
    Simulation(crate::fabric_execution::SimulatedExecutionAdapter<P>),
}

// r[impl molten.fabric_execution.port_contract]
pub fn compose_system_extension_execution_fabric<P: ExecutionOutputPublisher>(
    profile: crate::fabric_execution::CanonicalExecutionProfile,
    publisher: P,
    scripts: std::collections::BTreeMap<String, crate::fabric_execution::ScriptedExecutionObservation>,
) -> Result<SystemExtensionExecutionFabric<P>, SystemExtensionExecutionFabricSelectionError> {
    match profile.profile.descriptor.kind {
        crate::fabric_execution::ExecutionProfileKind::LiveBoundedProcess => {
            if !scripts.is_empty() {
                return Err(SystemExtensionExecutionFabricSelectionError::LiveProfileHasSimulationScripts);
            }
            let adapter = crate::fabric_execution::LiveExecutionAdapter::new(profile, publisher)
                .map_err(|_| SystemExtensionExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(SystemExtensionExecutionFabric::Live(adapter))
        }
        crate::fabric_execution::ExecutionProfileKind::DeterministicSimulation => {
            let adapter = crate::fabric_execution::SimulatedExecutionAdapter::new(profile, publisher, scripts)
                .map_err(|_| SystemExtensionExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(SystemExtensionExecutionFabric::Simulation(adapter))
        }
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for SystemExtensionExecutionFabric<P> {
    fn profile(&self) -> &crate::fabric_execution::CanonicalExecutionProfile {
        match self {
            Self::Live(adapter) => adapter.profile(),
            Self::Simulation(adapter) => adapter.profile(),
        }
    }

    fn execute(
        &mut self,
        request: &crate::fabric_execution::CanonicalExecutionRequest,
        resolved: &crate::fabric_execution::ResolvedExecutionContext,
        cancellation: Option<&std::sync::atomic::AtomicBool>,
    ) -> crate::fabric_execution::ExecutionPortResult<crate::fabric_execution::CanonicalExecutionReceipt> {
        match self {
            Self::Live(adapter) => adapter.execute(request, resolved, cancellation),
            Self::Simulation(adapter) => adapter.execute(request, resolved, cancellation),
        }
    }

    fn reconcile(
        &self,
        operation_ref: &str,
        generation: u64,
    ) -> crate::fabric_execution::ExecutionReconciliationStatus {
        match self {
            Self::Live(adapter) => adapter.reconcile(operation_ref, generation),
            Self::Simulation(adapter) => adapter.reconcile(operation_ref, generation),
        }
    }
}
