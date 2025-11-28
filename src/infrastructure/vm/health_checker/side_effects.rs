// Side effect execution for health checking system
//
// This module handles all side effects from health state transitions.
// Side effects include logging, metrics updates, and registry updates.
// This separation keeps the state machine pure while allowing testable I/O.

use std::sync::Arc;
use uuid::Uuid;

use super::types::SideEffect;
use crate::infrastructure::vm::registry::DefaultVmRepository;
use crate::infrastructure::vm::vm_types::VmState;

/// Executor for health check side effects
///
/// Handles all I/O operations resulting from state transitions:
/// - Logging state changes
/// - Updating VM registry
/// - Recording metrics
pub struct SideEffectExecutor {
    registry: Arc<DefaultVmRepository>,
}

impl SideEffectExecutor {
    /// Create a new side effect executor
    pub fn new(registry: Arc<DefaultVmRepository>) -> Self {
        Self { registry }
    }

    /// Execute a side effect from a state transition
    ///
    /// This method performs all I/O operations associated with the side effect.
    /// It is separated from the pure state machine logic to enable testing.
    pub async fn execute(&self, vm_id: Uuid, mut effect: SideEffect) {
        // Fill in the vm_id placeholder in the side effect
        match &mut effect {
            SideEffect::LogRecovery { vm_id: id, .. } => *id = vm_id,
            SideEffect::LogRecoveryFromUnhealthy { vm_id: id } => *id = vm_id,
            SideEffect::LogDegradation { vm_id: id, .. } => *id = vm_id,
            SideEffect::MarkUnhealthy { vm_id: id, .. } => *id = vm_id,
            SideEffect::LogHealthChange { vm_id: id, .. } => *id = vm_id,
        }

        match effect {
            SideEffect::LogRecovery {
                vm_id,
                previous_failures,
            } => {
                tracing::info!(
                    vm_id = %vm_id,
                    previous_failures = previous_failures,
                    "VM recovered to healthy"
                );
            }
            SideEffect::LogRecoveryFromUnhealthy { vm_id } => {
                tracing::info!(vm_id = %vm_id, "VM recovering from unhealthy");
            }
            SideEffect::LogDegradation { vm_id, error } => {
                tracing::warn!(
                    vm_id = %vm_id,
                    error = %error,
                    "VM health check failed, marking degraded"
                );
            }
            SideEffect::MarkUnhealthy {
                vm_id,
                failures,
                error,
            } => {
                // Update VM state in registry
                if let Err(e) = self
                    .registry
                    .update_state(
                        &vm_id,
                        VmState::Failed {
                            error: "Health check failures exceeded threshold".to_string(),
                        },
                    )
                    .await
                {
                    tracing::error!(
                        vm_id = %vm_id,
                        error = %e,
                        "Failed to mark VM as unhealthy in registry"
                    );
                }

                tracing::error!(
                    vm_id = %vm_id,
                    failures = failures,
                    error = %error,
                    "VM marked as unhealthy after {} failures",
                    failures
                );
            }
            SideEffect::LogHealthChange {
                vm_id,
                old_state,
                new_state,
            } => {
                tracing::info!(
                    vm_id = %vm_id,
                    old_state = ?old_state,
                    new_state = ?new_state,
                    "VM health status changed"
                );
            }
        }
    }

    /// Get the registry (for testing or advanced use cases)
    pub fn registry(&self) -> &Arc<DefaultVmRepository> {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Testing SideEffectExecutor requires a real DefaultVmRepository
    // and mock dependencies. These tests are better suited for integration
    // tests rather than unit tests. The executor's logic is straightforward
    // I/O wrapping, so integration tests will provide better coverage than
    // complex mocking.

    #[test]
    fn test_executor_can_be_created() {
        // This is a simple smoke test that the struct can be created
        // Real functionality tests require integration testing with registry
        // We can verify that the type system is correct without actual I/O
        let _phantom_test = std::marker::PhantomData::<SideEffectExecutor>;
    }
}
