//! Castore irpc client.
//!
//! Implements snix-castore's [`BlobService`] and [`DirectoryService`] traits
//! by making irpc calls to a remote [`CastoreServer`].

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use async_trait::async_trait;
use prost::Message;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::blobservice::BlobReader;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::BlobWriter;
use snix_castore::directoryservice::DirectoryPutter;
use snix_castore::directoryservice::DirectoryService;
use tracing::debug;
use tracing::instrument;

use crate::protocol::*;

/// Type alias for a boxed stream, matching snix-castore's expectation.
type BoxStream<'a, T> = futures::stream::BoxStream<'a, T>;

type DirError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Convert a 32-byte array to B3Digest.
fn bytes_to_digest(bytes: &[u8; 32]) -> B3Digest {
    B3Digest::from(bytes)
}

// ===========================================================================
// BlobService client
// ===========================================================================

/// irpc-based BlobService client.
///
/// Makes RPC calls to a remote [`CastoreServer`] for all blob operations.
#[derive(Clone)]
pub struct IrpcBlobService {
    client: irpc::Client<CastoreProtocol>,
}

impl IrpcBlobService {
    /// Create a client connected to a remote castore server via iroh.
    pub fn new(endpoint: iroh::Endpoint, addr: impl Into<iroh::EndpointAddr>) -> Self {
        let conn = IrohConnection::new(endpoint, addr.into(), crate::CASTORE_ALPN.to_vec());
        Self {
            client: irpc::Client::boxed(conn),
        }
    }

    /// Create from an existing irpc client (e.g. for local/in-process use).
    pub fn from_client(client: irpc::Client<CastoreProtocol>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BlobService for IrpcBlobService {
    #[instrument(skip(self), fields(digest = %digest))]
    async fn has(&self, digest: &B3Digest) -> io::Result<bool> {
        let msg = BlobHas {
            digest: *digest.as_ref(),
        };
        self.client.rpc(msg).await.map_err(|e| io::Error::other(format!("irpc blob has: {e}")))
    }

    #[instrument(skip(self), fields(digest = %digest))]
    async fn open_read(&self, digest: &B3Digest) -> io::Result<Option<Box<dyn BlobReader>>> {
        let msg = BlobRead {
            digest: *digest.as_ref(),
        };
        let resp: BlobReadResponse =
            self.client.rpc(msg).await.map_err(|e| io::Error::other(format!("irpc blob read: {e}")))?;

        match resp.data {
            Some(data) => {
                debug!(size = data.len(), "blob read");
                Ok(Some(Box::new(io::Cursor::new(data))))
            }
            None => {
                debug!("blob not found");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self))]
    async fn open_write(&self) -> Box<dyn BlobWriter> {
        Box::new(IrpcBlobWriter {
            client: self.client.clone(),
            buffer: Vec::new(),
            digest: None,
        })
    }
}

/// BlobWriter that buffers locally and uploads on close.
struct IrpcBlobWriter {
    client: irpc::Client<CastoreProtocol>,
    buffer: Vec<u8>,
    digest: Option<B3Digest>,
}

impl tokio::io::AsyncWrite for IrpcBlobWriter {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl Unpin for IrpcBlobWriter {}

#[async_trait]
impl BlobWriter for IrpcBlobWriter {
    async fn close(&mut self) -> io::Result<B3Digest> {
        if let Some(digest) = self.digest {
            return Ok(digest);
        }

        let data = std::mem::take(&mut self.buffer);
        let msg = BlobWrite { data };
        let digest_bytes: [u8; 32] =
            self.client.rpc(msg).await.map_err(|e| io::Error::other(format!("irpc blob write: {e}")))?;

        let digest = bytes_to_digest(&digest_bytes);
        self.digest = Some(digest);
        debug!(digest = %digest, "blob written via irpc");
        Ok(digest)
    }
}

// ===========================================================================
// DirectoryService client
// ===========================================================================

/// irpc-based DirectoryService client.
///
/// Makes RPC calls to a remote [`CastoreServer`] for directory operations.
#[derive(Clone)]
pub struct IrpcDirectoryService {
    client: irpc::Client<CastoreProtocol>,
}

impl IrpcDirectoryService {
    /// Create a client connected to a remote castore server via iroh.
    pub fn new(endpoint: iroh::Endpoint, addr: impl Into<iroh::EndpointAddr>) -> Self {
        let conn = IrohConnection::new(endpoint, addr.into(), crate::CASTORE_ALPN.to_vec());
        Self {
            client: irpc::Client::boxed(conn),
        }
    }

    /// Create from an existing irpc client (e.g. for local/in-process use).
    pub fn from_client(client: irpc::Client<CastoreProtocol>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DirectoryService for IrpcDirectoryService {
    #[instrument(skip(self), fields(digest = %digest))]
    async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>, DirError> {
        let msg = DirGet {
            digest: *digest.as_ref(),
        };
        let resp: DirGetResponse = self.client.rpc(msg).await?;

        match resp.data {
            Some(bytes) => {
                let proto = snix_castore::proto::Directory::decode(bytes.as_slice())?;
                let dir = Directory::try_from(proto)?;
                debug!("directory found");
                Ok(Some(dir))
            }
            None => {
                debug!("directory not found");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, directory), fields(digest = %directory.digest()))]
    async fn put(&self, directory: Directory) -> Result<B3Digest, DirError> {
        let proto = snix_castore::proto::Directory::from(directory);
        let msg = DirPut {
            data: proto.encode_to_vec(),
        };
        let digest_bytes: [u8; 32] = self.client.rpc(msg).await?;
        let digest = bytes_to_digest(&digest_bytes);
        debug!(digest = %digest, "directory stored via irpc");
        Ok(digest)
    }

    fn get_recursive(&self, root_directory_digest: &B3Digest) -> BoxStream<'_, Result<Directory, DirError>> {
        let msg = DirGetRecursive {
            digest: *root_directory_digest.as_ref(),
        };
        let client = self.client.clone();

        Box::pin(async_stream::try_stream! {
            let mut rx = client.server_streaming(msg, 64).await
                .map_err(|e| Box::new(io::Error::other(format!("irpc: {e}"))) as DirError)?;

            while let Some(item) = rx.recv().await
                .map_err(|e| Box::new(io::Error::other(format!("irpc recv: {e}"))) as DirError)?
            {
                match item.result {
                    Ok(bytes) => {
                        let proto = snix_castore::proto::Directory::decode(bytes.as_slice())
                            .map_err(|e| -> DirError { Box::new(e) })?;
                        let dir = Directory::try_from(proto)
                            .map_err(|e| -> DirError { Box::new(e) })?;
                        yield dir;
                    }
                    Err(e) => {
                        Err(Box::new(io::Error::other(e)) as DirError)?;
                    }
                }
            }
        })
    }

    fn put_multiple_start(&self) -> Box<dyn DirectoryPutter + '_> {
        Box::new(IrpcDirectoryPutter {
            client: self.client.clone(),
            started: false,
            tx: None,
            rx: None,
        })
    }
}

/// DirectoryPutter that streams directories to the server.
struct IrpcDirectoryPutter {
    client: irpc::Client<CastoreProtocol>,
    started: bool,
    tx: Option<irpc::channel::mpsc::Sender<DirPutItem>>,
    rx: Option<irpc::channel::oneshot::Receiver<DirPutMultipleResponse>>,
}

impl IrpcDirectoryPutter {
    async fn ensure_started(&mut self) -> Result<&irpc::channel::mpsc::Sender<DirPutItem>, DirError> {
        if !self.started {
            let (tx, rx) = self
                .client
                .client_streaming(DirPutMultiple, 64)
                .await
                .map_err(|e| Box::new(io::Error::other(format!("irpc: {e}"))) as DirError)?;
            self.tx = Some(tx);
            self.rx = Some(rx);
            self.started = true;
        }
        self.tx.as_ref().ok_or_else(|| Box::new(io::Error::other("putter already closed")) as DirError)
    }
}

#[async_trait]
impl DirectoryPutter for IrpcDirectoryPutter {
    async fn put(&mut self, directory: Directory) -> Result<(), DirError> {
        let tx = self.ensure_started().await?;
        let proto = snix_castore::proto::Directory::from(directory);
        tx.send(DirPutItem {
            data: proto.encode_to_vec(),
        })
        .await
        .map_err(|e| Box::new(io::Error::other(format!("irpc send: {e}"))) as DirError)?;
        Ok(())
    }

    async fn close(&mut self) -> Result<B3Digest, DirError> {
        if !self.started {
            return Err(Box::new(io::Error::other("no directories were put")));
        }

        // Drop tx to signal end of stream
        self.tx.take();

        let rx = self.rx.take().ok_or_else(|| Box::new(io::Error::other("putter already closed")) as DirError)?;

        let resp: DirPutMultipleResponse =
            rx.await.map_err(|e| Box::new(io::Error::other(format!("irpc recv: {e}"))) as DirError)?;
        match resp.result {
            Ok(digest_bytes) => Ok(bytes_to_digest(&digest_bytes)),
            Err(e) => Err(Box::new(io::Error::other(e))),
        }
    }
}

// ===========================================================================
// Iroh transport for irpc (inline, avoids irpc-iroh version mismatch)
// ===========================================================================

/// Lazy iroh connection that implements irpc's RemoteConnection.
///
/// Connects on first use, reuses the connection for subsequent requests,
/// and reconnects if the connection drops.
#[derive(Debug, Clone)]
struct IrohConnection(Arc<IrohConnectionInner>);

#[derive(Debug)]
struct IrohConnectionInner {
    endpoint: iroh::Endpoint,
    addr: iroh::EndpointAddr,
    alpn: Vec<u8>,
    connection: tokio::sync::Mutex<Option<iroh::endpoint::Connection>>,
}

impl IrohConnection {
    fn new(endpoint: iroh::Endpoint, addr: iroh::EndpointAddr, alpn: Vec<u8>) -> Self {
        Self(Arc::new(IrohConnectionInner {
            endpoint,
            addr,
            alpn,
            connection: tokio::sync::Mutex::new(None),
        }))
    }
}

impl irpc::rpc::RemoteConnection for IrohConnection {
    fn clone_boxed(&self) -> Box<dyn irpc::rpc::RemoteConnection> {
        Box::new(self.clone())
    }

    fn open_bi(
        &self,
    ) -> n0_future::future::Boxed<Result<(iroh::endpoint::SendStream, iroh::endpoint::RecvStream), irpc::RequestError>>
    {
        let inner = self.0.clone();
        Box::pin(async move {
            let mut guard = inner.connection.lock().await;
            if let Some(conn) = guard.as_ref() {
                if let Ok(pair) = conn.open_bi().await {
                    return Ok(pair);
                }
                *guard = None;
            }
            // Connect fresh
            let conn =
                inner.endpoint.connect(inner.addr.clone(), &inner.alpn).await.map_err(|e| -> irpc::RequestError {
                    let any: n0_error::AnyError = io::Error::other(format!("iroh connect: {e}")).into();
                    any.into()
                })?;
            let pair = conn.open_bi().await?;
            *guard = Some(conn);
            Ok(pair)
        })
    }

    fn zero_rtt_accepted(&self) -> n0_future::future::Boxed<bool> {
        Box::pin(async { true })
    }
}
