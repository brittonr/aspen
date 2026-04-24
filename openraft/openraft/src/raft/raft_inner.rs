use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tracing::Instrument;
use tracing::Level;

use crate::Config;
use crate::OptionalSend;
use crate::RaftMetrics;
use crate::RaftTypeConfig;
use crate::async_runtime::MpscSender;
use crate::async_runtime::watch::WatchReceiver;
use crate::async_runtime::watch::WatchSender;
use crate::config::RuntimeConfig;
use crate::core::TickHandle;
use crate::core::io_flush_tracking::IoProgressWatcher;
use crate::core::raft_msg::RaftMsg;
use crate::core::raft_msg::external_command::ExternalCommand;
use crate::display_ext::DisplayOptionExt;
use crate::error::Fatal;
use crate::metrics::RaftDataMetrics;
use crate::metrics::RaftServerMetrics;
use crate::metrics::Wait;
use crate::raft::core_state::CoreState;
use crate::type_config::AsyncRuntime;
use crate::type_config::TypeConfigExt;
use crate::type_config::alias::AsyncRuntimeOf;
use crate::type_config::alias::MpscSenderOf;
use crate::type_config::alias::MutexOf;
use crate::type_config::alias::OneshotReceiverOf;
use crate::type_config::alias::OneshotSenderOf;
use crate::type_config::alias::WatchReceiverOf;

/// RaftInner is the internal handle and provides internally used APIs to communicate with
/// `RaftCore`.
pub(in crate::raft) struct RaftInner<C>
where C: RaftTypeConfig
{
    pub(in crate::raft) id: C::NodeId,
    pub(in crate::raft) config: Arc<Config>,
    pub(in crate::raft) runtime_config: Arc<RuntimeConfig>,
    pub(in crate::raft) tick_handle: TickHandle<C>,
    pub(in crate::raft) tx_api: MpscSenderOf<C, RaftMsg<C>>,
    pub(in crate::raft) rx_metrics: WatchReceiverOf<C, RaftMetrics<C>>,
    pub(in crate::raft) rx_data_metrics: WatchReceiverOf<C, RaftDataMetrics<C>>,
    pub(in crate::raft) rx_server_metrics: WatchReceiverOf<C, RaftServerMetrics<C>>,
    pub(in crate::raft) progress_watcher: IoProgressWatcher<C>,

    /// Lock ordering: if both std mutexes are ever held together, acquire `tx_shutdown`
    /// before `core_state`, and never hold either lock across `.await`.
    pub(in crate::raft) tx_shutdown: std::sync::Mutex<Option<OneshotSenderOf<C, ()>>>,
    pub(in crate::raft) core_state: std::sync::Mutex<CoreState<C>>,

    /// The ongoing snapshot transmission.
    #[cfg_attr(not(feature = "tokio-rt"), allow(dead_code))]
    // This field will only be read when feature tokio-rt is on
    pub(in crate::raft) snapshot: MutexOf<C, Option<crate::network::snapshot_transport::Streaming<C>>>,
}

impl<C> RaftInner<C>
where C: RaftTypeConfig
{
    pub(crate) fn id(&self) -> &C::NodeId {
        &self.id
    }

    pub(crate) fn config(&self) -> &Config {
        self.config.as_ref()
    }

    pub(crate) async fn send_msg(&self, mes: RaftMsg<C>) -> Result<(), Fatal<C>> {
        let send_res = self.tx_api.send(mes).await;

        if let Err(e) = send_res {
            let msg = e.0;

            let fatal = self.get_core_stop_error().await;
            tracing::error!("Failed to send RaftMsg: {msg} to RaftCore; error: {fatal}",);
            return Err(fatal);
        }
        Ok(())
    }

    /// Invoke RaftCore by sending a RaftMsg and blocks waiting for response.
    pub(crate) async fn call_core<T>(&self, mes: RaftMsg<C>, rx: OneshotReceiverOf<C, T>) -> Result<T, Fatal<C>>
    where T: OptionalSend {
        async {
            let message_summary = if tracing::enabled!(Level::DEBUG) {
                Some(mes.to_string())
            } else {
                None
            };

            self.send_msg(mes).await?;
            self.recv_msg(rx).await.inspect_err(|_e| {
                tracing::error!("Failed to receive from RaftCore: error when {}", message_summary.display());
            })
        }
        .instrument(raft_inner_span("call_core"))
        .await
    }

    /// Receive a message from RaftCore, return an error if RaftCore has stopped.
    pub(crate) async fn recv_msg<T, E>(&self, rx: impl Future<Output = Result<T, E>>) -> Result<T, Fatal<C>>
    where
        T: OptionalSend,
        E: OptionalSend,
    {
        let recv_res = rx.await;
        tracing::debug!("{} receives result is error: {:?}", func_name!(), recv_res.is_err());

        match recv_res {
            Ok(x) => Ok(x),
            Err(_) => {
                let fatal = self.get_core_stop_error().await;
                tracing::error!(error = debug(&fatal), "error when {}", func_name!());
                Err(fatal)
            }
        }
    }

    /// Send an [`ExternalCommand`] to RaftCore to execute in the `RaftCore` thread.
    ///
    /// It returns at once.
    pub(in crate::raft) async fn send_external_command(&self, cmd: ExternalCommand<C>) -> Result<(), Fatal<C>> {
        let send_res = self.tx_api.send(RaftMsg::ExternalCommand { cmd }).await;

        if send_res.is_err() {
            let fatal = self.get_core_stop_error().await;
            return Err(fatal);
        }
        Ok(())
    }

    pub(in crate::raft) fn is_core_running(&self) -> bool {
        let state = self.core_state_guard();
        state.is_running()
    }

    /// Get the error that caused RaftCore to stop.
    pub(crate) async fn get_core_stop_error(&self) -> Fatal<C> {
        // Wait for the core task to finish.
        self.join_core_task().await;

        // Retrieve the result.
        let core_res = {
            let state = self.core_state_guard();
            match &*state {
                CoreState::Done(core_task_res) => core_task_res.clone(),
                _other_state => {
                    debug_assert!(false, "RaftCore should have already quit before stop error lookup");
                    tracing::warn!("RaftCore stop requested before core entered Done state");
                    return Fatal::Stopped;
                }
            }
        };

        match core_res {
            Ok(_) => {
                debug_assert!(false, "RaftCore stop result must be an error");
                Fatal::Stopped
            }
            Err(fatal) => fatal,
        }
    }

    /// Wait for `RaftCore` task to finish and record the returned value from the task.
    pub(in crate::raft) async fn join_core_task(&self) {
        async {
            // Get the Running state of RaftCore,
            // or an error if RaftCore has been in Joining state.
            let running_res = {
                let mut state = self.core_state_guard();

                match &*state {
                    CoreState::Running(_) => {
                        let (tx, rx) = C::watch_channel::<bool>(false);

                        let prev = std::mem::replace(&mut *state, CoreState::Joining(rx));

                        match prev {
                            CoreState::Running(join_handle) => Ok((join_handle, tx)),
                            CoreState::Joining(watch_rx) => Err(watch_rx),
                            CoreState::Done(_) => {
                                debug_assert!(false, "core state changed to Done while starting join");
                                return;
                            }
                        }
                    }
                    CoreState::Joining(watch_rx) => Err(watch_rx.clone()),
                    CoreState::Done(_) => {
                        // RaftCore has already finished exiting, nothing to do
                        return;
                    }
                }
            };

            match running_res {
                Ok((join_handle, tx)) => {
                    let join_res = join_handle.await;

                    tracing::info!(res = debug(&join_res), "RaftCore exited");

                    let core_task_res = match join_res {
                        Err(err) => {
                            if AsyncRuntimeOf::<C>::is_panic(&err) {
                                Err(Fatal::Panicked)
                            } else {
                                Err(Fatal::Stopped)
                            }
                        }
                        Ok(returned_res) => returned_res,
                    };

                    {
                        let mut state = self.core_state_guard();
                        *state = CoreState::Done(core_task_res);
                    }
                    if tx.send(true).is_err() {
                        tracing::debug!("core join watcher dropped before shutdown notification");
                    }
                }
                Err(mut rx) => {
                    // Another thread is waiting for the core to finish.
                    while !*rx.borrow_watched() {
                        if rx.changed().await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
        .instrument(raft_inner_span("join_core_task"))
        .await
    }

    pub(crate) fn wait(&self, wait_timeout: Option<Duration>) -> Wait<C> {
        const SECONDS_PER_DAY: u64 = 24_u64 * 60_u64 * 60_u64;
        const DAYS_PER_YEAR: u64 = 365;
        const WAIT_TIMEOUT_YEARS: u64 = 100;
        const DEFAULT_WAIT_TIMEOUT_SECS: u64 =
            SECONDS_PER_DAY.saturating_mul(DAYS_PER_YEAR).saturating_mul(WAIT_TIMEOUT_YEARS);

        let wait_timeout = wait_timeout.unwrap_or_else(|| Duration::from_secs(DEFAULT_WAIT_TIMEOUT_SECS));

        Wait {
            timeout: wait_timeout,
            rx: self.rx_metrics.clone(),
        }
    }

    fn core_state_guard(&self) -> std::sync::MutexGuard<'_, CoreState<C>> {
        match self.core_state.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                tracing::warn!("core_state mutex poisoned; recovering inner state");
                poisoned.into_inner()
            }
        }
    }
}

fn raft_inner_span(operation: &'static str) -> tracing::Span {
    tracing::span!(Level::DEBUG, "raft_inner", operation = operation)
}
