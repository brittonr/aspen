//! The canonical identity port.
//!
//! A replica names its election timers, snapshots, recovery barriers, log entries, and recovery
//! corpora by content refs. The kernel decides which facts each identity covers; the port decides
//! how they are encoded and hashed. The `molten` shell hashes canonical packed Preserves records
//! with BLAKE3, so live replicas, durable stores, and peers agree on every ref.

/// The facts an election timer ref covers: one arm of one replica's timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElectionTimer<'a> {
    pub group_binding_ref: &'a str,
    pub node_id: &'a str,
    pub service_generation: u64,
    pub term: u64,
    pub sequence: u64,
}

/// The facts a recovery observation ref covers: the stable corpus a peer reports, independent of
/// retry and rejoin state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryCorpus<'a> {
    pub term: u64,
    pub commit_index: u64,
    pub log: &'a [crate::model::ReplicatedEntry],
    pub snapshot: Option<&'a crate::model::ReplicaSnapshot>,
}

pub trait ReplicaIdentity {
    fn election_timer_ref(&self, timer: &ElectionTimer<'_>) -> Result<String, crate::denial::IdentityError>;

    /// The snapshot's identity over every field except `snapshot_ref` itself.
    fn snapshot_ref(&self, snapshot: &crate::model::ReplicaSnapshot) -> Result<String, crate::denial::IdentityError>;

    /// The round ref of a recovery barrier opened at `term` from the active election timer.
    fn recovery_barrier_ref(&self, timer_ref: &str, term: u64) -> Result<String, crate::denial::IdentityError>;

    fn entry_ref(&self, entry: &crate::model::ReplicatedEntry) -> Result<String, crate::denial::IdentityError>;

    fn recovery_corpus_ref(&self, corpus: &RecoveryCorpus<'_>) -> Result<String, crate::denial::IdentityError>;
}
