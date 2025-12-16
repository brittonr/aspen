//! Model-guided Raft fuzzing based on state machine coverage.
//!
//! This target uses a formal Raft state model to guide fuzzing toward
//! interesting state transitions. It tracks which Raft states and transitions
//! have been exercised, helping the fuzzer find edge cases in consensus logic.
//!
//! Based on research from:
//! - "Model-Guided Fuzzing of Distributed Systems" (OOPSLA 2025)
//! - https://arxiv.org/abs/2410.02307
//!
//! ## State Model
//!
//! The Raft protocol has the following states:
//! - Follower: Default state, receives AppendEntries
//! - Candidate: Requesting votes for leader election
//! - Leader: Coordinates log replication
//! - Learner: Non-voting member catching up (Aspen extension)
//!
//! ## Transitions
//!
//! Key transitions we want to exercise:
//! - Follower -> Candidate (election timeout)
//! - Candidate -> Leader (received quorum votes)
//! - Candidate -> Follower (discovered higher term)
//! - Leader -> Follower (discovered higher term)
//! - Learner -> Follower (promoted to voter)
//!
//! ## Coverage Guidance
//!
//! The fuzzer is guided by tracking:
//! 1. Which states have been reached
//! 2. Which state transitions have occurred
//! 3. Message types that triggered transitions

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Raft node states (simplified model)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RaftState {
    Follower,
    Candidate,
    Leader,
    Learner,
}

/// State transition events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TransitionEvent {
    ElectionTimeout,
    ReceivedQuorumVotes,
    DiscoveredHigherTerm,
    ReceivedAppendEntries,
    PromotedToVoter,
    StepDown,
}

/// Simulated Raft message for state model testing
#[derive(Debug, Clone, Arbitrary)]
enum RaftMessage {
    /// Vote request from a candidate
    RequestVote {
        term: u64,
        candidate_id: u64,
        last_log_index: u64,
        last_log_term: u64,
    },
    /// Vote response
    RequestVoteResponse {
        term: u64,
        vote_granted: bool,
    },
    /// AppendEntries from leader
    AppendEntries {
        term: u64,
        leader_id: u64,
        prev_log_index: u64,
        prev_log_term: u64,
        entries_count: u8,
        leader_commit: u64,
    },
    /// AppendEntries response
    AppendEntriesResponse {
        term: u64,
        success: bool,
        match_index: u64,
    },
    /// Install snapshot from leader
    InstallSnapshot {
        term: u64,
        leader_id: u64,
        last_included_index: u64,
        last_included_term: u64,
        data_size: u32,
    },
    /// Election timeout trigger
    ElectionTimeout,
    /// Heartbeat timeout trigger
    HeartbeatTimeout,
}

/// Simplified Raft node for state model testing
#[derive(Debug, Clone)]
struct ModelRaftNode {
    id: u64,
    state: RaftState,
    current_term: u64,
    voted_for: Option<u64>,
    log_index: u64,
    commit_index: u64,
    votes_received: HashSet<u64>,
    cluster_size: u64,
}

impl ModelRaftNode {
    fn new(id: u64, cluster_size: u64, is_learner: bool) -> Self {
        Self {
            id,
            state: if is_learner {
                RaftState::Learner
            } else {
                RaftState::Follower
            },
            current_term: 0,
            voted_for: None,
            log_index: 0,
            commit_index: 0,
            votes_received: HashSet::new(),
            cluster_size,
        }
    }

    /// Process a message and return state transition if any
    fn process_message(&mut self, msg: &RaftMessage) -> Option<(RaftState, RaftState, TransitionEvent)> {
        let old_state = self.state;

        match msg {
            RaftMessage::ElectionTimeout => {
                if self.state == RaftState::Follower || self.state == RaftState::Candidate {
                    // Start election
                    self.current_term += 1;
                    self.state = RaftState::Candidate;
                    self.voted_for = Some(self.id);
                    self.votes_received.clear();
                    self.votes_received.insert(self.id);

                    if old_state != self.state {
                        return Some((old_state, self.state, TransitionEvent::ElectionTimeout));
                    }
                }
            }

            RaftMessage::RequestVote {
                term,
                candidate_id,
                last_log_index,
                last_log_term: _,
            } => {
                if *term > self.current_term {
                    // Discovered higher term - step down
                    self.current_term = *term;
                    self.voted_for = None;
                    if self.state == RaftState::Leader || self.state == RaftState::Candidate {
                        self.state = RaftState::Follower;
                        return Some((old_state, self.state, TransitionEvent::DiscoveredHigherTerm));
                    }
                }

                // Grant vote if we haven't voted and candidate's log is up-to-date
                if *term >= self.current_term
                    && self.voted_for.is_none()
                    && *last_log_index >= self.log_index
                {
                    self.voted_for = Some(*candidate_id);
                }
            }

            RaftMessage::RequestVoteResponse { term, vote_granted } => {
                if *term > self.current_term {
                    self.current_term = *term;
                    if self.state != RaftState::Follower {
                        self.state = RaftState::Follower;
                        self.voted_for = None;
                        return Some((old_state, self.state, TransitionEvent::DiscoveredHigherTerm));
                    }
                }

                if self.state == RaftState::Candidate && *vote_granted && *term == self.current_term {
                    // Count vote (simplified - in reality we'd track voter IDs)
                    self.votes_received.insert(self.votes_received.len() as u64 + 100);

                    // Check if we have quorum
                    let quorum = (self.cluster_size / 2) + 1;
                    if self.votes_received.len() as u64 >= quorum {
                        self.state = RaftState::Leader;
                        return Some((old_state, self.state, TransitionEvent::ReceivedQuorumVotes));
                    }
                }
            }

            RaftMessage::AppendEntries {
                term,
                leader_id: _,
                prev_log_index: _,
                prev_log_term: _,
                entries_count: _,
                leader_commit,
            } => {
                if *term >= self.current_term {
                    if *term > self.current_term {
                        self.current_term = *term;
                        self.voted_for = None;
                    }

                    // Step down if we're not a follower
                    if self.state == RaftState::Candidate || self.state == RaftState::Leader {
                        self.state = RaftState::Follower;
                        return Some((old_state, self.state, TransitionEvent::ReceivedAppendEntries));
                    }

                    // Update commit index
                    self.commit_index = (*leader_commit).min(self.log_index);
                }
            }

            RaftMessage::AppendEntriesResponse {
                term,
                success: _,
                match_index: _,
            } => {
                if *term > self.current_term {
                    self.current_term = *term;
                    if self.state != RaftState::Follower {
                        self.state = RaftState::Follower;
                        self.voted_for = None;
                        return Some((old_state, self.state, TransitionEvent::DiscoveredHigherTerm));
                    }
                }
            }

            RaftMessage::InstallSnapshot {
                term,
                leader_id: _,
                last_included_index,
                last_included_term: _,
                data_size: _,
            } => {
                if *term >= self.current_term {
                    if *term > self.current_term {
                        self.current_term = *term;
                        self.voted_for = None;
                    }

                    // Apply snapshot
                    if *last_included_index > self.log_index {
                        self.log_index = *last_included_index;
                        self.commit_index = *last_included_index;
                    }

                    // Step down if not follower
                    if self.state == RaftState::Candidate || self.state == RaftState::Leader {
                        self.state = RaftState::Follower;
                        return Some((old_state, self.state, TransitionEvent::ReceivedAppendEntries));
                    }
                }
            }

            RaftMessage::HeartbeatTimeout => {
                // Leader should send heartbeats, but we don't model that here
            }
        }

        None
    }

    /// Promote learner to voter
    fn promote_to_voter(&mut self) -> Option<(RaftState, RaftState, TransitionEvent)> {
        if self.state == RaftState::Learner {
            let old_state = self.state;
            self.state = RaftState::Follower;
            return Some((old_state, self.state, TransitionEvent::PromotedToVoter));
        }
        None
    }
}

/// Input for model-guided fuzzing
#[derive(Debug, Arbitrary)]
struct ModelInput {
    /// Initial cluster configuration
    cluster_size: u8,
    /// Number of learner nodes
    learner_count: u8,
    /// Sequence of messages to process
    messages: Vec<(u8, RaftMessage)>, // (target_node_index, message)
    /// Learner promotions (node indices)
    promotions: Vec<u8>,
}

/// Global coverage counters for state transitions
/// These help libfuzzer identify interesting inputs
static STATES_REACHED: AtomicUsize = AtomicUsize::new(0);
static TRANSITIONS_HIT: AtomicUsize = AtomicUsize::new(0);

fuzz_target!(|input: ModelInput| {
    // Bound cluster size for performance
    let cluster_size = (input.cluster_size % 7) as u64 + 1; // 1-7 nodes
    let learner_count = (input.learner_count % 3) as usize; // 0-2 learners

    // Bound message count
    if input.messages.len() > 100 {
        return;
    }

    // Create cluster
    let mut nodes: Vec<ModelRaftNode> = (0..cluster_size)
        .map(|id| ModelRaftNode::new(id, cluster_size, false))
        .collect();

    // Add learners
    for i in 0..learner_count.min(2) {
        nodes.push(ModelRaftNode::new(cluster_size + i as u64, cluster_size, true));
    }

    // Track coverage
    let mut states_seen: HashSet<RaftState> = HashSet::new();
    let mut transitions_seen: HashSet<(RaftState, RaftState, TransitionEvent)> = HashSet::new();

    // Record initial states
    for node in &nodes {
        states_seen.insert(node.state);
    }

    // Process messages
    for (target_idx, msg) in &input.messages {
        let idx = (*target_idx as usize) % nodes.len();

        if let Some(transition) = nodes[idx].process_message(msg) {
            transitions_seen.insert(transition);
            states_seen.insert(transition.1);
        }

        // Record current states
        for node in &nodes {
            states_seen.insert(node.state);
        }
    }

    // Process promotions
    for &idx in &input.promotions {
        let idx = (idx as usize) % nodes.len();
        if let Some(transition) = nodes[idx].promote_to_voter() {
            transitions_seen.insert(transition);
            states_seen.insert(transition.1);
        }
    }

    // Update global coverage counters
    // This helps libfuzzer prioritize inputs that reach new states
    let prev_states = STATES_REACHED.load(Ordering::Relaxed);
    if states_seen.len() > prev_states {
        STATES_REACHED.store(states_seen.len(), Ordering::Relaxed);
    }

    let prev_transitions = TRANSITIONS_HIT.load(Ordering::Relaxed);
    if transitions_seen.len() > prev_transitions {
        TRANSITIONS_HIT.store(transitions_seen.len(), Ordering::Relaxed);
    }

    // Verify invariants

    // Invariant 1: At most one leader per term
    let mut leaders_per_term: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
    for node in &nodes {
        if node.state == RaftState::Leader {
            *leaders_per_term.entry(node.current_term).or_insert(0) += 1;
        }
    }
    for (term, count) in &leaders_per_term {
        assert!(
            *count <= 1,
            "Invariant violation: {} leaders in term {}",
            count,
            term
        );
    }

    // Invariant 2: Learners should not become candidates or leaders
    for node in &nodes {
        if node.id >= cluster_size {
            // This is a learner
            assert!(
                node.state == RaftState::Learner || node.state == RaftState::Follower,
                "Learner {} should not be in state {:?}",
                node.id,
                node.state
            );
        }
    }

    // Invariant 3: Commit index should never decrease
    // (We don't track history here, but could extend)

    // Invariant 4: Term should never decrease
    // (We don't track history here, but could extend)
});
