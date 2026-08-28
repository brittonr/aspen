use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use crate::fabric_execution::CanonicalExecutionProfile;
use crate::fabric_execution::CanonicalExecutionReceipt;
use crate::fabric_execution::CanonicalExecutionRequest;
use crate::fabric_execution::ExecutionFabricPort;
use crate::fabric_execution::ExecutionOutputPublisher;
use crate::fabric_execution::ExecutionPortResult;
use crate::fabric_execution::ExecutionProfileKind;
use crate::fabric_execution::ExecutionReconciliationStatus;
use crate::fabric_execution::LiveExecutionAdapter;
use crate::fabric_execution::ResolvedExecutionContext;
use crate::fabric_execution::ScriptedExecutionObservation;
use crate::fabric_execution::SimulatedExecutionAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemExtensionExecutionFabricSelectionError {
    LiveProfileHasSimulationScripts,
    AdapterProfileMismatch,
}

pub enum SystemExtensionExecutionFabric<P: ExecutionOutputPublisher> {
    Live(LiveExecutionAdapter<P>),
    Simulation(SimulatedExecutionAdapter<P>),
}

// r[impl molten.fabric_execution.port_contract]
pub fn compose_system_extension_execution_fabric<P: ExecutionOutputPublisher>(
    profile: CanonicalExecutionProfile,
    publisher: P,
    scripts: BTreeMap<String, ScriptedExecutionObservation>,
) -> Result<SystemExtensionExecutionFabric<P>, SystemExtensionExecutionFabricSelectionError> {
    match profile.profile.descriptor.kind {
        ExecutionProfileKind::LiveBoundedProcess => {
            if !scripts.is_empty() {
                return Err(SystemExtensionExecutionFabricSelectionError::LiveProfileHasSimulationScripts);
            }
            let adapter = LiveExecutionAdapter::new(profile, publisher)
                .map_err(|_| SystemExtensionExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(SystemExtensionExecutionFabric::Live(adapter))
        }
        ExecutionProfileKind::DeterministicSimulation => {
            let adapter = SimulatedExecutionAdapter::new(profile, publisher, scripts)
                .map_err(|_| SystemExtensionExecutionFabricSelectionError::AdapterProfileMismatch)?;
            Ok(SystemExtensionExecutionFabric::Simulation(adapter))
        }
    }
}

impl<P: ExecutionOutputPublisher> ExecutionFabricPort for SystemExtensionExecutionFabric<P> {
    fn profile(&self) -> &CanonicalExecutionProfile {
        match self {
            Self::Live(adapter) => adapter.profile(),
            Self::Simulation(adapter) => adapter.profile(),
        }
    }

    fn execute(
        &mut self,
        request: &CanonicalExecutionRequest,
        resolved: &ResolvedExecutionContext,
        cancellation: Option<&AtomicBool>,
    ) -> ExecutionPortResult<CanonicalExecutionReceipt> {
        match self {
            Self::Live(adapter) => adapter.execute(request, resolved, cancellation),
            Self::Simulation(adapter) => adapter.execute(request, resolved, cancellation),
        }
    }

    fn reconcile(&self, operation_ref: &str, generation: u64) -> ExecutionReconciliationStatus {
        match self {
            Self::Live(adapter) => adapter.reconcile(operation_ref, generation),
            Self::Simulation(adapter) => adapter.reconcile(operation_ref, generation),
        }
    }
}
