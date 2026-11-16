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

pub async fn saturation_application(
    listener: tokio::net::UnixListener,
    root: &std::path::Path,
    is_client: bool,
) -> TestResult {
    let (mut socket, _) = listener.accept().await?;
    if is_client {
        let chunk = [0x5a_u8; 8192];
        let mut written = 0_u64;
        let mut has_observed_backpressure = false;
        while written < config::SATURATION_BYTES {
            socket.writable().await?;
            let remaining = usize::try_from(config::SATURATION_BYTES - written)?;
            match socket.try_write(&chunk[..remaining.min(chunk.len())]) {
                Ok(0) => return Err("protected producer stopped before admitted byte bound".into()),
                Ok(count) => written += count as u64,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if !has_observed_backpressure {
                        has_observed_backpressure = true;
                        atomic_value(
                            &root.join("producer-blocked"),
                            &molten::preserves_rail::string("local-write-would-block"),
                        )?;
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }
        assert!(has_observed_backpressure, "bulk producer never observed local backpressure");
        atomic_value(&root.join("producer-finished"), &molten::preserves_rail::string("all-bulk-bytes-written"))?;
        socket.shutdown().await?;
        let mut received = Vec::new();
        socket.take(MAX_ARTIFACT_BYTES).read_to_end(&mut received).await?;
        assert_eq!(received, config::RESPONSE, "bulk consumer lost protected response");
    } else {
        atomic_value(&root.join("receiver-paused"), &molten::preserves_rail::string("protected-socket-accepted"))?;
        tokio::time::timeout(std::time::Duration::from_secs(6), async {
            while !root.join("release-receiver").exists() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await?;
        let mut received = 0_u64;
        let mut chunk = [0_u8; 8192];
        loop {
            let count = socket.read(&mut chunk).await?;
            if count == 0 {
                break;
            }
            assert!(chunk[..count].iter().all(|byte| *byte == 0x5a), "protected bulk payload changed");
            received += count as u64;
            assert!(received <= config::SATURATION_BYTES, "protected receiver exceeded admitted bound");
        }
        assert_eq!(received, config::SATURATION_BYTES, "protected receiver missed bulk bytes");
        socket.write_all(config::RESPONSE).await?;
        socket.shutdown().await?;
    }
    Ok(())
}
