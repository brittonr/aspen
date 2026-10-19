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

fn event_action(event: &Event) -> String {
    let fields = event.value.collect_simple_record("fabric-time-event-v1", None).expect("canonical event");
    fields
        .iter()
        .find_map(|value| {
            let field = value.collect_simple_record("field", Some(2))?;
            (field[0].as_string().as_deref() == Some("action"))
                .then(|| field[1].as_string().expect("action").into_owned())
        })
        .expect("event action")
}

type Snapshot = (SchedulerState, molten_core::fabric_time::capacity::UseState);
#[derive(Debug)]
struct DeliveryBoundary {
    snapshot: Snapshot,
    occurrence: usize,
    sequence: u64,
}

enum ConcreteEffects {
    Live(ThreadSchedulerWakeAdapter),
    Simulation(SimulationSchedulerWakeAdapter),
}
struct Effects {
    concrete: ConcreteEffects,
    live_receiver: Option<std::sync::mpsc::SyncSender<ReceiverInput>>,
    simulation_boundaries: std::collections::VecDeque<DeliveryBoundary>,
    preparations: u64,
    deliveries: u64,
}
enum Prepared {
    Live(std::thread::Thread),
    Simulation(u64),
}
impl SchedulerWakePort for Effects {
    type Prepared = Prepared;
    fn prepare(&mut self, key: &RunnableKey, index: usize) -> crate::fabric::FabricPortResult<Prepared> {
        self.preparations += 1;
        match &mut self.concrete {
            ConcreteEffects::Live(port) => port.prepare(key, index).map(Prepared::Live),
            ConcreteEffects::Simulation(port) => port.prepare(key, index).map(Prepared::Simulation),
        }
    }
    fn deliver(
        &mut self,
        prepared: Prepared,
        committed: SchedulerWakeCommit<'_>,
    ) -> crate::fabric::FabricPortResult<()> {
        self.deliveries += 1;
        // Capture the actual borrowed owner fields AT delivery, before entering the
        // concrete adapter. Never read state later behind the command mutex. This
        // observation travels with that real delivery, not a predicted test result.
        let boundary = DeliveryBoundary {
            snapshot: (committed.scheduler.clone(), committed.capacity.clone()),
            occurrence: committed.occurrence,
            sequence: self.deliveries,
        };
        match (&mut self.concrete, prepared) {
            (ConcreteEffects::Live(port), Prepared::Live(target)) => {
                self.live_receiver
                    .as_ref()
                    .expect("live receiving mailbox")
                    .send(ReceiverInput::Delivery(boundary))
                    .expect("record real delivery boundary");
                port.deliver(target, committed)
            }
            (ConcreteEffects::Simulation(port), Prepared::Simulation(index)) => {
                self.simulation_boundaries.push_back(boundary);
                port.deliver(index, committed)
            }
            _ => unreachable!("prepared transport matches owning port"),
        }
    }
}

fn boundary_is_committed(boundary: &DeliveryBoundary, key: &RunnableKey) -> bool {
    let (state, capacity) = &boundary.snapshot;
    let Some(row) = state.runnables.get(boundary.occurrence) else {
        return false;
    };
    let active = state
        .runnables
        .iter()
        .filter(|row| matches!(row.phase, RunnablePhase::Ready | RunnablePhase::Running | RunnablePhase::Blocked))
        .count();
    let ready = state.runnables.iter().filter(|row| row.phase == RunnablePhase::Ready).count();
    &row.key == key
        && row.phase == RunnablePhase::Ready
        && !capacity.is_released
        && capacity.profile_ref == state.profile_ref
        && capacity.generation == state.generation
        && u64::try_from(active).ok() == Some(capacity.runnable_usage)
        && u64::try_from(ready).ok() == Some(capacity.queue_usage)
}

include!("receiver.rs");
include!("harness.rs");

include!("capacity.rs");
include!("fencing.rs");
