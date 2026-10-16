
use molten::system_extension::*;
use support::*;

const START_TICK: u64 = 10;
const REQUEST_TICK: u64 = 20;
const EFFECT_COMPLETION_TICK: u64 = 25;
const CHECKPOINT_TICK: u64 = 30;
const RESTART_TICK: u64 = 40;
const DRAIN_TICK: u64 = 50;
const STOP_TICK: u64 = 60;
const STALE_GENERATION: u64 = GENERATION + 1;
const SHORT_TIMEOUT_MS: u64 = 50;
const NORMAL_TIMEOUT_MS: u64 = 5_000;
const SMALL_OUTPUT_BYTES: u64 = 4;
const FULL_OUTPUT_BYTES: u64 = 1_048_576;
const MAX_VALUE_BYTES_USIZE: usize = 262_144;
const OVERSIZED_VALUE_BYTES: usize = MAX_VALUE_BYTES_USIZE + 1;
const EXPECTED_GENERIC_COMPLETIONS: usize = 1;

#[test]
fn native_fixture_rejects_present_home_before_reading_callback_input() -> TestResult<()> {
    let mut homes = Vec::with_capacity(3);
    homes.push(std::ffi::OsString::new());
    homes.push(std::ffi::OsString::from("not-admitted"));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        homes.push(std::ffi::OsString::from_vec(vec![0xff]));
    }
    for home in homes {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_molten-native-extension-fixture"))
            .env_clear()
            .env("HOME", home)
            .arg("--previously-ignored")
            .stdin(std::process::Stdio::null())
            .output()
            .or_fail("execute native fixture with forbidden HOME")?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stderr, b"native extension fixture failed: inherited HOME is not admitted\n");
        assert!(output.stdout.is_empty(), "denied fixture must not emit a callback outcome");
    }
    Ok(())
}

// r[verify molten.system_extension.native_host.execution]
// r[verify molten.system_extension.native_host.intent]
// r[verify molten.system_extension.native_host.effects]
// r[verify molten.system_extension.native_host.effect_completion]
// r[verify molten.system_extension.native_host.effect_completion_value.accepted]
// r[verify molten.system_extension.native_host.operator]
// r[verify molten.system_extension.native_host.recovery]
// r[verify molten.system_extension.native_host.validation]
// r[verify molten.system_extension.native_host.nonclaims]
#[test]
fn separate_process_service_runs_lifecycle_effect_restart_and_removal() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start native service")?;
    assert_eq!(service.status().or_fail("running status")?.claim_level, "local-live-materialized-values-pilot",);
    let started_state_ref = service
        .instance()
        .or_fail("started native instance")?
        .state_ref
        .or_fail("materialized start state")?;
    let started_state = cohort
        .values
        .lock()
        .or_fail("native values")?
        .materialize(&started_state_ref, FULL_OUTPUT_BYTES)
        .or_fail("read materialized start state")?;
    assert!(!started_state.bytes.is_empty());

    let stale_observations = service.host().executor().observations().len();
    assert!(service.ingress(&ingress(STALE_GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK,).is_err());
    assert_eq!(service.host().executor().observations().len(), stale_observations);

    let accepted = {
        let mut client = Client::new(&mut service);
        client
            .submit(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK)
            .or_fail("accepted native ingress")?
    };
    let (callback_receipt, outcome) =
        accepted.dispatch.require_executed("accepted ingress").or_fail("accepted ingress callback")?;
    assert_eq!(callback_receipt.approved_effects.len(), 1);
    let request_state_ref = outcome.state_ref.or_fail("request state ref")?;
    assert_eq!(
        service.instance().or_fail("request instance")?.state_ref.as_deref(),
        Some(request_state_ref.as_str()),
    );
    cohort
        .values
        .lock()
        .or_fail("native values")?
        .materialize(&callback_receipt.approved_effects[0].request_ref, FULL_OUTPUT_BYTES)
        .or_fail("published effect request body")?;
    let mut effects = EffectPort::default();
    let completions = service.route_effects(&callback_receipt, &mut effects).or_fail("route exact native effect")?;
    assert_eq!(completions.len(), 1);
    let materialized = completions[0].materialized_output.as_ref().or_fail("materialized provider effect output")?;
    assert_eq!(materialized.bytes, EFFECT_OUTPUT_BYTES);
    assert_eq!(materialized.value_ref, completions[0].output_ref);
    assert_eq!(effects.routed, 1);
    service
        .deliver_effect_completion(&completions[0], EFFECT_COMPLETION_TICK)
        .or_fail("deliver effect completion")?
        .require_executed("effect completion")
        .or_fail("effect completion callback")?;

    service.checkpoint(CHECKPOINT_TICK).or_fail("checkpoint native service")?;
    let artifact_index = build_index(
        &service.instance().or_fail("indexed native instance")?,
        service.host().executor().observations(),
        std::slice::from_ref(&callback_receipt),
        &completions,
    )
    .or_fail("native artifact index")?;
    verify_index(&artifact_index).or_fail("verify native artifact index")?;
    assert!(artifact_index.members.iter().any(|member| member.role == Role::Effect));
    assert!(artifact_index.members.iter().any(|member| member.role == Role::SemanticState));
    assert!(artifact_index.members.iter().any(|member| member.role == Role::ValuePublication));
    let mut tampered_index = artifact_index.clone();
    tampered_index.members[0].parent_ref = HASH_F.to_string();
    assert!(verify_index(&tampered_index).is_err());

    assert_restart_recovers_and_removes(&cohort, service)
}

/// The checkpointed instance recovers from the durable journal, restarts, drains, stops, and is
/// removed, and the journal history records each callback intent before its publication.
fn assert_restart_recovers_and_removes(cohort: &Cohort, service: Service) -> TestResult<()> {
    let instance_id = service.instance().or_fail("native instance")?.instance_id;
    assert!(service.instance().or_fail("checkpointed instance")?.checkpoint_ref.is_some());
    drop(service);

    let restored = cohort
        .journal
        .lock()
        .or_fail("native journal")?
        .latest_instance(&instance_id)
        .or_fail("load native instance")?
        .or_fail("durable native instance")?;
    let mut recovered = cohort.recovered(restored)?;
    recovered.restart(RESTART_TICK).or_fail("restart and recover native service")?;
    assert!(recovered.status().or_fail("recovered status")?.recovery.is_empty());
    recovered.drain(DRAIN_TICK).or_fail("drain native service")?;
    recovered.stop(STOP_TICK).or_fail("stop native service")?;
    recovered.remove().or_fail("remove native service")?;
    assert_eq!(recovered.host().state().phase, LifecyclePhase::Removed);
    assert!(recovered.host().executor().observations().len() > 1);
    let history = cohort.journal.lock().or_fail("native journal")?.history(&instance_id).or_fail("native history")?;
    assert!(history.len() > 1);
    assert!(callback_intent_precedes_publication(&history));
    Ok(())
}

fn callback_intent_precedes_publication(history: &[NativeInstanceRecord]) -> bool {
    let callback_intent = history.iter().position(|record| {
        record.unresolved.iter().any(|operation| {
            operation.kind == NativeOperationKind::Callback && operation.state == NativeOperationState::IntentCommitted
        })
    });
    let publication_intent = history.iter().position(|record| {
        record.unresolved.iter().any(|operation| {
            operation.kind == NativeOperationKind::ValuePublication
                && operation.state == NativeOperationState::IntentCommitted
        })
    });
    matches!((callback_intent, publication_intent), (Some(callback), Some(publication)) if callback < publication)
}

// r[verify molten.system_extension.native_host.effect_completion_value.compatibility]
#[test]
fn generic_system_extension_completion_can_remain_reference_only() -> TestResult<()> {
    let run = run_executable(ExecutionProfile::InProcessNative)
        .or_fail("generic system extension fixture")?;
    assert_eq!(run.first_effect_completions.len(), EXPECTED_GENERIC_COMPLETIONS,);
    assert!(run.first_effect_completions.iter().all(|completion| completion.materialized_output.is_none()));
    Ok(())
}

// r[verify molten.system_extension.native_host.profile]
// r[verify molten.system_extension.native_host.ingress]
// r[verify molten.system_extension.native_host.neutrality]
#[test]
fn exact_profile_and_generation_fail_closed_without_hidden_fallback() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start native service")?;
    let failure = molten::fabric_execution::unavailable_port_failure(HASH_A);
    assert_eq!(failure.kind, molten::fabric_execution::PortFailureKind::ProfileUnavailable);
    assert!(failure.detail.contains("no fallback"));

    let mut wrong_transport = ingress(GENERATION, cohort.admitted.manifest_ref())?;
    wrong_transport.alpn = "unreviewed/fallback".to_string();
    assert!(service.ingress(&wrong_transport, REQUEST_TICK).is_err());
    assert!(service.ingress(&ingress(STALE_GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK,).is_err());
    Ok(())
}

// r[verify molten.system_extension.native_host.value_intent]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn uncertain_ingress_publication_blocks_callback_and_retry() -> TestResult<()> {
    let controlled = ControlledValuePort::default();
    let mut cohort = Cohort::new()?;
    cohort.values = shared_callback_value_port(controlled.clone());
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start controlled native service")?;
    let observations = service.host().executor().observations().len();
    controlled.fail_next(ValuePortFailureKind::UnknownAfterAcceptance)?;

    let failure = service
        .ingress(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK)
        .expect_err("uncertain ingress publication must fail");
    assert!(matches!(
        failure,
        Failure::Executor(CallbackFailure::Value(ref value_failure))
            if value_failure.may_have_published()
    ));
    assert_eq!(service.host().executor().observations().len(), observations);
    let instance = service.instance().or_fail("uncertain ingress instance")?;
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::ValuePublication && operation.state == NativeOperationState::Unknown
    }));
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::Ingress && operation.state == NativeOperationState::Unknown
    }));
    Ok(())
}

// r[verify molten.system_extension.native_host.value_publication]
// r[verify molten.system_extension.native_host.value_intent]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn uncertain_callback_publication_blocks_state_and_provider_effects() -> TestResult<()> {
    let controlled = ControlledValuePort::default();
    let mut cohort = Cohort::new()?;
    cohort.values = shared_callback_value_port(controlled.clone());
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start controlled native service")?;
    let prior_state_ref = service.instance().or_fail("prior native instance")?.state_ref;
    controlled.fail_after(1, ValuePortFailureKind::UnknownAfterAcceptance)?;

    let result = service
        .ingress(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK)
        .or_fail("ingress classification")?;
    assert!(matches!(result.dispatch, DispatchResult::Failed { .. }));
    let instance = service.instance().or_fail("failed callback instance")?;
    assert_eq!(instance.state_ref, prior_state_ref);
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::ValuePublication && operation.state == NativeOperationState::Unknown
    }));
    Ok(())
}

// r[verify molten.system_extension.native_host.value_materialization]
// r[verify molten.system_extension.native_host.semantic_state]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn restart_with_missing_state_bytes_fails_before_process_start() -> TestResult<()> {
    let mut cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start native service")?;
    service.checkpoint(CHECKPOINT_TICK).or_fail("checkpoint native service")?;
    let restored = service.instance().or_fail("checkpointed instance")?;
    let expected_state_ref = restored.state_ref.clone();
    drop(service);

    cohort.values = shared_callback_value_port(InMemoryCallbackValuePort::empty());
    let mut recovered = cohort.recovered(restored)?;
    assert!(recovered.restart(RESTART_TICK).is_err());
    let observations = recovered.host().executor().observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].lifecycle, molten::fabric_execution::ExecutionLifecycleState::FailedBeforeStart,);
    assert_eq!(recovered.instance().or_fail("failed recovery instance")?.state_ref, expected_state_ref);
    Ok(())
}

#[derive(Clone, Copy)]
enum InvalidEffectOutput {
    Missing,
    IdentityMismatch,
    Oversized,
}

struct InvalidEffectPort {
    kind: InvalidEffectOutput,
    routed: u64,
}
