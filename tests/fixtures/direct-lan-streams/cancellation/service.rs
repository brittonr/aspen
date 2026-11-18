use tokio::io::AsyncWriteExt;

pub(super) const OPEN_EFFECT: u8 = 0x73;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenAction {
    Ready,
    HoldAfterEffect,
    FailAfterEffect,
}

pub(super) struct Service {
    authority: crate::service::SocketService,
    socket: Option<tokio::net::UnixStream>,
    action: OpenAction,
    release: std::sync::Arc<tokio::sync::Notify>,
}

impl Service {
    pub(super) fn new(socket: tokio::net::UnixStream, is_client: bool, action: OpenAction) -> crate::TestResult<Self> {
        Ok(Self {
            authority: crate::service::SocketService {
                path: std::path::PathBuf::new(),
                expected: crate::config::request(1)?,
                peer: crate::config::peer(is_client, 1)?,
                revoked: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                opens: 0,
            },
            socket: Some(socket),
            action,
            release: std::sync::Arc::new(tokio::sync::Notify::new()),
        })
    }

    pub(super) fn release(&self) -> std::sync::Arc<tokio::sync::Notify> {
        std::sync::Arc::clone(&self.release)
    }
}

impl molten::fabric_transport::ProtectedSocketService for Service {
    fn authorize(
        &mut self,
        inbound: molten::fabric_transport::GatedInbound,
        request: &molten::fabric_transport::DirectStreamRequest,
    ) -> molten::fabric::FabricPortResult<String> {
        self.authority.authorize(inbound, request)
    }

    fn check_current(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<()> {
        self.authority.check_current(admitted)
    }

    async fn open(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<tokio::net::UnixStream> {
        self.check_current(admitted)?;
        let mut socket = self
            .socket
            .take()
            .ok_or_else(|| molten::fabric::FabricPortError::malformed("test service opened twice"))?;
        if self.action != OpenAction::Ready {
            // The independent protected peer observes a real write before the
            // open future can return. Cancellation cannot undo that effect.
            socket
                .write_all(&[OPEN_EFFECT])
                .await
                .map_err(|error| molten::fabric::FabricPortError::transport(error.to_string()))?;
            self.release.notified().await;
            if self.action == OpenAction::FailAfterEffect {
                return Err(molten::fabric::FabricPortError::transport("injected failure after protected write"));
            }
        }
        Ok(socket)
    }
}
