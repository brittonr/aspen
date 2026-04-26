use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use anyerror::AnyError;
use openraft_macros::since;
use tracing::Instrument;
use tracing::Level;
use tracing::Span;
use validit::Valid;

use crate::EffectiveMembership;
use crate::LogIdOptionExt;
use crate::MembershipState;
use crate::RaftLogReader;
use crate::RaftSnapshotBuilder;
use crate::RaftState;
use crate::RaftTypeConfig;
use crate::StorageError;
use crate::StoredMembership;
use crate::display_ext::DisplayOptionExt;
use crate::engine::LogIdList;
use crate::entry::RaftEntry;
use crate::entry::RaftPayload;
use crate::error::StorageIOResult;
use crate::raft_state::IOState;
use crate::storage::RaftLogStorage;
use crate::storage::RaftStateMachine;
use crate::storage::log_reader_ext::RaftLogReaderExt;
use crate::type_config::TypeConfigExt;
use crate::type_config::alias::LeaderIdOf;
use crate::type_config::alias::LogIdOf;
use crate::type_config::alias::TermOf;
use crate::type_config::alias::VoteOf;
use crate::utime::Leased;
use crate::vote::RaftLeaderId;
use crate::vote::RaftVote;

const DEFAULT_NODE_ID_TEXT: &str = "xx";
const REAPPLY_CHUNK_ENTRY_COUNT: u64 = 64;
const MEMBERSHIP_SEARCH_STEP_COUNT: u64 = 64;
const MAX_MEMBERSHIP_RESULTS: usize = 2;

/// StorageHelper provides additional methods to access a [`RaftLogStorage`] and
/// [`RaftStateMachine`] implementation.
pub struct StorageHelper<'a, C, LS, SM>
where
    C: RaftTypeConfig,
    LS: RaftLogStorage<C>,
    SM: RaftStateMachine<C>,
{
    pub(crate) log_store: &'a mut LS,
    pub(crate) state_machine: &'a mut SM,

    id: Option<C::NodeId>,
    id_str: String,

    _p: PhantomData<C>,
}

impl<'a, C, LS, SM> StorageHelper<'a, C, LS, SM>
where
    C: RaftTypeConfig,
    LS: RaftLogStorage<C>,
    SM: RaftStateMachine<C>,
{
    /// Creates a new `StorageHelper` that provides additional functions based on the underlying
    ///  [`RaftLogStorage`] and [`RaftStateMachine`] implementation.
    pub fn new(sto: &'a mut LS, sm: &'a mut SM) -> Self {
        Self {
            log_store: sto,
            state_machine: sm,
            id: None,
            id_str: DEFAULT_NODE_ID_TEXT.to_string(),
            _p: PhantomData,
        }
    }

    /// Set the ID of this node
    #[since(version = "0.10.0")]
    pub fn with_id(mut self, id: C::NodeId) -> Self {
        self.id_str = id.to_string();
        self.id = Some(id);
        self
    }

    // TODO: let RaftStore store node-id.
    //       To achieve this, RaftLogStorage must store node-id
    //       To achieve this, RaftLogStorage has to provide API to initialize with a node id and API to
    //       read node-id
    /// Get Raft's state information from storage.
    ///
    /// When the Raft node is first started, it will call this interface to fetch the last known
    /// state from stable storage.
    pub async fn get_initial_state(&mut self) -> Result<RaftState<C>, StorageError<C>> {
        let mut log_reader = self.log_store.get_log_reader().await;
        let vote = log_reader.read_vote().await.sto_read_vote()?;
        // When absent, create a default value for this node.
        let vote = match vote {
            Some(vote) => vote,
            None => {
                let leader_id = LeaderIdOf::<C>::new(TermOf::<C>::default(), self.node_id()?);
                VoteOf::<C>::from_leader_id(leader_id, false)
            }
        };

        let mut committed = self.log_store.read_committed().await.sto_read_logs()?;

        let st = self.log_store.get_log_state().await.sto_read_logs()?;
        let mut last_purged_log_id = st.last_purged_log_id;
        let mut last_log_id = st.last_log_id;

        let (last_applied, _) = self.state_machine.applied_state().await.sto_read_sm()?;

        tracing::info!(
            vote = display(&vote),
            last_purged_log_id = display(last_purged_log_id.display()),
            last_applied = display(last_applied.display()),
            committed = display(committed.display()),
            last_log_id = display(last_log_id.display()),
            "get_initial_state"
        );

        // TODO: It is possible `committed < last_applied` because when installing snapshot,
        //       new committed should be saved, but not yet.
        if committed < last_applied {
            committed = last_applied.clone();
        }

        // For transient state machines: install persistent snapshot to restore state efficiently.
        self.restore_from_snapshot().await?;
        let (mut last_applied, _) = self.state_machine.applied_state().await.sto_read_sm()?;

        // Re-apply log entries to recover SM to latest state.
        // For transient state machines, this re-applies logs from snapshot position to committed.
        if last_applied < committed {
            let start = last_applied.next_index();
            let end = committed.next_index();

            // If required logs are purged, it's an error - we can't recover
            if start < last_purged_log_id.next_index() {
                let err = AnyError::error(format!(
                    "Cannot re-apply logs: need logs from index {}, but purged up to {}",
                    start,
                    last_purged_log_id.display()
                ));
                return Err(StorageError::read_log_at_index(start, err));
            }

            tracing::info!(start, end, "Re-applying committed logs to restore state machine to latest state");

            self.reapply_committed(start, end).await?;

            last_applied = committed.clone();
        }

        let mem_state = self.get_membership().await?;

        // Clean up dirty state: snapshot is installed, but logs are not cleaned.
        if last_log_id < last_applied {
            tracing::info!(
                "Clean the hole between last_log_id({}) and last_applied({}) by purging logs to {}",
                last_log_id.display(),
                last_applied.display(),
                last_applied.display(),
            );

            let last_applied_log_id = self.required_last_applied(&last_applied)?;
            self.log_store.purge(last_applied_log_id.clone()).await.sto_write_logs()?;
            last_log_id = Some(last_applied_log_id.clone());
            last_purged_log_id = Some(last_applied_log_id);
        }

        tracing::info!("load key log ids from ({},{}]", last_purged_log_id.display(), last_log_id.display());

        let log_id_list = self.get_key_log_ids(last_purged_log_id.clone(), last_log_id.clone()).await?;

        let snapshot = self.state_machine.get_current_snapshot().await.sto_read_snapshot(None)?;

        // If there is not a snapshot and there are logs purged, which means the snapshot is not persisted,
        // we just rebuild it so that replication can use it.
        let snapshot = match snapshot {
            None => {
                if last_purged_log_id.is_some() {
                    let mut builder = self.required_snapshot_builder().await?;
                    let snapshot = builder.build_snapshot().await.sto_write_snapshot(None)?;
                    Some(snapshot)
                } else {
                    None
                }
            }
            s @ Some(_) => s,
        };
        let snapshot_meta = match snapshot {
            Some(snapshot) => snapshot.meta,
            None => Default::default(),
        };

        let io_state = IOState::new(
            &self.id_str,
            &vote,
            last_applied.clone(),
            snapshot_meta.last_log_id.clone(),
            last_purged_log_id.clone(),
        );

        let now = C::now();

        Ok(RaftState {
            // The initial value for `vote` is the minimal possible value.
            // See: [Conditions for initialization][precondition]
            //
            // [precondition]: crate::docs::cluster_control::cluster_formation#preconditions-for-initialization
            //
            // TODO: If the lease reverted upon restart,
            //       the lease based linearizable read consistency will be broken.
            //       When lease based read is added, the restarted node must sleep for a while,
            //       before serving.
            vote: Leased::new(now, Duration::ZERO, vote),
            purged_next: last_purged_log_id.next_index(),
            log_ids: log_id_list,
            membership_state: mem_state,
            snapshot_meta,

            // -- volatile fields: they are not persisted.
            last_inflight_id: 0,
            server_state: Default::default(),
            io_state: Valid::new(io_state),
            purge_upto: last_purged_log_id,
        })
    }

    /// Restore state machine by installing snapshot if available and newer than last_applied.
    ///
    /// For transient state machines, this installs the last persistent snapshot to efficiently
    /// restore the state machine to a recent position.
    async fn restore_from_snapshot(&mut self) -> Result<(), StorageError<C>> {
        let (last_applied, _) = self.state_machine.applied_state().await.sto_read_sm()?;
        let snapshot = self.state_machine.get_current_snapshot().await.sto_read_snapshot(None)?;

        let Some(snap) = snapshot else {
            return Ok(());
        };

        if snap.meta.last_log_id > last_applied {
            tracing::info!(
                snapshot_last_log_id = display(snap.meta.last_log_id.display()),
                last_applied = display(last_applied.display()),
                "Installing snapshot to restore transient state machine"
            );

            self.state_machine
                .install_snapshot(&snap.meta, snap.snapshot)
                .await
                .sto_write_snapshot(Some(snap.meta.signature()))?;

            tracing::info!(
                new_last_applied = display(snap.meta.last_log_id.display()),
                "Snapshot installed, state machine restored to snapshot position"
            );
        }

        Ok(())
    }

    /// Read log entries from [`RaftLogReader`] in chunks and apply them to the state machine.
    pub(crate) async fn reapply_committed(&mut self, mut start: u64, end: u64) -> Result<(), StorageError<C>> {
        let chunk_entry_count = REAPPLY_CHUNK_ENTRY_COUNT;
        let max_chunk_count = ChunkCountBounds::new(start, end, chunk_entry_count).chunk_count();

        tracing::info!(
            "re-apply log [{}..{}) in {} item chunks to state machine",
            start,
            end,
            chunk_entry_count,
        );

        let mut log_reader = self.log_store.get_log_reader().await;

        for _chunk_index in 0..max_chunk_count {
            if start >= end {
                break;
            }

            let chunk_end = std::cmp::min(end, start.saturating_add(chunk_entry_count));
            let entries = log_reader.try_get_log_entries(start..chunk_end).await.sto_read_logs()?;

            let first = entries.first().map(|entry| entry.index());
            let last = entries.last().map(|entry| entry.index());
            let expected_last_index = chunk_end.saturating_sub(1);

            let make_err = || {
                let err = AnyError::error(format!(
                    "Failed to get log entries, expected index: [{}, {}), got [{:?}, {:?})",
                    start, chunk_end, first, last
                ));

                tracing::error!("{}", err);
                err
            };

            if first != Some(start) {
                return Err(StorageError::read_log_at_index(start, make_err()));
            }
            if last != Some(expected_last_index) {
                return Err(StorageError::read_log_at_index(expected_last_index, make_err()));
            }

            let replayed_entry_count = chunk_end.saturating_sub(start);
            tracing::info!("re-apply {} log entries: [{}, {}),", replayed_entry_count, start, chunk_end);
            let last_applied = match entries.last().map(|entry| entry.log_id()) {
                Some(log_id) => log_id,
                None => return Err(StorageError::read_log_at_index(start, make_err())),
            };
            let apply_items = entries.into_iter().map(|entry| Ok((entry, None)));
            let apply_stream = futures::stream::iter(apply_items);
            self.state_machine.apply(apply_stream).await.sto_apply(last_applied)?;

            start = chunk_end;
        }

        Ok(())
    }

    /// Returns the last two membership configs found in log or state machine.
    ///
    /// A raft node needs to store at most 2 membership config logs:
    /// - The first one must be committed, because raft allows proposing new membership only when
    ///   the previous one is committed.
    /// - The second may be committed or not.
    ///
    /// Because when handling append-entries RPC, (1) a raft follower will delete logs that are
    /// inconsistent with the leader,
    /// and (2) a membership will take effect at once it is written,
    /// a follower needs to revert the effective membership to a previous one.
    ///
    /// And because (3) there is at most one outstanding, uncommitted membership log,
    /// a follower only needs to revert at most one membership log.
    ///
    /// Thus, a raft node will only need to store at most two recent membership logs.
    pub async fn get_membership(&mut self) -> Result<MembershipState<C>, StorageError<C>> {
        let (last_applied, sm_mem) = self.state_machine.applied_state().await.sto_read_sm()?;

        let log_mem = self.last_membership_in_log(last_applied.next_index()).await?;
        tracing::debug!(membership_in_sm=?sm_mem, membership_in_log=?log_mem, "{}", func_name!());

        // There 2 membership configs in logs.
        if log_mem.len() == 2 {
            return Ok(MembershipState::new(
                Arc::new(EffectiveMembership::new_from_stored_membership(log_mem[0].clone())),
                Arc::new(EffectiveMembership::new_from_stored_membership(log_mem[1].clone())),
            ));
        }

        let effective = if log_mem.is_empty() {
            EffectiveMembership::new_from_stored_membership(sm_mem.clone())
        } else {
            EffectiveMembership::new_from_stored_membership(log_mem[0].clone())
        };

        let res = MembershipState::new(
            Arc::new(EffectiveMembership::new_from_stored_membership(sm_mem)),
            Arc::new(effective),
        );

        Ok(res)
    }

    /// Get the last 2 membership configs found in the log.
    ///
    /// This method returns at most membership logs with the greatest log index which is
    /// `>=since_index`. If no such membership log is found, it returns `None`, e.g., when logs
    /// are cleaned after being applied.
    pub async fn last_membership_in_log(
        &mut self,
        since_index: u64,
    ) -> Result<Vec<StoredMembership<C>>, StorageError<C>> {
        async move {
            let st = self.log_store.get_log_state().await.sto_read_logs()?;

            let mut end = st.last_log_id.next_index();

            tracing::info!("load membership from log: [{}..{})", since_index, end);

            let start = std::cmp::max(st.last_purged_log_id.next_index(), since_index);
            let step_count = MEMBERSHIP_SEARCH_STEP_COUNT;
            let max_step_count = ChunkCountBounds::new(start, end, step_count).chunk_count();

            let mut res = Vec::with_capacity(MAX_MEMBERSHIP_RESULTS);
            let mut log_reader = self.log_store.get_log_reader().await;

            for _step_index in 0..max_step_count {
                if start >= end {
                    break;
                }

                let step_start = std::cmp::max(start, end.saturating_sub(step_count));
                let entries = log_reader.try_get_log_entries(step_start..end).await.sto_read_logs()?;

                for ent in entries.iter().rev() {
                    if let Some(mem) = ent.get_membership() {
                        let em = StoredMembership::new(Some(ent.log_id()), mem);
                        res.insert(0, em);
                        if res.len() == MAX_MEMBERSHIP_RESULTS {
                            return Ok(res);
                        }
                    }
                }

                end = end.saturating_sub(step_count);
            }

            Ok(res)
        }
        .instrument(tracing::span!(parent: &Span::current(), Level::TRACE, "last_membership_in_log"))
        .await
    }

    // TODO: store purged: Option<LogId> separately.
    /// Get key-log-ids from the log store.
    ///
    /// Key-log-ids are the first log id of each Leader.
    async fn get_key_log_ids(
        &mut self,
        purged: Option<LogIdOf<C>>,
        last: Option<LogIdOf<C>>,
    ) -> Result<LogIdList<C>, StorageError<C>> {
        let mut log_reader = self.log_store.get_log_reader().await;

        let last = match last {
            None => return Ok(LogIdList::new(vec![])),
            Some(x) => x,
        };

        if purged.index() == Some(last.index()) {
            return Ok(LogIdList::new(vec![last]));
        }

        let first = log_reader.get_log_id(purged.next_index()).await?;

        let mut log_ids = log_reader.get_key_log_ids(first..=last).await.sto_read_logs()?;

        if !log_ids.is_empty()
            && let Some(purged) = purged
        {
            if purged.committed_leader_id() == log_ids[0].committed_leader_id() {
                if log_ids.len() >= 2 {
                    log_ids[0] = purged;
                } else {
                    log_ids.insert(0, purged);
                }
            } else {
                log_ids.insert(0, purged);
            }
        }

        Ok(LogIdList::new(log_ids))
    }

    fn node_id(&self) -> Result<C::NodeId, StorageError<C>> {
        match self.id.clone() {
            Some(node_id) => Ok(node_id),
            None => Err(StorageError::read_vote(AnyError::error(
                "StorageHelper::with_id() must be called before creating a default vote",
            ))),
        }
    }

    fn required_last_applied(&self, last_applied: &Option<LogIdOf<C>>) -> Result<LogIdOf<C>, StorageError<C>> {
        match last_applied.clone() {
            Some(log_id) => Ok(log_id),
            None => Err(StorageError::read(AnyError::error(
                "last_applied must exist before purging dirty logs",
            ))),
        }
    }

    async fn required_snapshot_builder(&mut self) -> Result<SM::SnapshotBuilder, StorageError<C>> {
        match self.state_machine.try_create_snapshot_builder(true).await {
            Some(builder) => Ok(builder),
            None => Err(StorageError::write_snapshot(
                None,
                AnyError::error("snapshot builder unavailable while rebuilding a missing snapshot"),
            )),
        }
    }
}

struct ChunkCountBounds {
    start_log_index: u64,
    end_log_index: u64,
    step_count: u64,
}

impl ChunkCountBounds {
    fn new(start_log_index: u64, end_log_index: u64, step_count: u64) -> Self {
        Self {
            start_log_index,
            end_log_index,
            step_count,
        }
    }

    fn chunk_count(self) -> usize {
        let span = self.end_log_index.saturating_sub(self.start_log_index);
        if self.step_count == 0 {
            return match usize::try_from(span) {
                Ok(chunk_count) => chunk_count,
                Err(_) => usize::MAX,
            };
        }

        let chunk_count_numerator = span.saturating_add(self.step_count.saturating_sub(1));
        let Some(chunk_count) = chunk_count_numerator.checked_div(self.step_count) else {
            return usize::MAX;
        };

        match usize::try_from(chunk_count) {
            Ok(chunk_count) => chunk_count,
            Err(_) => usize::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkCountBounds;

    const START_LOG_INDEX: u64 = 10;
    const END_LOG_INDEX_REQUIRING_TWO_CHUNKS: u64 = 75;
    const CHUNK_STEP_COUNT: u64 = 64;
    const EXPECTED_ROUNDED_CHUNKS: usize = 2;

    const ZERO_STEP_COUNT: u64 = 0;
    const END_LOG_INDEX_FOR_ZERO_STEP: u64 = 15;
    const EXPECTED_SPAN_CHUNKS: usize = 5;

    const REVERSED_START_LOG_INDEX: u64 = 75;
    const REVERSED_END_LOG_INDEX: u64 = 10;
    const EXPECTED_EMPTY_CHUNKS: usize = 0;

    #[test]
    fn chunk_count_rounds_up_partial_chunk() {
        let bounds = ChunkCountBounds::new(START_LOG_INDEX, END_LOG_INDEX_REQUIRING_TWO_CHUNKS, CHUNK_STEP_COUNT);

        assert_eq!(EXPECTED_ROUNDED_CHUNKS, bounds.chunk_count());
    }

    #[test]
    fn chunk_count_zero_step_returns_span_without_division() {
        let bounds = ChunkCountBounds::new(START_LOG_INDEX, END_LOG_INDEX_FOR_ZERO_STEP, ZERO_STEP_COUNT);

        assert_eq!(EXPECTED_SPAN_CHUNKS, bounds.chunk_count());
    }

    #[test]
    fn chunk_count_reversed_bounds_returns_zero() {
        let bounds = ChunkCountBounds::new(REVERSED_START_LOG_INDEX, REVERSED_END_LOG_INDEX, CHUNK_STEP_COUNT);

        assert_eq!(EXPECTED_EMPTY_CHUNKS, bounds.chunk_count());
    }
}
