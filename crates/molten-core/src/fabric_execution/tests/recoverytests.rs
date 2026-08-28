use super::*;

// r[verify molten.fabric_execution.uncertainty]
#[test]
fn recovery_distinguishes_prestart_terminal_unknown_and_stale() {
    let identity = request().identity();
    let base = ExecutionRecoveryFacts {
        identity: identity.clone(),
        active_generation: GENERATION,
        intent_committed: true,
        start_observed: false,
        terminal_observed: false,
        teardown_observed: false,
    };
    assert_eq!(classify_execution_recovery(&base), ExecutionRecoveryDecision::DefiniteNotStarted);
    assert_eq!(
        classify_execution_recovery(&ExecutionRecoveryFacts {
            start_observed: true,
            ..base.clone()
        }),
        ExecutionRecoveryDecision::UnknownRequiresReconciliation
    );
    assert_eq!(
        classify_execution_recovery(&ExecutionRecoveryFacts {
            start_observed: true,
            terminal_observed: true,
            teardown_observed: true,
            ..base.clone()
        }),
        ExecutionRecoveryDecision::Terminal
    );
    assert_eq!(
        classify_execution_recovery(&ExecutionRecoveryFacts {
            identity: ExecutionIdentity {
                generation: STALE_GENERATION,
                ..identity
            },
            ..base
        }),
        ExecutionRecoveryDecision::Stale
    );
}
