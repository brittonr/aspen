//! Finite unary buffers: indexing is always refused, never emulated.
use std::pin;
use std::sync;
use std::task;

use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

pub struct Reader(pub std::io::Cursor<Vec<u8>>);
#[derive(Clone, Default)]
pub struct Writer(sync::Arc<sync::Mutex<Vec<u8>>>);
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
        mut self: pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut io::ReadBuf<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
impl AsyncWrite for Writer {
    fn poll_write(
        self: pin::Pin<&mut Self>,
        _: &mut task::Context<'_>,
        bytes: &[u8],
    ) -> task::Poll<std::io::Result<usize>> {
        let mut value = match self.0.lock() {
            Ok(value) => value,
            Err(_) => return task::Poll::Ready(Err(std::io::Error::other("poisoned result buffer"))),
        };
        if value.len().saturating_add(bytes.len()) > super::MAX_BYTES {
            return task::Poll::Ready(Err(std::io::Error::other("RPC result exceeds finite budget")));
        }
        value.extend_from_slice(bytes);
        task::Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: pin::Pin<&mut Self>, _: &mut task::Context<'_>) -> task::Poll<std::io::Result<()>> {
        task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: pin::Pin<&mut Self>, _: &mut task::Context<'_>) -> task::Poll<std::io::Result<()>> {
        task::Poll::Ready(Ok(()))
    }
}
