use std::process::Command;

use molten_core::addressable_actor::*;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use super::support::*;

const CHILD_MODE_ENV: &str = "MOLTEN_ADDRESSABLE_ACTOR_CHILD_MODE";
const CHILD_MODE_STALE: &str = "stale-generation";
const CHILD_MODE_CURRENT: &str = "current-generation";
const CHILD_TEST_NAME: &str = "addressable_actor::tests::multiprocess::child_actor_process";
const TEST_EXACT_ARGUMENT: &str = "--exact";
const TEST_NOCAPTURE_ARGUMENT: &str = "--nocapture";
const CHILD_SUCCESS: i32 = 0;
const WAKE_EFFECT_COUNT: usize = 2;

// r[verify molten.addressable_actor.verification]
#[test]
fn multiprocess_restart_recovers_state_and_fences_stale_generation() {
    let workspace =
        crate::test_support::process_workspace("addressable-actor-multiprocess").expect("actor process workspace");
    let workspace_path: &std::path::Path = workspace.as_ref();
    let directory = cap_std::fs::Dir::open_ambient_dir(workspace_path, cap_std::ambient_authority())
        .expect("open actor process workspace");
    let root = NodeStateRoot::from_dir(directory);
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let profile = profile();
    let actor_key = actor_key();
    let initial = initial_state(&profile, &actor_key);
    let actor_key_ref = initial.actor_key_ref.clone();
    let wake_request = request(&initial, "parent-wake", INITIAL_TICK, ActorOperation::Wake { reason: message_wake() });
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let current_generation = {
        let mut store = LocalActorStore::open(&storage, ENGINE_EPOCH).expect("actor store");
        let waking = apply_actor_request(&mut store, &mut effects, &mut statuses, &ActorServiceRequest {
            profile: &profile,
            actor_key: &actor_key,
            host_binding: &host_binding(&profile, &initial),
            expected: empty_expected(),
            request: &wake_request,
            requested_engine_epoch: ENGINE_EPOCH,
        })
        .expect("parent wake");
        let previous = waking.final_state;
        let mut replacement = previous.state.clone();
        replacement.extension_generation = replacement.extension_generation.saturating_add(1);
        replacement.lifecycle_sequence = replacement.lifecycle_sequence.saturating_add(1);
        replacement.revision = replacement.revision.saturating_add(1);
        replacement.phase = ActorPhase::Dormant;
        replacement.active_wake_ref = None;
        let replacement = PublishedActorState::from_state(replacement);
        let observation = store
            .compare_and_commit(&ActorCommitRequest {
                actor_key_ref: actor_key_ref.clone(),
                expected: expected(&previous),
                next: replacement.clone(),
                requested_engine_epoch: ENGINE_EPOCH,
            })
            .expect("replace actor generation");
        assert_eq!(observation.disposition, ActorCommitDisposition::Applied);
        replacement.state.extension_generation
    };
    drop(storage);

    run_child(workspace_path, CHILD_MODE_STALE);
    run_child(workspace_path, CHILD_MODE_CURRENT);

    let directory = cap_std::fs::Dir::open_ambient_dir(workspace_path, cap_std::ambient_authority())
        .expect("reopen actor process workspace");
    let root = NodeStateRoot::from_dir(directory);
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("reopened storage namespace");
    let store = LocalActorStore::open(&storage, ENGINE_EPOCH).expect("reopened actor store");
    let observed = store.load(&actor_key_ref).expect("read final actor state").expect("final actor state");
    assert_eq!(observed.state.extension_generation, current_generation);
    assert_eq!(observed.state.phase, ActorPhase::Starting);
}

#[test]
fn child_actor_process() {
    let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
        return;
    };
    let directory = cap_std::fs::Dir::open_ambient_dir(
        std::env::current_dir().expect("child current directory"),
        cap_std::ambient_authority(),
    )
    .expect("open child actor root");
    let root = NodeStateRoot::from_dir(directory);
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("child storage namespace");
    let profile = profile();
    let actor_key = actor_key();
    let actor_key_ref = identify_actor_key(&actor_key);
    let mut store = LocalActorStore::open(&storage, ENGINE_EPOCH).expect("child actor store");
    let published = store.load(&actor_key_ref).expect("child actor state read").expect("child actor state");
    let mut wake_request = request(
        &published.state,
        if mode == CHILD_MODE_STALE {
            "child-stale"
        } else {
            "child-current"
        },
        IDLE_TICK,
        ActorOperation::Wake { reason: message_wake() },
    );
    if mode == CHILD_MODE_STALE {
        wake_request.extension_generation = wake_request.extension_generation.saturating_sub(1);
    }
    let mut effects = MemoryEffectPort::succeeding(WAKE_EFFECT_COUNT);
    let mut statuses = MemoryStatusPort::default();
    let outcome = apply_actor_request(&mut store, &mut effects, &mut statuses, &ActorServiceRequest {
        profile: &profile,
        actor_key: &actor_key,
        host_binding: &host_binding(&profile, &published.state),
        expected: expected(&published),
        request: &wake_request,
        requested_engine_epoch: ENGINE_EPOCH,
    })
    .expect("child actor outcome");
    match mode.as_str() {
        CHILD_MODE_STALE => {
            assert_eq!(outcome.receipt.status, ActorServiceStatus::Denied);
            assert_eq!(outcome.transition.issue, Some(ActorIssue::StaleGeneration));
            assert_eq!(effects.execution_calls, 0);
        }
        CHILD_MODE_CURRENT => {
            assert_eq!(outcome.receipt.status, ActorServiceStatus::Applied);
            assert_eq!(outcome.final_state.state.phase, ActorPhase::Starting);
        }
        other => panic!("unsupported actor child mode {other}"),
    }
}

fn run_child(root: &std::path::Path, mode: &str) {
    let output = Command::new(std::env::current_exe().expect("current actor test executable"))
        .arg(TEST_EXACT_ARGUMENT)
        .arg(CHILD_TEST_NAME)
        .arg(TEST_NOCAPTURE_ARGUMENT)
        .env(CHILD_MODE_ENV, mode)
        .current_dir(root)
        .output()
        .expect("run actor child");
    assert_eq!(
        output.status.code(),
        Some(CHILD_SUCCESS),
        "actor child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
