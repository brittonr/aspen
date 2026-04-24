//! tick emitter emits a `RaftMsg::Tick` event at a certain interval.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use futures::future::Either;
use tracing::Instrument;
use tracing::Level;
use tracing::Span;

use crate::RaftTypeConfig;
use crate::core::notification::Notification;
use crate::type_config::TypeConfigExt;
use crate::type_config::alias::JoinHandleOf;
use crate::type_config::alias::MpscSenderOf;
use crate::type_config::alias::OneshotReceiverOf;
use crate::type_config::alias::OneshotSenderOf;
use crate::type_config::async_runtime::mpsc::MpscSender;
use crate::type_config::async_runtime::oneshot::OneshotSender;

/// Emit RaftMsg::Tick event at regular `interval`.
pub(crate) struct Tick<C>
where C: RaftTypeConfig
{
    interval: Duration,

    tx: MpscSenderOf<C, Notification<C>>,

    /// Emit event or not
    enabled: Arc<AtomicBool>,
}

pub(crate) struct TickHandle<C>
where C: RaftTypeConfig
{
    enabled: Arc<AtomicBool>,
    /// Lock ordering: acquire `shutdown` before `join_handle` when both are needed.
    shutdown: Mutex<Option<OneshotSenderOf<C, ()>>>,
    /// Lock ordering: acquire after `shutdown`.
    join_handle: Mutex<Option<JoinHandleOf<C, ()>>>,
}

impl<C> Drop for TickHandle<C>
where C: RaftTypeConfig
{
    /// Signal the tick loop to stop, without waiting for it to stop.
    fn drop(&mut self) {
        let has_shutdown_sender = {
            let shutdown_guard = lock_option_mutex(&self.shutdown, "tick.shutdown");
            shutdown_guard.is_some()
        };
        if !has_shutdown_sender {
            return;
        }
        let _ = self.shutdown();
    }
}

fn lock_option_mutex<'a, T>(mutex: &'a Mutex<Option<T>>, lock_name: &'static str) -> MutexGuard<'a, Option<T>> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(lock_name, "Recovering poisoned mutex");
            poisoned.into_inner()
        }
    }
}

impl<C> Tick<C>
where C: RaftTypeConfig
{
    pub(crate) fn spawn(interval: Duration, tx: MpscSenderOf<C, Notification<C>>, enabled: bool) -> TickHandle<C> {
        let enabled = Arc::new(AtomicBool::from(enabled));
        let this = Self {
            interval,
            enabled: enabled.clone(),
            tx,
        };

        let (shutdown, shutdown_rx) = C::oneshot();

        let shutdown = Mutex::new(Some(shutdown));

        let join_handle = C::spawn(this.tick_loop(shutdown_rx).instrument(tracing::span!(
            parent: &Span::current(),
            Level::DEBUG,
            "tick"
        )));

        TickHandle {
            enabled,
            shutdown,
            join_handle: Mutex::new(Some(join_handle)),
        }
    }

    pub(crate) async fn tick_loop(self, cancel_rx: OneshotReceiverOf<C, ()>) {
        let mut tick_index: u64 = 0;
        let mut should_stop = false;
        let mut cancel = std::pin::pin!(cancel_rx);

        while !should_stop {
            let next_tick_at = C::now() + self.interval;
            let sleep_fut = std::pin::pin!(C::sleep_until(next_tick_at));
            let cancel_fut = cancel.as_mut();

            should_stop = match futures::future::select(cancel_fut, sleep_fut).await {
                Either::Left((_canceled, _)) => {
                    tracing::info!("TickLoop received cancel signal, quit");
                    true
                }
                Either::Right((_, _)) => false,
            };
            if should_stop {
                continue;
            }

            if !self.enabled.load(Ordering::Relaxed) {
                continue;
            }

            tick_index = tick_index.saturating_add(1);

            let send_res = self.tx.send(Notification::Tick { i: tick_index }).await;
            if let Err(_e) = send_res {
                tracing::info!("Stopping tick_loop(), main loop terminated");
                should_stop = true;
            } else {
                tracing::debug!("Tick sent: {}", tick_index)
            }
        }
    }
}

impl<C> TickHandle<C>
where C: RaftTypeConfig
{
    pub(crate) fn enable(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Signal the tick loop to stop. And return a JoinHandle to wait for the loop to stop.
    ///
    /// If it is called twice, the second call will return None.
    pub(crate) fn shutdown(&self) -> Option<JoinHandleOf<C, ()>> {
        {
            let shutdown = {
                let mut shutdown_guard = lock_option_mutex(&self.shutdown, "tick.shutdown");
                shutdown_guard.take()
            };

            if let Some(shutdown) = shutdown {
                let send_res = shutdown.send(());
                tracing::info!("Timer shutdown signal sent: {send_res:?}");
            } else {
                tracing::warn!("Double call to Raft::shutdown()");
            }
        }

        {
            let mut join_handle_guard = lock_option_mutex(&self.join_handle, "tick.join_handle");
            join_handle_guard.take()
        }
    }
}

// AsyncRuntime::spawn is `spawn_local` with singlethreaded enabled.
// It will result in a panic:
// `spawn_local` called from outside of a `task::LocalSet`.
#[cfg(not(feature = "singlethreaded"))]
#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::time::Duration;

    use crate::OptionalSend;
    use crate::RaftTypeConfig;
    use crate::core::Tick;
    use crate::impls::TokioRuntime;
    use crate::type_config::TypeConfigExt;

    #[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub(crate) struct TickUTConfig {}
    impl RaftTypeConfig for TickUTConfig {
        type D = u64;
        type R = ();
        type NodeId = u64;
        type Node = ();
        type Term = u64;
        type LeaderId = crate::impls::leader_id_adv::LeaderId<Self>;
        type Vote = crate::impls::Vote<Self>;
        type Entry = crate::Entry<Self>;
        type SnapshotData = Cursor<Vec<u8>>;
        type AsyncRuntime = TokioRuntime;
        type Responder<T>
            = crate::impls::OneshotResponder<Self, T>
        where T: OptionalSend + 'static;
    }

    #[tokio::test]
    async fn test_shutdown() -> anyhow::Result<()> {
        let (tx, mut rx) = TickUTConfig::mpsc(1024);
        let th = Tick::<TickUTConfig>::spawn(Duration::from_millis(100), tx, true);

        TickUTConfig::sleep(Duration::from_millis(500)).await;
        let _ = th.shutdown().unwrap().await;
        TickUTConfig::sleep(Duration::from_millis(500)).await;

        let mut received = vec![];
        while let Some(x) = rx.recv().await {
            received.push(x);
        }

        assert!(received.len() < 10, "no more tick will be received after shutdown: {}", received.len());

        Ok(())
    }
}
