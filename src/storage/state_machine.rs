//! State machine adapters that run alongside the log backend.
//!
//! The state-machine interface owns two responsibilities:
//! - Apply deterministic byte payloads in order, returning the monotonic apply
//!   clock so Raft can compare progress across nodes.
//! - Produce snapshots for failover/testing and hydrate state back when needed.

use std::sync::Mutex;

/// State-machine trait implemented by persistent/deterministic engines.
pub trait StateMachineHandle: Send + Sync + 'static {
    /// Apply bytes, returning the logical clock.
    fn apply(&self, bytes: &[u8]) -> anyhow::Result<u64>;
    /// Highest applied clock.
    fn last_applied(&self) -> u64;
    /// Serialize current state for snapshots.
    fn snapshot(&self) -> anyhow::Result<Vec<u8>>;
    /// Restore state from a snapshot.
    fn hydrate(&self, bytes: &[u8]) -> anyhow::Result<()>;
}

/// In-memory handle that keeps a deterministic log of mutations.
#[derive(Debug, Default)]
pub struct InMemoryStateMachine {
    applied: Mutex<Vec<Vec<u8>>>,
}

impl StateMachineHandle for InMemoryStateMachine {
    fn apply(&self, bytes: &[u8]) -> anyhow::Result<u64> {
        let mut applied = self.applied.lock().unwrap();
        applied.push(bytes.to_vec());
        Ok(applied.len() as u64)
    }

    fn last_applied(&self) -> u64 {
        self.applied.lock().unwrap().len() as u64
    }

    fn snapshot(&self) -> anyhow::Result<Vec<u8>> {
        let applied = self.applied.lock().unwrap();
        Ok(applied.concat())
    }

    fn hydrate(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut applied = self.applied.lock().unwrap();
        applied.clear();
        applied.push(bytes.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_updates_clock() {
        let sm = InMemoryStateMachine::default();
        assert_eq!(sm.apply(b"alpha").unwrap(), 1);
        assert_eq!(sm.apply(b"beta").unwrap(), 2);
        assert_eq!(sm.last_applied(), 2);
    }

    #[test]
    fn snapshot_and_hydrate_restores_state() {
        let sm = InMemoryStateMachine::default();
        sm.apply(b"alpha").unwrap();
        sm.apply(b"beta").unwrap();
        let snapshot = sm.snapshot().unwrap();
        let sm2 = InMemoryStateMachine::default();
        sm2.hydrate(&snapshot).unwrap();
        assert_eq!(sm2.last_applied(), 1);
    }
}
