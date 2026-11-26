// Controls: the native-work invariants must not hold when no child process
// started or when delivery fails with anything other than uncertainty.
// r[verify molten.system_extension.native_effect_uncertainty]
// r[verify molten.fabric_execution.uncertainty]
#[test]
fn no_started_child_or_untyped_failure_is_not_uncertain_effect_evidence() {
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0_u64));
    let mut runtime = super::host::ProcessRuntime::new(attempts.clone()).expect("effect runtime");
    let missing = runtime
        .effect
        .execute(super::MISSING_PROGRAM, &runtime.effect.resolved(super::MISSING_PROGRAM))
        .expect("pre-start failure");
    assert_eq!(missing.kind, crate::fabric_execution::PortFailureKind::RejectedBeforeStart);
    assert!(!runtime.effect.take_child_started().expect("marker probe"));
    assert_eq!(attempts.get(), 0);
    let started = runtime
        .effect
        .execute(super::EFFECT_PROGRAM, &runtime.effect.resolved(super::EFFECT_PROGRAM))
        .expect("post-start failure");
    assert_eq!(started.kind, crate::fabric_execution::PortFailureKind::UnknownAfterStart);
    assert!(runtime.effect.take_child_started().expect("marker probe"));
    assert!(!super::host::is_uncertain(&Err(crate::error::Failure::invalid_harness("untyped failure"))));
    assert!(!super::host::is_uncertain(&Ok(())));
    assert!(super::host::is_uncertain(&Err(crate::error::Failure::classified(
        crate::error::FailureClass::UncertainExternalOutcome,
        "uncertain",
    ))));
}
