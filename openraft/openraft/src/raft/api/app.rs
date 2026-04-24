use openraft_macros::since;
use tracing::Instrument;

use crate::RaftTypeConfig;
use crate::ReadPolicy;
use crate::core::raft_msg::RaftMsg;
use crate::error::ClientWriteError;
use crate::error::Fatal;
use crate::error::LinearizableReadError;
use crate::impls::ProgressResponder;
use crate::raft::ClientWriteResponse;
use crate::raft::ClientWriteResult;
use crate::raft::linearizable_read::Linearizer;
use crate::raft::raft_inner::RaftInner;
use crate::raft::responder::core_responder::CoreResponder;
use crate::type_config::TypeConfigExt;
use crate::type_config::alias::WriteResponderOf;

/// Provides application-facing APIs for interacting with the Raft system.
///
/// This struct contains methods for client operations such as linearizable reads
/// and writes.
#[since(version = "0.10.0")]
pub(crate) struct AppApi<'a, C>
where C: RaftTypeConfig
{
    inner: &'a RaftInner<C>,
}

impl<'a, C> AppApi<'a, C>
where C: RaftTypeConfig
{
    pub(in crate::raft) fn new(inner: &'a RaftInner<C>) -> Self {
        Self { inner }
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn get_read_linearizer(
        &self,
        read_policy: ReadPolicy,
    ) -> Result<Result<Linearizer<C>, LinearizableReadError<C>>, Fatal<C>> {
        async move {
            let (tx, rx) = C::oneshot();
            self.inner.call_core(RaftMsg::EnsureLinearizableRead { read_policy, tx }, rx).await
        }
        .instrument(tracing::debug_span!("get_read_linearizer"))
        .await
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn client_write(
        &self,
        app_data: C::D,
        // TODO: ClientWriteError can only be ForwardToLeader Error
    ) -> Result<Result<ClientWriteResponse<C>, ClientWriteError<C>>, Fatal<C>> {
        async move {
            let (responder, _commit_rx, complete_rx) = ProgressResponder::new();

            self.do_client_write_ff(app_data, Some(CoreResponder::Progress(responder))).await?;

            let res: ClientWriteResult<C> = self.inner.recv_msg(complete_rx).await?;

            Ok(res)
        }
        .instrument(tracing::debug_span!("client_write"))
        .await
    }

    #[since(version = "0.10.0")]
    pub(crate) async fn client_write_ff(
        &self,
        app_data: C::D,
        responder: Option<WriteResponderOf<C>>,
    ) -> Result<(), Fatal<C>> {
        async move {
            self.do_client_write_ff(app_data, responder.map(|r| CoreResponder::UserDefined(r))).await
        }
        .instrument(tracing::debug_span!("client_write_ff"))
        .await
    }

    /// Fire-and-forget version of `client_write`, accept a generic responder.
    #[since(version = "0.10.0")]
    async fn do_client_write_ff(&self, app_data: C::D, responder: Option<CoreResponder<C>>) -> Result<(), Fatal<C>> {
        self.inner
            .send_msg(RaftMsg::ClientWriteRequest {
                app_data,
                responder,
                expected_leader: None,
            })
            .await?;

        Ok(())
    }
}
