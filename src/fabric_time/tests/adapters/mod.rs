use preserves::ValueImpl;

use super::*;

const SINGLE_SLOT: u64 = 1;
const TWO_SLOTS: u64 = 2;
const RESUME_PRIORITY: i32 = 7;
const WAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

fn profile(kind: TimeProfileKind, overload: SchedulerOverloadPolicy, active_slots: u64, ready_slots: u64) -> Profile {
    let mut requested = descriptor(kind, "scheduler-adapter-regression", HASH_A);
    requested.max_runnables = active_slots;
    requested.max_scheduler_concurrency = SINGLE_SLOT;
    requested.max_scheduler_queue_depth = ready_slots;
    requested.scheduler_policy.overload = overload;
    admit_profile(&requested).expect("bounded scheduler profile")
}

fn key(id: &str) -> RunnableKey {
    RunnableKey {
        service_id: "test-service".to_string(),
        generation: GENERATION,
        runnable_id: id.to_string(),
    }
}

fn context(profile: &AdmittedTimeProfile, generation: u64) -> ExtensionTimeContext {
    ExtensionTimeContext::from_test_snapshot("test-service", generation, profile, Vec::new())
}

fn command(
    context: &ExtensionTimeContext,
    profile: &AdmittedTimeProfile,
    state: &SchedulerState,
    command: SchedulerCommand,
) -> SchedulerTransition {
    context
        .apply_scheduler_command(profile, profile.scheduler_policy, state, &command)
        .expect("scheduler transition")
}

fn event_action(event: &Event) -> String {
    const FIELD_ARITY: usize = 2;
    let fields = event.value.collect_simple_record("fabric-time-event-v1", None).expect("canonical event");
    fields
        .iter()
        .find_map(|value| {
            let field = value.collect_simple_record("field", Some(FIELD_ARITY))?;
            (field[0].as_string().as_deref() == Some("action"))
                .then(|| field[1].as_string().expect("string action").into_owned())
        })
        .expect("event action")
}

fn event(profile: &AdmittedTimeProfile, transition: &SchedulerTransition) -> Event {
    scheduler_transition(&profile.profile_ref, transition).expect("canonical scheduler event")
}

// Exercise the actual host wake, not only the route return value. Always release and join
// the target before asserting, so a missing wake fails within a bound without leaking a thread.
fn assert_live_wake(transition: &SchedulerTransition) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let target = std::thread::spawn(move || {
        std::thread::park();
        sender.send(()).expect("report host wake");
    });
    let mut adapter = ThreadSchedulerWakeAdapter::default();
    adapter
        .register(transition.runnable.clone(), target.thread().clone())
        .expect("register wake target");
    let routed = adapter.route(transition);
    let observed = receiver.recv_timeout(WAKE_TIMEOUT);
    target.thread().unpark();
    target.join().expect("wake target joined");
    assert!(adapter.unregister(&transition.runnable));
    routed.expect("route admitted wake");
    observed.expect("admitted transition must wake the host target");
}

fn assert_denied_wake(transition: &SchedulerTransition) {
    // Register the exact target, so rejection cannot be explained by a missing registration.
    let mut adapter = ThreadSchedulerWakeAdapter::default();
    adapter
        .register(transition.runnable.clone(), std::thread::current())
        .expect("register denied target");
    assert!(adapter.route(transition).is_err());
    assert!(adapter.unregister(&transition.runnable));
}

include!("capacity.rs");
include!("fencing.rs");
