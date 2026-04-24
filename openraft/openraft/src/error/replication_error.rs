use crate::RaftTypeConfig;
use crate::StorageError;
use crate::error::RPCError;
use crate::error::higher_vote::HigherVote;
use crate::error::replication_closed::ReplicationClosed;

/// Error variants related to the Replication.
#[derive(Debug, thiserror::Error)]
// Justified: this internal boundary preserves full storage and RPC error context instead of
// boxing the largest variants, because replication callers immediately branch on those errors.
#[allow(clippy::large_enum_variant, reason = "storage and RPC errors intentionally keep their full context on this internal error boundary")]
pub(crate) enum ReplicationError<C>
where C: RaftTypeConfig
{
    #[error(transparent)]
    HigherVote(#[from] HigherVote<C>),

    #[error(transparent)]
    Closed(#[from] ReplicationClosed),

    #[error(transparent)]
    StorageError(#[from] StorageError<C>),

    #[error(transparent)]
    RpcError(#[from] RPCError<C>),
}
