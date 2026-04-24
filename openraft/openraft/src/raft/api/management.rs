use std::collections::BTreeSet;
use std::fmt::Debug;

use maplit::btreemap;
use openraft_macros::since;
use tracing::Instrument;
use tracing::Level;
use tracing::Span;

use crate::ChangeMembers;
use crate::LogIdOptionExt;
use crate::OptionalSend;
use crate::RaftMetrics;
use crate::RaftTypeConfig;
use crate::core::raft_msg::RaftMsg;
use crate::core::replication_lag;
use crate::display_ext::DisplayResult;
use crate::display_ext::DisplayResultExt;
use crate::error::Fatal;
use crate::error::InitializeError;
use crate::impls::ProgressResponder;
use crate::membership::IntoNodes;
use crate::raft::ClientWriteResult;
use crate::raft::raft_inner::RaftInner;
use crate::type_config::TypeConfigExt;
use crate::type_config::alias::LogIdOf;
use crate::type_config::alias::OneshotReceiverOf;

/// Provides management APIs for the Raft system.
///
/// This struct contains methods for managing the Raft cluster, including
/// membership changes and node additions.
#[since(version = "0.10.0")]
pub(crate) struct ManagementApi<'a, C>
where C: RaftTypeConfig
{
    inner: &'a RaftInner<C>,
}

impl<'a, C> ManagementApi<'a, C>
where C: RaftTypeConfig
{
    pub(in crate::raft) fn new(inner: &'a RaftInner<C>) -> Self {
        Self { inner }
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn initialize<T>(&self, members: T) -> Result<Result<(), InitializeError<C>>, Fatal<C>>
    where T: IntoNodes<C::NodeId, C::Node> + Debug {
        async move {
            let (tx, rx) = C::oneshot();
            self.inner
                .call_core(
                    RaftMsg::Initialize {
                        members: members.into_nodes(),
                        tx,
                    },
                    rx,
                )
                .await
        }
        .instrument(tracing::span!(parent: &Span::current(), Level::DEBUG, "initialize"))
        .await
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn change_membership(
        &self,
        members: impl Into<ChangeMembers<C>>,
        should_retain: bool,
    ) -> Result<ClientWriteResult<C>, Fatal<C>> {
        async move {
            let changes: ChangeMembers<C> = members.into();

            tracing::info!(
                changes = debug(&changes),
                retain = display(should_retain),
                "change_membership: start to commit joint config"
            );

            let (tx, rx) = new_responder_pair::<C, _>();

            // res is error if membership cannot be changed.
            // If no error, it will enter a joint state
            let client_write_result = self
                .inner
                .call_core(
                    RaftMsg::ChangeMembership {
                        changes: changes.clone(),
                        retain: should_retain,
                        tx,
                    },
                    rx,
                )
                .await?;

            let resp = match client_write_result {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!("the first step error: {}", e);
                    return Ok(Err(e));
                }
            };

            tracing::debug!("res of first step: {}", resp);

            let joint_membership = match resp.membership.clone() {
                Some(membership) => membership,
                None => {
                    tracing::warn!(log_id = display(&resp.log_id), "ChangeMembership succeeded without membership; skip joint flatten step");
                    return Ok(Ok(resp));
                }
            };

            if joint_membership.get_joint_config().len() == 1 {
                return Ok(Ok(resp));
            }

            tracing::debug!("committed a joint config: {} {:?}", resp.log_id, joint_membership);
            tracing::debug!("the second step is to change to uniform config: {:?}", changes);

            let (tx, rx) = new_responder_pair::<C, _>();

            // The second step, send a NOOP change to flatten the joint config.
            let noop_changes = ChangeMembers::AddVoterIds(BTreeSet::new());
            let client_write_result = self
                .inner
                .call_core(
                    RaftMsg::ChangeMembership {
                        changes: noop_changes,
                        retain: should_retain,
                        tx,
                    },
                    rx,
                )
                .await?;

            tracing::info!("result of second step of change_membership: {}", client_write_result.display());

            if let Err(e) = &client_write_result {
                tracing::error!("the second step error: {}", e);
            }

            Ok(client_write_result)
        }
        .instrument(tracing::span!(parent: &Span::current(), Level::INFO, "change_membership"))
        .await
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn add_learner(
        &self,
        id: C::NodeId,
        node: C::Node,
        should_block_until_ready: bool,
    ) -> Result<ClientWriteResult<C>, Fatal<C>> {
        let span_target = id.clone();
        async move {
            let (tx, rx) = new_responder_pair::<C, _>();

            let msg = RaftMsg::ChangeMembership {
                changes: ChangeMembers::AddNodes(btreemap! {id.clone()=>node}),
                retain: true,
                tx,
            };

            let client_write_result = self.inner.call_core(msg, rx).await?;

            let resp = match client_write_result {
                Ok(x) => x,
                Err(e) => return Ok(Err(e)),
            };

            if !should_block_until_ready {
                return Ok(Ok(resp));
            }

            if self.inner.id == id {
                return Ok(Ok(resp));
            }

            // Otherwise, blocks until the replication to the new learner becomes up to date.

            // The log id of the membership that contains the added learner.
            let membership_log_id = &resp.log_id;

            let wait_res = self
                .inner
                .wait(None)
                .metrics(
                    |metrics| match self.check_replication_upto_date(metrics, &id, Some(membership_log_id)) {
                        Ok(_matching) => true,
                        // keep waiting
                        Err(_) => false,
                    },
                    "wait new learner to become line-rate",
                )
                .await;

            tracing::info!(wait_res = display(DisplayResult(&wait_res)), "waiting for replication to new learner");

            Ok(Ok(resp))
        }
        .instrument(
            tracing::span!(
                parent: &Span::current(),
                Level::DEBUG,
                "add_learner",
                target = display(&span_target)
            ),
        )
        .await
    }

    #[since(version = "0.10.0")]
    fn check_replication_upto_date(
        &self,
        metrics: &RaftMetrics<C>,
        node_id: &C::NodeId,
        membership_log_id: Option<&LogIdOf<C>>,
    ) -> Result<Option<LogIdOf<C>>, ()> {
        if metrics.membership_config.log_id().as_ref() < membership_log_id {
            // Waiting for the latest metrics to report.
            return Err(());
        }

        if metrics.membership_config.membership().get_node(node_id).is_none() {
            // This learner has been removed.
            return Ok(None);
        }

        let repl = match &metrics.replication {
            None => {
                // This node is no longer a leader.
                return Ok(None);
            }
            Some(x) => x,
        };

        let replication_metrics = repl;
        let target_metrics = match replication_metrics.get(node_id) {
            None => {
                // Maybe replication is not reported yet. Keep waiting.
                return Err(());
            }
            Some(x) => x,
        };

        let matched = target_metrics.clone();

        let distance = replication_lag(&matched.index(), &metrics.last_log_index);

        if distance <= self.inner.config.replication_lag_threshold {
            // replication became up to date.
            return Ok(matched);
        }

        // Not up to date, keep waiting.
        Err(())
    }
}

fn new_responder_pair<C, T>() -> (ProgressResponder<C, T>, OneshotReceiverOf<C, T>)
where
    C: RaftTypeConfig,
    T: OptionalSend,
{
    let (tx, _commit_rx, complete_rx) = ProgressResponder::new();

    (tx, complete_rx)
}
