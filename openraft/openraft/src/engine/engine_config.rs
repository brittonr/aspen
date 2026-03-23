use std::time::Duration;

use rand::Rng;

use crate::Config;
use crate::RaftTypeConfig;
use crate::engine::time_state;
use crate::type_config::alias::AsyncRuntimeOf;
use crate::type_config::async_runtime::AsyncRuntime;

/// Config for Engine
#[derive(Clone, Debug)]
#[derive(PartialEq, Eq)]
pub(crate) struct EngineConfig<C: RaftTypeConfig> {
    /// The id of this node.
    pub(crate) id: C::NodeId,

    /// The maximum number of applied logs to keep before purging.
    pub(crate) max_in_snapshot_log_to_keep: u64,

    /// The minimal number of applied logs to purge in a batch.
    pub(crate) purge_batch_size: u64,

    /// The maximum number of entries per payload allowed to be transmitted during replication
    pub(crate) max_payload_entries: u64,

    pub(crate) allow_log_reversion: bool,

    pub(crate) timer_config: time_state::Config,

    /// Election timeout bounds for re-randomization on each election attempt.
    /// Per Raft §5.2, a new random timeout must be chosen for each election
    /// to resolve split-votes quickly.
    pub(crate) election_timeout_min: u64,
    pub(crate) election_timeout_max: u64,
}

impl<C> EngineConfig<C>
where C: RaftTypeConfig
{
    pub(crate) fn new(id: C::NodeId, config: &Config) -> Self {
        let election_timeout = Duration::from_millis(config.new_rand_election_timeout::<AsyncRuntimeOf<C>>());
        Self {
            id,
            max_in_snapshot_log_to_keep: config.max_in_snapshot_log_to_keep,
            purge_batch_size: config.purge_batch_size,
            max_payload_entries: config.max_payload_entries,
            allow_log_reversion: config.get_allow_log_reversion(),

            timer_config: time_state::Config {
                election_timeout,
                smaller_log_timeout: Duration::from_millis(config.election_timeout_max * 2),
                leader_lease: Duration::from_millis(config.election_timeout_max),
            },
            election_timeout_min: config.election_timeout_min,
            election_timeout_max: config.election_timeout_max,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_default(id: C::NodeId) -> Self {
        Self {
            id,
            max_in_snapshot_log_to_keep: 1000,
            purge_batch_size: 256,
            max_payload_entries: 300,
            allow_log_reversion: false,
            timer_config: time_state::Config::default(),
            election_timeout_min: 150,
            election_timeout_max: 300,
        }
    }

    /// Re-randomize the election timeout within the configured bounds.
    /// Called on each election attempt per Raft §5.2 to prevent persistent
    /// split-votes when nodes have similar initial timeouts.
    pub(crate) fn rerandomize_election_timeout(&mut self) {
        let new_timeout = AsyncRuntimeOf::<C>::thread_rng()
            .random_range(self.election_timeout_min..self.election_timeout_max);
        self.timer_config.election_timeout = Duration::from_millis(new_timeout);
    }
}
