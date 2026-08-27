#![feature(register_tool)]
#![register_tool(tigerstyle)]

#[path = "nativesystemextension/support.rs"]
mod support;

use std::path::PathBuf;
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
    assert_eq!(service.status().expect("running status").claim_level, "local-live-pilot");

    let stale_observations = service.host().executor().observations().len();
    assert!(service.ingress(&ingress(STALE_GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK,).is_err());
    assert_eq!(service.host().executor().observations().len(), stale_observations);

    let accepted = {
        let mut client = NativeServiceClient::new(&mut service);
        client
            .submit(&ingress(GENERATION, cohort.admitted.manifest_ref()), REQUEST_TICK)
            .expect("accepted native ingress")
    };
    let (callback_receipt, _outcome) =
        accepted.dispatch.require_executed("accepted ingress").expect("accepted ingress callback");
    assert_eq!(callback_receipt.approved_effects.len(), 1);
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
    assert!(cohort.journal.lock().expect("native journal").history(&instance_id).expect("native history").len() > 1);
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
