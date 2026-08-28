use molten_core::addressable_actor::*;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use super::support::*;

const WAKE_EFFECT_COUNT: usize = 2;

// r[verify molten.addressable_actor.lifecycle]
// r[verify molten.addressable_actor.authority]
#[test]
fn service_commits_before_each_freshly_admitted_wake_effect() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let host = host_binding(&profile, &initial);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host,
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("wake actor");

    assert_eq!(outcome.receipt.status, ActorServiceStatus::Applied);
    assert_eq!(outcome.final_state.state.phase, ActorPhase::Starting);
    assert_eq!(effects.admission_calls, WAKE_EFFECT_COUNT);
    assert_eq!(effects.execution_calls, WAKE_EFFECT_COUNT);
    assert_eq!(outcome.effect_observations.len(), WAKE_EFFECT_COUNT);
    assert!(
        outcome
            .effect_observations
            .iter()
            .all(|observation| observation.disposition == ActorEffectDisposition::Succeeded)
    );
    assert_eq!(statuses.status_refs.len(), 1);
    assert!(!outcome.receipt.bytes.is_empty());
}

#[test]
fn stale_host_generation_denies_before_storage_or_effects() {
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut host = host_binding(&profile, &initial);
    host.system_extension_generation = host.system_extension_generation.saturating_add(1);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let error = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host,
        expected: empty_expected(),
        request: &request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect_err("stale host generation");

    assert_eq!(error, ActorServiceError::Host(ActorIssue::SystemExtensionGenerationMismatch));
    assert_eq!(commit.compare_calls, 0);
    assert_eq!(effects.execution_calls, 0);
}

// r[verify molten.addressable_actor.verification]
#[test]
fn capability_rooted_store_reopens_exact_actor_state() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary actor root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone actor root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let actor_key_ref = initial.actor_key_ref.clone();
    let request = request(&initial, "wake-message", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let expected_state_ref = {
        let mut store = LocalActorStore::open(&storage, ENGINE_EPOCH).expect("actor store");
        let outcome = apply_actor_request(&mut store, &mut effects, &mut statuses, &ActorServiceRequest {
            profile: &profile,
            actor_key: &actor_key,
            host_binding: &host_binding(&profile, &initial),
            expected: empty_expected(),
            request: &request,
            requested_engine_epoch: ENGINE_EPOCH,
        })
        .expect("persist actor wake");
        outcome.final_state.state_ref
    };
    let reopened = LocalActorStore::open(&storage, ENGINE_EPOCH).expect("reopened actor store");
    let observed = reopened.load(&actor_key_ref).expect("read actor state").expect("published actor state");
    assert_eq!(observed.state_ref, expected_state_ref);
    assert_eq!(observed.state.phase, ActorPhase::Starting);
}
