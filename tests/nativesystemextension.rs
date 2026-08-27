#![feature(register_tool)]
#![register_tool(tigerstyle)]

#[path = "nativesystemextension/support.rs"]
mod support;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

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

// r[verify molten.system_extension.native_host.execution]
// r[verify molten.system_extension.native_host.intent]
// r[verify molten.system_extension.native_host.effects]
// r[verify molten.system_extension.native_host.effect_completion]
// r[verify molten.system_extension.native_host.operator]
// r[verify molten.system_extension.native_host.recovery]
// r[verify molten.system_extension.native_host.validation]
// r[verify molten.system_extension.native_host.nonclaims]
#[test]
fn separate_process_service_runs_lifecycle_effect_restart_and_removal() {
    let cohort = Cohort::new();
    let mut service = cohort.install();
    service.start(START_TICK).expect("start native service");
    assert_eq!(service.status().expect("running status").claim_level, "local-live-materialized-values-pilot",);
    let started_state_ref =
        service.instance().expect("started native instance").state_ref.expect("materialized start state");
    let started_state = cohort
        .values
        .lock()
        .expect("native values")
        .materialize(&started_state_ref, FULL_OUTPUT_BYTES)
        .expect("read materialized start state");
    assert!(!started_state.bytes.is_empty());

    let stale_observations = service.host().executor().observations().len();
    assert!(service.ingress(&ingress(STALE_GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK,).is_err());
    assert_eq!(service.host().executor().observations().len(), stale_observations);

    let accepted = {
        let mut client = NativeServiceClient::new(&mut service);
        client
            .submit(&ingress(GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK)
            .expect("accepted native ingress")
    };
    let (callback_receipt, outcome) =
        accepted.dispatch.require_executed("accepted ingress").expect("accepted ingress callback");
    assert_eq!(callback_receipt.approved_effects.len(), 1);
    let request_state_ref = outcome.state_ref.expect("request state ref");
    assert_eq!(service.instance().expect("request instance").state_ref.as_deref(), Some(request_state_ref.as_str()),);
    cohort
        .values
        .lock()
        .expect("native values")
        .materialize(&callback_receipt.approved_effects[0].request_ref, FULL_OUTPUT_BYTES)
        .expect("published effect request body");
    let mut effects = EffectPort::default();
    let completions = service.route_effects(&callback_receipt, &mut effects).expect("route exact native effect");
    assert_eq!(completions.len(), 1);
    assert_eq!(effects.routed, 1);
    service
        .deliver_effect_completion(&completions[0], EFFECT_COMPLETION_TICK)
        .expect("deliver effect completion")
        .require_executed("effect completion")
        .expect("effect completion callback");

    service.checkpoint(CHECKPOINT_TICK).expect("checkpoint native service");
    let artifact_index = build_native_artifact_index(
        &service.instance().expect("indexed native instance"),
        service.host().executor().observations(),
        std::slice::from_ref(&callback_receipt),
        &completions,
    )
    .expect("native artifact index");
    verify_native_artifact_index(&artifact_index).expect("verify native artifact index");
    assert!(artifact_index.members.iter().any(|member| member.role == NativeArtifactRole::Effect));
    assert!(artifact_index.members.iter().any(|member| member.role == NativeArtifactRole::SemanticState));
    assert!(artifact_index.members.iter().any(|member| member.role == NativeArtifactRole::ValuePublication));
    let mut tampered_index = artifact_index.clone();
    tampered_index.members[0].parent_ref = HASH_F.to_string();
    assert!(verify_native_artifact_index(&tampered_index).is_err());

    let instance_id = service.instance().expect("native instance").instance_id;
    assert!(service.instance().expect("checkpointed instance").checkpoint_ref.is_some());
    drop(service);

    let restored = cohort
        .journal
        .lock()
        .expect("native journal")
        .latest_instance(&instance_id)
        .expect("load native instance")
        .expect("durable native instance");
    let mut recovered = cohort.recovered(restored);
    recovered.restart(RESTART_TICK).expect("restart and recover native service");
    assert!(recovered.status().expect("recovered status").recovery.is_empty());
    recovered.drain(DRAIN_TICK).expect("drain native service");
    recovered.stop(STOP_TICK).expect("stop native service");
    recovered.remove().expect("remove native service");
    assert_eq!(recovered.host().state().phase, LifecyclePhase::Removed);
    assert!(recovered.host().executor().observations().len() > 1);
    let history = cohort.journal.lock().expect("native journal").history(&instance_id).expect("native history");
    assert!(history.len() > 1);
    assert!(callback_intent_precedes_publication(&history));
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

// r[verify molten.system_extension.native_host.profile]
// r[verify molten.system_extension.native_host.ingress]
// r[verify molten.system_extension.native_host.neutrality]
#[test]
fn exact_profile_and_generation_fail_closed_without_hidden_fallback() {
    let cohort = Cohort::new();
    let mut service = cohort.install();
    service.start(START_TICK).expect("start native service");
    let failure = molten::fabric_execution::unavailable_execution_port_failure(HASH_A);
    assert_eq!(failure.kind, molten::fabric_execution::ExecutionPortFailureKind::ProfileUnavailable);
    assert!(failure.detail.contains("no fallback"));

    let mut wrong_transport = ingress(GENERATION, cohort.admitted.manifest_ref());
    wrong_transport.alpn = "unreviewed/fallback".to_string();
    assert!(service.ingress(&wrong_transport, REQUEST_TICK).is_err());
    assert!(service.ingress(&ingress(STALE_GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK,).is_err());
}

// r[verify molten.system_extension.native_host.value_intent]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn uncertain_ingress_publication_blocks_callback_and_retry() {
    let controlled = ControlledValuePort::default();
    let mut cohort = Cohort::new();
    cohort.values = shared_native_callback_value_port(controlled.clone());
    let mut service = cohort.install();
    service.start(START_TICK).expect("start controlled native service");
    let observations = service.host().executor().observations().len();
    controlled.fail_next(NativeValuePortFailureKind::UnknownAfterAcceptance);

    let failure = service
        .ingress(&ingress(GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK)
        .expect_err("uncertain ingress publication must fail");
    assert!(matches!(
        failure,
        NativeServiceError::Executor(NativeExecutorError::Value(ref value_failure))
            if value_failure.may_have_published()
    ));
    assert_eq!(service.host().executor().observations().len(), observations);
    let instance = service.instance().expect("uncertain ingress instance");
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::ValuePublication && operation.state == NativeOperationState::Unknown
    }));
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::Ingress && operation.state == NativeOperationState::Unknown
    }));
}

// r[verify molten.system_extension.native_host.value_publication]
// r[verify molten.system_extension.native_host.value_intent]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn uncertain_callback_publication_blocks_state_and_provider_effects() {
    let controlled = ControlledValuePort::default();
    let mut cohort = Cohort::new();
    cohort.values = shared_native_callback_value_port(controlled.clone());
    let mut service = cohort.install();
    service.start(START_TICK).expect("start controlled native service");
    let prior_state_ref = service.instance().expect("prior native instance").state_ref;
    controlled.fail_after(1, NativeValuePortFailureKind::UnknownAfterAcceptance);

    let result = service
        .ingress(&ingress(GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK)
        .expect("ingress classification");
    assert!(matches!(result.dispatch, HostDispatchResult::Failed { .. }));
    let instance = service.instance().expect("failed callback instance");
    assert_eq!(instance.state_ref, prior_state_ref);
    assert!(instance.unresolved.iter().any(|operation| {
        operation.kind == NativeOperationKind::ValuePublication && operation.state == NativeOperationState::Unknown
    }));
}

// r[verify molten.system_extension.native_host.value_materialization]
// r[verify molten.system_extension.native_host.semantic_state]
// r[verify molten.system_extension.native_host.value_validation]
#[test]
fn restart_with_missing_state_bytes_fails_before_process_start() {
    let mut cohort = Cohort::new();
    let mut service = cohort.install();
    service.start(START_TICK).expect("start native service");
    service.checkpoint(CHECKPOINT_TICK).expect("checkpoint native service");
    let restored = service.instance().expect("checkpointed instance");
    let expected_state_ref = restored.state_ref.clone();
    drop(service);

    cohort.values = shared_native_callback_value_port(InMemoryNativeCallbackValuePort::default());
    let mut recovered = cohort.recovered(restored);
    assert!(recovered.restart(RESTART_TICK).is_err());
    let observations = recovered.host().executor().observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].lifecycle, molten::fabric_execution::ExecutionLifecycleState::FailedBeforeStart,);
    assert_eq!(recovered.instance().expect("failed recovery instance").state_ref, expected_state_ref);
}

#[derive(Clone, Default)]
struct ControlledValuePort {
    inner: Arc<Mutex<InMemoryNativeCallbackValuePort>>,
    failure: Arc<Mutex<Option<(usize, NativeValuePortFailureKind)>>>,
}

impl ControlledValuePort {
    fn fail_next(&self, kind: NativeValuePortFailureKind) {
        self.fail_after(0, kind);
    }

    fn fail_after(&self, successful_publications: usize, kind: NativeValuePortFailureKind) {
        *self.failure.lock().expect("controlled value failure") = Some((successful_publications, kind));
    }
}

impl NativeCallbackValuePort for ControlledValuePort {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> std::result::Result<NativeCallbackValue, NativeValuePortFailure> {
        self.inner
            .lock()
            .map_err(|_| {
                NativeValuePortFailure::new(
                    NativeValuePortFailureKind::RejectedBeforeAcceptance,
                    "controlled value port lock is unavailable",
                )
            })?
            .materialize(value_ref, maximum_bytes)
    }

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> std::result::Result<NativeValuePublicationReceipt, NativeValuePortFailure> {
        let failure = {
            let mut failure = self.failure.lock().map_err(|_| {
                NativeValuePortFailure::new(
                    NativeValuePortFailureKind::RejectedBeforeAcceptance,
                    "controlled value failure lock is unavailable",
                )
            })?;
            match *failure {
                Some((0, kind)) => {
                    *failure = None;
                    Some(kind)
                }
                Some((remaining, kind)) => {
                    *failure = Some((remaining - 1, kind));
                    None
                }
                None => None,
            }
        };
        let mut inner = self.inner.lock().map_err(|_| {
            NativeValuePortFailure::new(
                NativeValuePortFailureKind::RejectedBeforeAcceptance,
                "controlled value port lock is unavailable",
            )
        })?;
        if let Some(kind) = failure {
            inner.fail_next_publication(kind);
        }
        inner.publish(value, maximum_bytes)
    }
}

// r[verify molten.system_extension.native_host.execution]
// r[verify molten.system_extension.native_host.validation]
#[test]
fn native_executor_fails_closed_for_malformed_nonzero_timeout_flood_spawn_and_cancellation() {
    let shell = PathBuf::from("/bin/sh");
    for (script, timeout_ms, output_bytes) in [
        ("printf 'malformed'", NORMAL_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("exit 7", NORMAL_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("while :; do :; done", SHORT_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("printf 'output-flood'", NORMAL_TIMEOUT_MS, SMALL_OUTPUT_BYTES),
    ] {
        let mut cohort = Cohort::new();
        cohort.replace_program(shell.clone(), vec!["-c".to_string(), script.to_string()], timeout_ms, output_bytes);
        let mut service = cohort.install();
        assert!(service.start(START_TICK).is_err());
        assert!(!service.host().executor().observations().is_empty());
    }

    let mut missing = Cohort::new();
    missing.replace_program(
        PathBuf::from("/definitely/missing/native-extension"),
        Vec::new(),
        NORMAL_TIMEOUT_MS,
        FULL_OUTPUT_BYTES,
    );
    let mut missing_service = missing.install();
    assert!(missing_service.start(START_TICK).is_err());

    let cancellation = Cohort::new();
    let mut cancelled_service = cancellation.install();
    cancelled_service.host().executor().cancellation_handle().store(true, Ordering::Release);
    assert!(cancelled_service.start(START_TICK).is_err());
    assert_eq!(
        cancelled_service.host().executor().observations()[0].lifecycle,
        molten::fabric_execution::ExecutionLifecycleState::Cancelled
    );
}
