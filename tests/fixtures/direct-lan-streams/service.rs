use crate::*;

pub struct SocketService {
    pub path: std::path::PathBuf,
    pub expected: molten::fabric_transport::DirectStreamRequest,
    pub peer: molten::fabric_transport::DirectPeerBinding,
    pub revoked: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub opens: usize,
}

impl SocketService {
    fn current(&self, request: &molten::fabric_transport::DirectStreamRequest) -> molten::fabric::FabricPortResult<()> {
        if self.revoked.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(molten::fabric::FabricPortError::capability("consumer operation authority revoked"));
        }
        if request.request_ref != self.expected.request_ref
            || request.operation_ref != self.expected.operation_ref
            || request.resource_ref != self.expected.resource_ref
            || request.generation != self.expected.generation
        {
            return Err(molten::fabric::FabricPortError::capability("consumer operation scope mismatch"));
        }
        Ok(())
    }
}

impl molten::fabric_transport::ProtectedSocketService for SocketService {
    fn authorize(
        &mut self,
        gated: molten::fabric_transport::GatedInbound,
        request: &molten::fabric_transport::DirectStreamRequest,
    ) -> molten::fabric::FabricPortResult<String> {
        self.current(request)?;
        let observation = gated.observation();
        if observation.peer_identity != self.peer.endpoint_id
            || observation.peer_context_ref != self.peer.peer_context_ref
            || observation.service_id != self.peer.service_id
            || observation.alpn != self.peer.alpn
            || observation.generation != self.peer.generation
            || observation.role != self.peer.remote_role
        {
            return Err(molten::fabric::FabricPortError::capability("consumer peer scope mismatch"));
        }
        reference("consumer-current-authority")
    }

    async fn open(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<tokio::net::UnixStream> {
        self.check_current(admitted)?;
        self.opens += 1;
        tokio::net::UnixStream::connect(&self.path)
            .await
            .map_err(|error| molten::fabric::FabricPortError::transport(error.to_string()))
    }

    fn check_current(
        &mut self,
        admitted: &molten::fabric_transport::direct::AdmittedOperation,
    ) -> molten::fabric::FabricPortResult<()> {
        self.current(admitted.request())?;
        if admitted.authority_ref() != reference("consumer-current-authority")? {
            return Err(molten::fabric::FabricPortError::capability("consumer authority evidence changed"));
        }
        Ok(())
    }
}

pub async fn application_exchange(
    listener: tokio::net::UnixListener,
    is_client: bool,
    request: &'static [u8],
    response: &'static [u8],
) -> TestResult {
    let (mut socket, _) = listener.accept().await?;
    if is_client {
        socket.write_all(request).await?;
        socket.shutdown().await?;
        let mut received = Vec::new();
        socket.take(MAX_ARTIFACT_BYTES).read_to_end(&mut received).await?;
        assert_eq!(received, response, "local consumer did not observe exact protected-service response");
    } else {
        let mut received = Vec::new();
        (&mut socket).take(MAX_ARTIFACT_BYTES).read_to_end(&mut received).await?;
        assert_eq!(received, request, "protected service did not observe exact consumer request");
        socket.write_all(response).await?;
        socket.shutdown().await?;
    }
    Ok(())
}
