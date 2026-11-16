//! Concurrency-bounded public connection intake for the long-running registry host.
//!
//! `max_connections` bounds connections served at the same time, not connections over the host's
//! lifetime. A connection holds one permit from accept until its response is written; at the bound
//! the host stops accepting and further connections wait in the listener backlog.

/// One fully read request frame and the connection that answers it. Dropping it releases the
/// connection's permit.
pub(super) struct Admitted {
    pub frame: Vec<u8>,
    pub stream: tokio::net::TcpStream,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

pub(super) struct Intake {
    permits: std::sync::Arc<tokio::sync::Semaphore>,
    readers: tokio::task::JoinSet<()>,
    sender: tokio::sync::mpsc::Sender<Admitted>,
    admitted: tokio::sync::mpsc::Receiver<Admitted>,
    read_timeout: std::time::Duration,
}

impl Intake {
    // r[impl molten.consensus.long_running_service_bounds]
    pub(super) fn new(max_connections: u64, read_timeout: std::time::Duration) -> crate::error::Result<Self> {
        let bound = usize::try_from(max_connections)
            .ok()
            .filter(|bound| *bound > 0)
            .ok_or_else(|| crate::error::Failure::invalid_harness("public connection bound must be positive"))?;
        let (sender, admitted) = tokio::sync::mpsc::channel(bound);
        Ok(Self {
            permits: std::sync::Arc::new(tokio::sync::Semaphore::new(bound)),
            readers: tokio::task::JoinSet::new(),
            sender,
            admitted,
            read_timeout,
        })
    }

    /// Whether another connection may be accepted now.
    pub(super) fn has_capacity(&self) -> bool {
        self.permits.available_permits() > 0
    }

    /// Read an accepted connection's request concurrently under one permit. Only this intake
    /// acquires permits, so capacity observed before accepting is still available here.
    pub(super) fn admit(&mut self, stream: tokio::net::TcpStream) -> crate::error::Result<()> {
        let permit = std::sync::Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(|_| crate::error::Failure::invalid_harness("public connection accepted above its bound"))?;
        while self.readers.try_join_next().is_some() {}
        let sender = self.sender.clone();
        let read_timeout = self.read_timeout;
        self.readers.spawn(async move {
            let mut stream = stream;
            // A connection that sends no complete frame in time is closed and frees its permit.
            let Ok(Ok(frame)) = tokio::time::timeout(read_timeout, super::read_frame(&mut stream)).await else {
                return;
            };
            let _delivered = sender
                .send(Admitted {
                    frame,
                    stream,
                    _permit: permit,
                })
                .await;
        });
        Ok(())
    }

    /// The next fully read request. Cancel-safe; never ends while the intake exists.
    pub(super) async fn next(&mut self) -> Admitted {
        loop {
            if let Some(admitted) = self.admitted.recv().await {
                return admitted;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncBufReadExt as _;
    use tokio::io::AsyncWriteExt as _;

    const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    const BLOCKED_WINDOW: std::time::Duration = std::time::Duration::from_millis(200);

    /// The host's intake step: accept while below the bound, otherwise answer the next request.
    async fn serve_one(listener: &tokio::net::TcpListener, intake: &mut super::Intake) -> Vec<u8> {
        loop {
            tokio::select! {
                accepted = listener.accept(), if intake.has_capacity() => {
                    intake.admit(accepted.expect("accept").0).expect("admit below bound");
                }
                admitted = intake.next() => {
                    let super::Admitted { frame, mut stream, .. } = admitted;
                    stream.write_all(&frame).await.expect("answer");
                    return frame;
                }
            }
        }
    }

    async fn exchange(address: std::net::SocketAddr, frame: &[u8]) -> Vec<u8> {
        let mut stream = tokio::net::TcpStream::connect(address).await.expect("connect");
        stream.write_all(frame).await.expect("send");
        read_line(stream).await
    }

    async fn read_line(stream: tokio::net::TcpStream) -> Vec<u8> {
        let mut response = Vec::new();
        tokio::io::BufReader::new(stream).read_until(b'\n', &mut response).await.expect("response");
        response
    }

    // r[verify molten.consensus.long_running_service_bounds]
    #[tokio::test]
    async fn more_sequential_requests_than_the_bound_are_all_served() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let mut intake = super::Intake::new(2, READ_TIMEOUT).expect("intake");
        for index in 0..7 {
            let frame = format!("request-{index}\n").into_bytes();
            let (served, answered) = tokio::join!(serve_one(&listener, &mut intake), exchange(address, &frame));
            assert_eq!(served, frame);
            assert_eq!(answered, frame);
        }
        assert!(intake.has_capacity());
    }

    // r[verify molten.consensus.long_running_service_bounds]
    #[tokio::test]
    async fn a_connection_above_the_bound_waits_for_a_permit_instead_of_failing() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let mut intake = super::Intake::new(1, READ_TIMEOUT).expect("intake");
        let mut first = tokio::net::TcpStream::connect(address).await.expect("first connect");
        intake.admit(listener.accept().await.expect("first accept").0).expect("first admit");
        assert!(!intake.has_capacity());

        let second = tokio::spawn(exchange(address, b"second\n"));
        let blocked = tokio::time::timeout(BLOCKED_WINDOW, serve_one(&listener, &mut intake)).await;
        assert!(blocked.is_err(), "nothing is served while the only permit waits on the first request");

        first.write_all(b"first\n").await.expect("first request");
        assert_eq!(serve_one(&listener, &mut intake).await, b"first\n");
        assert_eq!(read_line(first).await, b"first\n");
        assert_eq!(serve_one(&listener, &mut intake).await, b"second\n");
        assert_eq!(second.await.expect("second client"), b"second\n");
    }
}
