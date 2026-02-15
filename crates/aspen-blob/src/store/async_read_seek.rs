//! AsyncReadSeek helper trait for streaming blob readers.

use tokio::io::AsyncRead;
use tokio::io::AsyncSeek;

/// Trait combining AsyncRead + AsyncSeek for streaming blob readers.
///
/// This helper trait is needed because Rust doesn't allow multiple non-auto
/// traits in a dyn trait object. By creating a supertrait that requires both,
/// we can use `dyn AsyncReadSeek` as a valid trait object.
pub trait AsyncReadSeek: AsyncRead + AsyncSeek + Send + Unpin {}

/// Blanket implementation for any type implementing both traits.
impl<T: AsyncRead + AsyncSeek + Send + Unpin> AsyncReadSeek for T {}
