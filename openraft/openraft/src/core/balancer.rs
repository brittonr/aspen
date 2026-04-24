//! This mod defines and manipulates the ratio of multiple messages to handle.
//!
//! The ratio should be adjust so that every channel won't be starved.

const INITIAL_RAFT_MESSAGE_BUDGET_DIVISOR: u64 = 10;
const NOTIFICATION_PRIORITY_NUMERATOR: u64 = 15;
const PRIORITY_ADJUSTMENT_DENOMINATOR: u64 = 16;
const RAFT_MESSAGE_PRIORITY_NUMERATOR: u64 = 17;
const MAX_RAFT_MESSAGE_BUDGET_DIVISOR: u64 = 2;
const MIN_RAFT_MESSAGE_BUDGET: u64 = 1;

/// Balance the ratio of different kind of message to handle.
pub(crate) struct Balancer {
    total: u64,

    /// The number of RaftMsg to handle in each round.
    raft_msg: u64,
}

impl Balancer {
    pub(crate) fn new(total: u64) -> Self {
        Self {
            total,
            // RaftMsg is the input entry.
            // We should consume as many as internal messages as possible.
            raft_msg: total / INITIAL_RAFT_MESSAGE_BUDGET_DIVISOR,
        }
    }

    pub(crate) fn raft_msg(&self) -> u64 {
        self.raft_msg
    }

    pub(crate) fn notification(&self) -> u64 {
        self.total.saturating_sub(self.raft_msg)
    }

    pub(crate) fn increase_notification(&mut self) {
        self.raft_msg = self
            .raft_msg
            .saturating_mul(NOTIFICATION_PRIORITY_NUMERATOR)
            / PRIORITY_ADJUSTMENT_DENOMINATOR;
        if self.raft_msg == 0 {
            self.raft_msg = MIN_RAFT_MESSAGE_BUDGET;
        }
    }

    pub(crate) fn increase_raft_msg(&mut self) {
        self.raft_msg = self
            .raft_msg
            .saturating_mul(RAFT_MESSAGE_PRIORITY_NUMERATOR)
            / PRIORITY_ADJUSTMENT_DENOMINATOR;

        let max_raft_message_budget = self.total / MAX_RAFT_MESSAGE_BUDGET_DIVISOR;

        // Always leave some budget for other channels
        if self.raft_msg > max_raft_message_budget {
            self.raft_msg = max_raft_message_budget;
        }
    }
}
