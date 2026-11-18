//! Finite unary buffers: indexing is always refused, never emulated.

use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

pub struct Reader(pub std::io::Cursor<Vec<u8>>);
#[derive(Clone, Default)]
pub struct Writer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl Writer {
    pub fn bytes(&self) -> super::Result<Vec<u8>> {
        self.0.lock().map(|value| value.clone()).map_err(super::failure)
    }
}
impl wrpc_transport::Index<Reader> for Reader {
    fn index(&self, _: &[usize]) -> anyhow::Result<Reader> {
        anyhow::bail!("async values are not admitted")
    }
}
impl wrpc_transport::Index<Writer> for Writer {
    fn index(&self, _: &[usize]) -> anyhow::Result<Writer> {
        anyhow::bail!("async values are not admitted")
    }
}
impl AsyncRead for Reader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
impl AsyncWrite for Writer {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut value = match self.0.lock() {
            Ok(value) => value,
            Err(_) => return std::task::Poll::Ready(Err(std::io::Error::other("poisoned result buffer"))),
        };
        if value.len().saturating_add(bytes.len()) > super::MAX_BYTES {
            return std::task::Poll::Ready(Err(std::io::Error::other("RPC result exceeds finite budget")));
        }
        value.extend_from_slice(bytes);
        std::task::Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}
