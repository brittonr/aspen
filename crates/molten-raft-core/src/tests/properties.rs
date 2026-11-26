//! Bounded Hegel properties over drawn schedules of a three-voter group: timeouts, heartbeats,
//! proposals, reads, and peer messages delivered, reordered, duplicated, or dropped.

use super::*;

const MAX_STEPS: u64 = 48;
const MAX_IN_FLIGHT: usize = 32;
const NODES: [&str; 3] = [NODE_A, NODE_B, NODE_C];

/// One drawn scheduler decision.
#[derive(Debug, Clone, Copy)]
enum Step {
    ElectionTimeout(usize),
    Heartbeat(usize),
    Propose(usize),
    Read(usize),
    Deliver(usize),
    Duplicate(usize),
    Drop(usize),
}

fn draw_step(tc: &hegel::TestCase) -> Step {
    let kind = tc.draw(hegel::generators::integers::<u8>().min_value(0).max_value(6));
    let target = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(MAX_IN_FLIGHT - 1));
    let node = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(NODES.len() - 1));
    match kind {
        0 => Step::ElectionTimeout(node),
        1 => Step::Heartbeat(node),
        2 => Step::Propose(node),
        3 => Step::Read(node),
        4 => Step::Deliver(target),
        5 => Step::Duplicate(target),
        _ => Step::Drop(target),
    }
}

fn draw_schedule(tc: &hegel::TestCase) -> Vec<Step> {
    let length = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(MAX_STEPS));
    (0..length).map(|_| draw_step(tc)).collect()
}

/// The group, the envelopes in flight, and every step's outcome as the kernel returned it.
struct Group {
    nodes: Vec<ReplicaState>,
    in_flight: Vec<ReplicaMessageEnvelope>,
    trace: Vec<crate::denial::Result<ReplicaTransition>>,
    proposals: u64,
}

impl Group {
    fn started() -> Self {
        Self {
            nodes: NODES.iter().map(|node| started_state(node)).collect(),
            in_flight: Vec::new(),
            trace: Vec::new(),
            proposals: 0,
        }
    }

    fn run(schedule: &[Step]) -> Self {
        let mut group = Self::started();
        for step in schedule {
            group.step(*step);
        }
        group
    }

    fn step(&mut self, step: Step) {
        let (node, event) = match step {
            Step::ElectionTimeout(node) => (node, ReplicaEvent::ElectionTimeout {
                timer_ref: self.nodes[node].active_election_timer_ref.clone(),
            }),
            Step::Heartbeat(node) => (node, ReplicaEvent::HeartbeatTimeout),
            Step::Propose(node) => (node, self.proposal()),
            Step::Read(node) => (node, ReplicaEvent::Read {
                request_ref: test_ref(&format!("property-read-{}", self.trace.len())),
                mode: ReadMode::Linearizable,
            }),
            Step::Deliver(_) | Step::Duplicate(_) | Step::Drop(_) => {
                let Some(envelope) = self.take(step) else {
                    return;
                };
                let node = NODES.iter().position(|node| *node == envelope.to).expect("envelope to a voter");
                (node, ReplicaEvent::Message { envelope })
            }
        };
        let outcome = apply_replica_event(&self.nodes[node], event);
        if let Ok(transition) = &outcome {
            self.nodes[node] = transition.next.clone();
            self.send(transition);
        }
        self.trace.push(outcome);
    }

    fn proposal(&mut self) -> ReplicaEvent {
        self.proposals += 1;
        ReplicaEvent::Propose {
            request_ref: test_ref(&format!("property-request-{}", self.proposals)),
            command_ref: test_ref(&format!("property-command-{}", self.proposals)),
            command_schema_ref: test_ref("property-schema"),
        }
    }

    /// The envelope a delivery step selects; a drop step discards it and selects nothing.
    fn take(&mut self, step: Step) -> Option<ReplicaMessageEnvelope> {
        match step {
            Step::Deliver(target) => Some(self.in_flight.remove(target.checked_rem(self.in_flight.len())?)),
            Step::Duplicate(target) => self.in_flight.get(target.checked_rem(self.in_flight.len())?).cloned(),
            Step::Drop(target) => {
                self.in_flight.remove(target.checked_rem(self.in_flight.len())?);
                None
            }
            Step::ElectionTimeout(_) | Step::Heartbeat(_) | Step::Propose(_) | Step::Read(_) => None,
        }
    }

    fn send(&mut self, transition: &ReplicaTransition) {
        for effect in &transition.effects {
            if let ReplicaEffect::Send { envelope } = effect {
                if self.in_flight.len() == MAX_IN_FLIGHT {
                    self.in_flight.remove(0);
                }
                self.in_flight.push(envelope.clone());
            }
        }
    }
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[hegel::test(test_cases = 64)]
fn hegel_identical_schedules_yield_identical_states_and_effect_plans(tc: hegel::TestCase) {
    let schedule = draw_schedule(&tc);
    let first = Group::run(&schedule);
    let second = Group::run(&schedule);
    assert_eq!(first.trace, second.trace, "the kernel is a function of its inputs");
    assert_eq!(first.nodes, second.nodes);
    assert_eq!(first.in_flight, second.in_flight);
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
// r[verify molten.fabric_consistency.live_raft]
#[hegel::test(test_cases = 64)]
fn hegel_schedules_keep_one_leader_per_term_and_agree_on_committed_entries(tc: hegel::TestCase) {
    let schedule = draw_schedule(&tc);
    let mut group = Group::started();
    let mut leaders = std::collections::BTreeMap::<u64, String>::new();
    for step in schedule {
        group.step(step);
        for node in group.nodes.iter().filter(|node| node.role == ReplicaRole::Leader) {
            let leader = leaders.entry(node.current_term).or_insert_with(|| node.node_id.clone());
            assert_eq!(leader, &node.node_id, "two leaders in term {}", node.current_term);
        }
        assert_committed_prefixes_agree(&group.nodes);
    }
}

/// Every index both replicas have committed holds the same entry in both logs.
fn assert_committed_prefixes_agree(nodes: &[ReplicaState]) {
    let pairs = nodes
        .iter()
        .enumerate()
        .flat_map(|(position, left)| nodes[position + 1..].iter().map(move |right| (left, right)));
    for (left, right) in pairs {
        assert_committed_pair_agrees(left, right);
    }
}

fn assert_committed_pair_agrees(left: &ReplicaState, right: &ReplicaState) {
    let committed = left.commit_index.min(right.commit_index);
    let shared = left
        .log
        .iter()
        .filter(|entry| entry.index <= committed)
        .filter_map(|entry| right.log.iter().find(|other| other.index == entry.index).map(|other| (entry, other)));
    for (entry, other) in shared {
        assert_eq!(entry, other, "committed index {} diverges", entry.index);
    }
}
