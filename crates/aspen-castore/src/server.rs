//! Castore irpc server.
//!
//! Receives irpc messages and dispatches to the underlying blob and directory stores.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use irpc::WithChannels;
use irpc::rpc::RemoteService;
use prost::Message;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::Node;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use tokio::io::AsyncWriteExt;
use tracing::debug;
use tracing::instrument;
use tracing::trace_span;
use tracing::warn;

use crate::protocol::*;

/// Maximum depth for recursive directory traversal.
const MAX_RECURSIVE_DEPTH: u32 = 256;

/// Maximum directories to buffer during recursive walk.
const MAX_RECURSIVE_BUFFER: u32 = 10_000;

/// Castore server that handles irpc messages.
///
/// Generic over the blob and directory service implementations.
pub struct CastoreServer<B, D> {
    blob: Arc<B>,
    dir: Arc<D>,
}

impl<B, D> Clone for CastoreServer<B, D> {
    fn clone(&self) -> Self {
        Self {
            blob: Arc::clone(&self.blob),
            dir: Arc::clone(&self.dir),
        }
    }
}

impl<B, D> CastoreServer<B, D>
where
    B: BlobService + 'static,
    D: DirectoryService + 'static,
{
    /// Create a new castore server with the given blob and directory backends.
    pub fn new(blob: B, dir: D) -> Self {
        Self {
            blob: Arc::new(blob),
            dir: Arc::new(dir),
        }
    }

    /// Create from Arc'd services (for sharing with other components).
    pub fn from_arcs(blob: Arc<B>, dir: Arc<D>) -> Self {
        Self { blob, dir }
    }

    /// Handle one incoming irpc message.
    pub async fn handle(&self, msg: CastoreMessage) {
        match msg {
            CastoreMessage::BlobHas(msg) => self.handle_blob_has(msg).await,
            CastoreMessage::BlobRead(msg) => self.handle_blob_read(msg).await,
            CastoreMessage::BlobWrite(msg) => self.handle_blob_write(msg).await,
            CastoreMessage::DirGet(msg) => self.handle_dir_get(msg).await,
            CastoreMessage::DirPut(msg) => self.handle_dir_put(msg).await,
            CastoreMessage::DirGetRecursive(msg) => self.handle_dir_get_recursive(msg).await,
            CastoreMessage::DirPutMultiple(msg) => self.handle_dir_put_multiple(msg).await,
        }
    }

    #[instrument(skip_all, fields(digest = hex::encode(msg.digest)))]
    async fn handle_blob_has(&self, msg: WithChannels<BlobHas, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let digest = B3Digest::from(&inner.digest);
        let result = match self.blob.has(&digest).await {
            Ok(exists) => exists,
            Err(e) => {
                warn!(error = %e, "blob has failed");
                false
            }
        };
        let _ = tx.send(result).await;
    }

    #[instrument(skip_all, fields(digest = hex::encode(msg.digest)))]
    async fn handle_blob_read(&self, msg: WithChannels<BlobRead, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let digest = B3Digest::from(&inner.digest);
        let response = match self.blob.open_read(&digest).await {
            Ok(Some(mut reader)) => {
                let mut buf = Vec::new();
                match tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buf).await {
                    Ok(_) => {
                        debug!(size = buf.len(), "blob read");
                        BlobReadResponse { data: Some(buf) }
                    }
                    Err(e) => {
                        warn!(error = %e, "blob read_to_end failed");
                        BlobReadResponse { data: None }
                    }
                }
            }
            Ok(None) => {
                debug!("blob not found");
                BlobReadResponse { data: None }
            }
            Err(e) => {
                warn!(error = %e, "blob open_read failed");
                BlobReadResponse { data: None }
            }
        };
        let _ = tx.send(response).await;
    }

    #[instrument(skip_all, fields(size = msg.data.len()))]
    async fn handle_blob_write(&self, msg: WithChannels<BlobWrite, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let mut writer = self.blob.open_write().await;
        let result = async {
            writer.write_all(&inner.data).await?;
            writer.close().await
        }
        .await;

        match result {
            Ok(digest) => {
                debug!(digest = %digest, "blob written");
                let _ = tx.send(*digest.as_ref()).await;
            }
            Err(e) => {
                warn!(error = %e, "blob write failed");
                // Send zeroed digest on error — caller will see it doesn't match
                let _ = tx.send([0u8; 32]).await;
            }
        }
    }

    #[instrument(skip_all, fields(digest = hex::encode(msg.digest)))]
    async fn handle_dir_get(&self, msg: WithChannels<DirGet, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let digest = B3Digest::from(&inner.digest);
        let response = match self.dir.get(&digest).await {
            Ok(Some(dir)) => {
                let proto = snix_castore::proto::Directory::from(dir);
                debug!("directory found");
                DirGetResponse {
                    data: Some(proto.encode_to_vec()),
                }
            }
            Ok(None) => {
                debug!("directory not found");
                DirGetResponse { data: None }
            }
            Err(e) => {
                warn!(error = %e, "directory get failed");
                DirGetResponse { data: None }
            }
        };
        let _ = tx.send(response).await;
    }

    #[instrument(skip_all, fields(data_len = msg.data.len()))]
    async fn handle_dir_put(&self, msg: WithChannels<DirPut, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let result = decode_and_put_directory(&*self.dir, &inner.data).await;
        match result {
            Ok(digest) => {
                debug!(digest = hex::encode(digest), "directory stored");
                let _ = tx.send(digest).await;
            }
            Err(e) => {
                warn!(error = %e, "directory put failed");
                let _ = tx.send([0u8; 32]).await;
            }
        }
    }

    #[instrument(skip_all, fields(digest = hex::encode(msg.digest)))]
    async fn handle_dir_get_recursive(&self, msg: WithChannels<DirGetRecursive, CastoreProtocol>) {
        let WithChannels { inner, tx, .. } = msg;
        let root = B3Digest::from(&inner.digest);

        // BFS traversal, root-to-leaves
        let mut queue = std::collections::VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        queue.push_back(root);
        let mut depth: u32 = 0;

        while let Some(digest) = queue.pop_front() {
            if visited.contains(&digest) {
                continue;
            }
            visited.insert(digest);

            if depth > MAX_RECURSIVE_DEPTH {
                let _ = tx
                    .send(DirGetRecursiveItem {
                        result: Err(format!("max depth {MAX_RECURSIVE_DEPTH} exceeded")),
                    })
                    .await;
                return;
            }
            if queue.len() as u32 > MAX_RECURSIVE_BUFFER {
                let _ = tx
                    .send(DirGetRecursiveItem {
                        result: Err(format!("max buffer {MAX_RECURSIVE_BUFFER} exceeded")),
                    })
                    .await;
                return;
            }

            match self.dir.get(&digest).await {
                Ok(Some(dir)) => {
                    // Queue child directories
                    for (_name, node) in dir.nodes() {
                        if let Node::Directory {
                            digest: child_digest, ..
                        } = node
                            && !visited.contains(child_digest)
                        {
                            queue.push_back(*child_digest);
                        }
                    }

                    let proto = snix_castore::proto::Directory::from(dir);
                    if tx
                        .send(DirGetRecursiveItem {
                            result: Ok(proto.encode_to_vec()),
                        })
                        .await
                        .is_err()
                    {
                        return; // client dropped
                    }
                    depth += 1;
                }
                Ok(None) => {
                    // Directory not found — skip (matches snix behavior)
                }
                Err(e) => {
                    let _ = tx
                        .send(DirGetRecursiveItem {
                            result: Err(format!("directory get error: {e}")),
                        })
                        .await;
                    return;
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn handle_dir_put_multiple(&self, msg: WithChannels<DirPutMultiple, CastoreProtocol>) {
        let WithChannels { tx, mut rx, .. } = msg;
        let mut putter = self.dir.put_multiple_start();
        let mut count: u32 = 0;

        // Receive directories from the client stream
        while let Ok(Some(item)) = rx.recv().await {
            match decode_directory(&item.data) {
                Ok(dir) => {
                    if let Err(e) = putter.put(dir).await {
                        let _ = tx
                            .send(DirPutMultipleResponse {
                                result: Err(format!("put error: {e}")),
                            })
                            .await;
                        return;
                    }
                    count += 1;
                }
                Err(e) => {
                    let _ = tx
                        .send(DirPutMultipleResponse {
                            result: Err(format!("decode error: {e}")),
                        })
                        .await;
                    return;
                }
            }
        }

        // Close and get root digest
        match putter.close().await {
            Ok(digest) => {
                debug!(count, digest = %digest, "batch put complete");
                let _ = tx
                    .send(DirPutMultipleResponse {
                        result: Ok(*digest.as_ref()),
                    })
                    .await;
            }
            Err(e) => {
                warn!(error = %e, count, "batch put close failed");
                let _ = tx
                    .send(DirPutMultipleResponse {
                        result: Err(format!("close error: {e}")),
                    })
                    .await;
            }
        }
    }

    /// Spawn the server actor, returning a tokio sender for feeding messages.
    pub fn spawn(self) -> tokio::sync::mpsc::Sender<CastoreMessage> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<CastoreMessage>(64);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let server = self.clone();
                tokio::spawn(async move { server.handle(msg).await });
            }
        });
        tx
    }

    /// Create an iroh [`ProtocolHandler`] for this server.
    ///
    /// Register the returned handler on an iroh `Router` with [`CASTORE_ALPN`]:
    ///
    /// ```ignore
    /// let handler = server.into_protocol_handler();
    /// let router = Router::builder(endpoint)
    ///     .accept(CASTORE_ALPN.to_vec(), handler)
    ///     .spawn();
    /// ```
    ///
    /// [`CASTORE_ALPN`]: crate::CASTORE_ALPN
    pub fn into_protocol_handler(self) -> CastoreProtocolHandler {
        let tx = self.spawn();
        let local = irpc::LocalSender::<CastoreProtocol>::from(tx);
        let handler = CastoreProtocol::remote_handler(local);
        CastoreProtocolHandler {
            handler,
            request_id: AtomicU64::new(0),
        }
    }
}

// ---------------------------------------------------------------------------
// iroh ProtocolHandler
// ---------------------------------------------------------------------------

/// iroh [`ProtocolHandler`] that bridges incoming QUIC connections to the
/// castore irpc actor.
///
/// Created via [`CastoreServer::into_protocol_handler`].
pub struct CastoreProtocolHandler {
    handler: irpc::rpc::Handler<CastoreProtocol>,
    request_id: AtomicU64,
}

impl std::fmt::Debug for CastoreProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CastoreProtocolHandler").finish()
    }
}

impl iroh::protocol::ProtocolHandler for CastoreProtocolHandler {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), iroh::protocol::AcceptError> {
        let handler = self.handler.clone();
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let span = trace_span!("castore-rpc", id);
        async { handle_irpc_connection(&connection, handler).await.map_err(iroh::protocol::AcceptError::from_err) }
            .instrument(span)
            .await
    }
}

use tracing::Instrument;

/// Read irpc requests from a QUIC connection and dispatch them.
async fn handle_irpc_connection(
    connection: &iroh::endpoint::Connection,
    handler: irpc::rpc::Handler<CastoreProtocol>,
) -> std::io::Result<()> {
    debug!(remote = %connection.remote_id().fmt_short(), "castore connection accepted");
    loop {
        let (send, mut recv) = match connection.accept_bi().await {
            Ok(pair) => pair,
            Err(iroh::endpoint::ConnectionError::ApplicationClosed(cause)) if cause.error_code.into_inner() == 0 => {
                debug!("remote closed connection");
                return Ok(());
            }
            Err(e) => {
                warn!(error = %e, "failed to accept bi stream");
                return Err(e.into());
            }
        };

        // Read length-prefixed postcard message (irpc wire format)
        let size = irpc::util::AsyncReadVarintExt::read_varint_u64(&mut recv)
            .await?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no size"))?;

        if size > irpc::rpc::MAX_MESSAGE_SIZE {
            connection.close(irpc::rpc::ERROR_CODE_MAX_MESSAGE_SIZE_EXCEEDED.into(), b"max message size exceeded");
            return Err(std::io::Error::other("max message size exceeded"));
        }

        let mut buf = vec![0u8; size as usize];
        recv.read_exact(&mut buf)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e))?;

        let msg: CastoreProtocol =
            postcard::from_bytes(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        handler(msg, recv, send).await?;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decode_directory(data: &[u8]) -> Result<Directory, String> {
    let proto = snix_castore::proto::Directory::decode(data).map_err(|e| format!("protobuf decode: {e}"))?;
    Directory::try_from(proto).map_err(|e| format!("directory validation: {e}"))
}

async fn decode_and_put_directory<D: DirectoryService>(dir_svc: &D, data: &[u8]) -> Result<[u8; 32], String> {
    let dir = decode_directory(data)?;
    let digest = dir_svc.put(dir).await.map_err(|e| format!("put: {e}"))?;
    Ok(*digest.as_ref())
}
