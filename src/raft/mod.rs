//! Raft orchestration lives here.
//!
//! # Responsibilities
//! - Define the traits required to wire `openraft` state machines into the
//!   cluster actors exposed in [`crate::cluster`].
//! - Provide deterministic seams so we can embed Raft actors in `madsim`
//!   simulations and capture seeds/failure traces.
//! - Keep storage/network dependencies explicit so property tests can mock them.
//!
//! # Composition
//! `raft` exposes [`RaftActorFactory`] so callers can spin up deterministic
//! actors that will eventually host the actual `openraft::Raft` instance. Phase 2
//! ships a placeholder actor that already enforces Tiger Style invariants and
//! records the seams for attaching real storage and transport wiring.

use crate::cluster::NodeServerHandle;
use crate::storage::StorageSurface;
use anyhow::Context;
use async_trait::async_trait;
use openraft::Config;
use ractor::{Actor, ActorProcessingErr, ActorRef};

/// Node metadata required before spawning Raft actors.
#[derive(Debug, Clone)]
pub struct RaftNodeSpec {
    /// Numeric node ID used by OpenRaft.
    pub node_id: u64,
    /// Cluster namespace (e.g., `aspen::primary`).
    pub cluster: String,
}

impl RaftNodeSpec {
    /// Validate `node_id` is non-zero and the cluster label is present.
    pub fn validate(&self) {
        assert!(self.node_id > 0, "Raft node ids must be > 0");
        assert!(
            !self.cluster.is_empty(),
            "cluster labels guide deterministic harness routing"
        );
    }

    fn actor_name(&self) -> String {
        format!("raft:{}:{}", self.cluster, self.node_id)
    }
}

/// Trait implemented by Raft actor factories.
#[async_trait]
pub trait RaftActorFactory {
    /// Message type used by the actor; must stay remote-friendly.
    type Message: ractor::Message;

    /// Spawn a Raft actor wired to the provided storage + NodeServer.
    async fn spawn_actor(
        spec: RaftNodeSpec,
        storage: StorageSurface,
        node_handle: &NodeServerHandle,
        config: Config,
    ) -> anyhow::Result<ActorRef<Self::Message>>;
}

crate::cluster_message! {
    /// Control channel for the placeholder Raft actor.
    pub enum RaftActorMessage {
        /// Apply a deterministic payload to the state machine.
        Apply(Vec<u8>),
        /// Install a snapshot captured by tests.
        InstallSnapshot(Vec<u8>),
        /// Graceful shutdown so harnesses can drain actors.
        Shutdown,
    }
}

/// Tiger Style implementation that will be swapped with the OpenRaft actor.
pub struct LocalRaftFactory;

#[async_trait]
impl RaftActorFactory for LocalRaftFactory {
    type Message = RaftActorMessage;

    async fn spawn_actor(
        spec: RaftNodeSpec,
        storage: StorageSurface,
        node_handle: &NodeServerHandle,
        config: Config,
    ) -> anyhow::Result<ActorRef<Self::Message>> {
        spec.validate();
        let deterministic = node_handle
            .deterministic_config()
            .simulation_seed
            .map(|seed| format!("seed:{seed}"))
            .unwrap_or_else(|| "live".to_string());
        let args = RaftActorArgs {
            storage,
            config,
            deterministic_label: deterministic,
        };
        let (actor, _) = Actor::spawn(Some(spec.actor_name()), RaftCoordinator { spec }, args)
            .await
            .context("failed to spawn Raft coordinator")?;
        Ok(actor)
    }
}

struct RaftActorArgs {
    storage: StorageSurface,
    config: Config,
    deterministic_label: String,
}

struct RaftCoordinator {
    spec: RaftNodeSpec,
}

struct RaftCoordinatorState {
    storage: StorageSurface,
    config: Config,
    deterministic_label: String,
}

#[ractor::async_trait]
impl Actor for RaftCoordinator {
    type Msg = RaftActorMessage;
    type State = RaftCoordinatorState;
    type Arguments = RaftActorArgs;

    async fn pre_start(
        &self,
        _: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let RaftActorArgs {
            storage,
            config,
            deterministic_label,
        } = args;
        let validated_config = config.validate().context("raft config failed validation")?;
        Ok(RaftCoordinatorState {
            storage,
            config: validated_config,
            deterministic_label,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        debug_assert!(
            self.spec.node_id > 0,
            "node ids must stay positive for deterministic actor routing"
        );
        debug_assert!(
            !state.deterministic_label.is_empty(),
            "deterministic labels help seed property tests"
        );
        debug_assert!(
            state.config.max_payload_entries > 0,
            "max_payload_entries is validated at startup"
        );
        match message {
            RaftActorMessage::Apply(payload) => {
                let sm = state.storage.state_machine();
                sm.apply(&payload).map(|_| ()).map_err(|err| err.into())
            }
            RaftActorMessage::InstallSnapshot(bytes) => {
                let sm = state.storage.state_machine();
                sm.hydrate(&bytes).map_err(|err| err.into())
            }
            RaftActorMessage::Shutdown => {
                myself.get_cell().stop(Some("raft_shutdown".into()));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::DeterministicClusterConfig;
    use crate::cluster::NodeServerConfig;
    use ractor::ActorRef;

    #[tokio::test]
    async fn raft_node_spec_validates_inputs() {
        let spec = RaftNodeSpec {
            node_id: 1,
            cluster: "aspen::primary".into(),
        };
        spec.validate();
    }

    #[tokio::test]
    async fn spawn_placeholder_actor_and_apply() {
        let config = Config::default();
        let surface = StorageSurface::in_memory();
        let node_server = NodeServerConfig::new("node-a", "localhost", 0, "supersafe")
            .with_determinism(DeterministicClusterConfig {
                simulation_seed: Some(7),
            })
            .launch()
            .await
            .expect("node server");
        let spec = RaftNodeSpec {
            node_id: 1,
            cluster: "aspen::primary".into(),
        };
        let actor: ActorRef<RaftActorMessage> =
            LocalRaftFactory::spawn_actor(spec, surface.clone(), &node_server, config)
                .await
                .expect("spawn actor");
        actor
            .cast(RaftActorMessage::Apply(b"hello".to_vec()))
            .unwrap();
        actor.cast(RaftActorMessage::Shutdown).unwrap();
        node_server.shutdown().await.unwrap();
    }
}
