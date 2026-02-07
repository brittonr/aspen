//! Automerge Sync Protocol Specification
//!
//! Formal specification for sync message limits and timeouts.
//!
//! # Properties
//!
//! 1. **AM-6: Sync Message Bound**: msg_size <= MAX
//! 2. **AM-7: Sync Buffer Bound**: buffer_size <= MAX
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-automerge/verus/sync_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum document size (base)
    pub const MAX_DOCUMENT_SIZE: u64 = 16 * 1024 * 1024;

    /// Sync metadata overhead (64 KB)
    pub const SYNC_METADATA_OVERHEAD: u64 = 64 * 1024;

    /// Maximum sync message size
    pub const MAX_SYNC_MESSAGE_SIZE: u64 = MAX_DOCUMENT_SIZE + SYNC_METADATA_OVERHEAD;

    /// Maximum sync buffer size (number of messages)
    pub const MAX_SYNC_BUFFER: u64 = 100;

    /// Sync timeout in seconds
    pub const SYNC_TIMEOUT_SECS: u64 = 30;

    /// Background sync interval in seconds
    pub const BACKGROUND_SYNC_INTERVAL_SECS: u64 = 60;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract sync message
    pub struct SyncMessageSpec {
        /// Size of the message in bytes
        pub size: u64,
        /// Message type
        pub msg_type: SyncMessageType,
    }

    /// Sync message types
    pub enum SyncMessageType {
        /// Initial sync request (bloom filter, heads)
        Request,
        /// Sync response with changes
        Response,
        /// Acknowledgment
        Ack,
    }

    /// Abstract sync buffer
    pub struct SyncBufferSpec {
        /// Buffered messages
        pub messages: Seq<SyncMessageSpec>,
        /// Total size of buffered messages
        pub total_size: u64,
    }

    /// Abstract sync session
    pub struct SyncSessionSpec {
        /// Remote peer ID
        pub peer_id: Seq<u8>,
        /// Start time of sync
        pub start_time: u64,
        /// Inbound message buffer
        pub inbound_buffer: SyncBufferSpec,
        /// Outbound message buffer
        pub outbound_buffer: SyncBufferSpec,
        /// Whether sync is complete
        pub complete: bool,
    }

    // ========================================================================
    // AM-6: Sync Message Bound
    // ========================================================================

    /// Sync message is within size limits
    pub open spec fn sync_message_bounded(msg: SyncMessageSpec) -> bool {
        msg.size <= MAX_SYNC_MESSAGE_SIZE
    }

    /// Proof: Reject oversized messages
    pub proof fn reject_oversized_messages(size: u64)
        requires size > MAX_SYNC_MESSAGE_SIZE
        ensures !sync_message_bounded(SyncMessageSpec {
            size,
            msg_type: SyncMessageType::Response,
        })
    {
        // size > MAX => !bounded
    }

    // ========================================================================
    // AM-7: Sync Buffer Bound
    // ========================================================================

    /// Sync buffer has bounded message count
    pub open spec fn sync_buffer_bounded(buffer: SyncBufferSpec) -> bool {
        buffer.messages.len() <= MAX_SYNC_BUFFER
    }

    /// All messages in buffer are bounded
    pub open spec fn buffer_messages_bounded(buffer: SyncBufferSpec) -> bool {
        forall |i: int| 0 <= i < buffer.messages.len() ==>
            sync_message_bounded(buffer.messages[i])
    }

    /// Full buffer invariant
    pub open spec fn buffer_invariant(buffer: SyncBufferSpec) -> bool {
        sync_buffer_bounded(buffer) &&
        buffer_messages_bounded(buffer)
    }

    // ========================================================================
    // Buffer Operations
    // ========================================================================

    /// Add message to buffer
    pub open spec fn add_to_buffer_effect(
        pre: SyncBufferSpec,
        post: SyncBufferSpec,
        msg: SyncMessageSpec,
    ) -> bool {
        sync_message_bounded(msg) &&
        post.messages.len() == pre.messages.len() + 1 &&
        post.messages[post.messages.len() - 1] == msg &&
        post.total_size == pre.total_size + msg.size
    }

    /// Can add to buffer
    pub open spec fn can_add_to_buffer(buffer: SyncBufferSpec) -> bool {
        buffer.messages.len() < MAX_SYNC_BUFFER
    }

    /// Proof: Adding preserves buffer invariant
    pub proof fn add_preserves_invariant(
        pre: SyncBufferSpec,
        post: SyncBufferSpec,
        msg: SyncMessageSpec,
    )
        requires
            buffer_invariant(pre),
            can_add_to_buffer(pre),
            add_to_buffer_effect(pre, post, msg),
        ensures buffer_invariant(post)
    {
        // count + 1 <= MAX
        // new message bounded
    }

    /// Remove message from buffer
    pub open spec fn remove_from_buffer_effect(
        pre: SyncBufferSpec,
        post: SyncBufferSpec,
        removed: SyncMessageSpec,
    ) -> bool
        recommends pre.messages.len() > 0
    {
        post.messages.len() == pre.messages.len() - 1 &&
        post.total_size == pre.total_size - removed.size
    }

    // ========================================================================
    // Sync Session Invariant
    // ========================================================================

    /// Sync session invariant
    pub open spec fn session_invariant(session: SyncSessionSpec) -> bool {
        buffer_invariant(session.inbound_buffer) &&
        buffer_invariant(session.outbound_buffer)
    }

    /// Initial session
    pub open spec fn initial_session(peer_id: Seq<u8>, start_time: u64) -> SyncSessionSpec {
        SyncSessionSpec {
            peer_id,
            start_time,
            inbound_buffer: SyncBufferSpec {
                messages: Seq::empty(),
                total_size: 0,
            },
            outbound_buffer: SyncBufferSpec {
                messages: Seq::empty(),
                total_size: 0,
            },
            complete: false,
        }
    }

    /// Proof: Initial session satisfies invariant
    pub proof fn initial_session_valid(peer_id: Seq<u8>, start_time: u64)
        ensures session_invariant(initial_session(peer_id, start_time))
    {
        // Empty buffers satisfy invariant
    }

    // ========================================================================
    // Timeout Handling
    // ========================================================================

    /// Session has timed out
    pub open spec fn session_timed_out(
        session: SyncSessionSpec,
        current_time: u64,
    ) -> bool {
        !session.complete &&
        current_time >= session.start_time + SYNC_TIMEOUT_SECS
    }

    /// Proof: Completed sessions don't timeout
    pub proof fn completed_never_timeout(
        session: SyncSessionSpec,
        current_time: u64,
    )
        requires session.complete
        ensures !session_timed_out(session, current_time)
    {
        // complete => !timed_out
    }

    // ========================================================================
    // Sync Progress
    // ========================================================================

    /// Sync is making progress
    pub open spec fn sync_progressing(
        pre: SyncSessionSpec,
        post: SyncSessionSpec,
    ) -> bool {
        // Either more messages processed or sync completed
        post.complete ||
        post.inbound_buffer.messages.len() < pre.inbound_buffer.messages.len() ||
        post.outbound_buffer.messages.len() > pre.outbound_buffer.messages.len()
    }

    /// Sync completion
    pub open spec fn sync_complete_effect(
        pre: SyncSessionSpec,
        post: SyncSessionSpec,
    ) -> bool {
        !pre.complete &&
        post.complete &&
        post.peer_id == pre.peer_id
    }

    // ========================================================================
    // Message Type Semantics
    // ========================================================================

    /// Request message initiates sync
    pub open spec fn request_initiates_sync(msg: SyncMessageSpec) -> bool {
        match msg.msg_type {
            SyncMessageType::Request => true,
            _ => false,
        }
    }

    /// Response message contains data
    pub open spec fn response_contains_data(msg: SyncMessageSpec) -> bool {
        match msg.msg_type {
            SyncMessageType::Response => msg.size > 0,
            _ => false,
        }
    }

    /// Ack message is small
    pub open spec fn ack_is_small(msg: SyncMessageSpec) -> bool {
        match msg.msg_type {
            SyncMessageType::Ack => msg.size < 1024,  // < 1 KB
            _ => true,
        }
    }
}
