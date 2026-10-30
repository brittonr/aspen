use molten::system_extension::NativeHostJournal;

use crate::support::*;

// r[verify molten.system_extension.node_hosting]
#[test]
fn drained_native_checkpoint_keeps_ingress_closed() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(1).or_fail("start actual native callback service")?;
    service.drain(2).or_fail("drain actual native callback service")?;
    service.checkpoint(3).or_fail("checkpoint drained native service")?;
    let instance = service.instance().or_fail("checkpointed native instance")?;
    assert!(instance.checkpoint_ref.is_some());
    assert!(!instance.is_accepting_ingress, "checkpoint must not reopen a drained member");
    let delivery = ingress(instance.lifecycle.generation, &instance.manifest_ref)?;
    assert!(service.ingress(&delivery, 4).is_err(), "drained checkpoint must not admit subsequent ingress");
    Ok(())
}

// r[verify molten.system_extension.node_hosting]
#[test]
fn native_replacement_executes_materialized_recovery_and_rejects_old_generation() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(1).or_fail("start actual native callback service")?;
    service.drain(2).or_fail("drain native generation")?;
    service.checkpoint(3).or_fail("checkpoint drained generation")?;
    let before = service.instance().or_fail("source native generation")?;
    let state_ref = before.state_ref.as_deref().or_fail("source materialized state reference")?;
    let prior = cohort
        .values
        .lock()
        .or_fail("source callback value lock")?
        .materialize(state_ref, MAX_VALUE_BYTES)
        .or_fail("materialize source semantic state")?;
    let port = molten::fabric_execution::Adapter::new(cohort.execution_profile.clone(), Publisher::default())
        .or_fail("replacement actual execution adapter")?;
    service
        .upgrade(
            molten::system_extension::Replacement {
                admission: molten::system_extension::AdmissionSet {
                    profile: cohort.native_profile.clone(),
                    executable: cohort.executable.clone(),
                    admitted: cohort.admitted.clone(),
                },
                port,
                values: cohort.values.clone(),
                template: cohort.template.clone(),
            },
            4,
        )
        .or_fail("actual native generation replacement")?;
    let after = service.instance().or_fail("replacement generation")?;
    let transformed = cohort
        .values
        .lock()
        .or_fail("replacement callback value lock")?
        .materialize(after.state_ref.as_deref().or_fail("transformed state reference")?, MAX_VALUE_BYTES)
        .or_fail("materialize actual transformed recovery state")?;
    let expected = [
        after.instance_id.as_bytes(),
        &after.lifecycle.generation.to_le_bytes(),
        &after.callback_sequence.to_le_bytes(),
        prior.bytes.as_slice(),
    ]
    .concat();
    assert_eq!(transformed.bytes, expected, "Recover must execute and publish its transformed materialized state");
    assert_eq!(after.lifecycle.phase, molten::system_extension::LifecyclePhase::Running);
    let stale = ingress(before.lifecycle.generation, &before.manifest_ref)?;
    assert!(service.ingress(&stale, 5).is_err(), "pre-upgrade ingress must lose influence");
    let current = ingress(after.lifecycle.generation, &after.manifest_ref)?;
    let result = service.ingress(&current, 6).or_fail("replacement accepts current-generation request")?;
    assert!(matches!(result.dispatch, molten::system_extension::DispatchResult::Executed { .. }));
    Ok(())
}

// r[verify molten.system_extension.node_hosting]
#[test]
fn native_restart_journals_the_new_fence_before_any_recovery_callback_intent() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(1).or_fail("start actual native service")?;
    service.checkpoint(2).or_fail("checkpoint native service")?;
    let before = service.instance().or_fail("pre-restart native instance")?;
    service.restart(3).or_fail("actual native recovery callback")?;
    let history = cohort
        .journal
        .lock()
        .or_fail("native journal lock")?
        .history(&before.instance_id)
        .or_fail("ordered native instance journal")?;
    let fenced_callback = history.iter().find(|record| {
        record.lifecycle.incarnation > before.lifecycle.incarnation
            && record.lifecycle.phase == molten::system_extension::LifecyclePhase::Recovering
            && record
                .unresolved
                .iter()
                .any(|operation| operation.kind == molten::system_extension::NativeOperationKind::Callback)
    });
    let record = fenced_callback.or_fail("durable fresh fence exists while callback intent is unresolved")?;
    assert!(!record.is_accepting_ingress, "fresh recovery fence must precede reopening ingress");
    let stale = molten::system_extension::WorkPresentation {
        work_ref: before.manifest_ref,
        class: molten::system_extension::WorkClass::RuntimeLocal,
        generation: before.lifecycle.generation,
        incarnation: before.lifecycle.incarnation,
    };
    assert!(molten::system_extension::admit_work_delivery(&record.lifecycle, &stale, None).is_err());
    Ok(())
}
