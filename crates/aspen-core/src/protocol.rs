//! Sans-IO protocol infrastructure for distributed state machines.
//!
//! This module provides the foundational types and traits for building
//! distributed protocol state machines that perform zero I/O directly.
//! All side effects (sending messages, persisting state, raising alarms)
//! flow through a context trait that the shell layer implements.
//!
//! # Pattern
//!
//! Protocol logic lives in a struct (e.g., `GossipNode`, `TrustNode`) that
//! takes a `&mut impl ProtocolCtx` on every method call. The context trait
//! provides access to persistent state, connected peers, and outbound message
//! queues. The protocol struct is synchronous and deterministic — no async,
//! no system time, no randomness.
//!
//! Production code implements the context trait with real I/O (iroh networking,
//! redb storage). Test code implements it with an in-memory [`TestCtx`] that
//! records all side effects for assertion.
//!
//! # Reference
//!
//! Inspired by Oxide's trust-quorum sans-IO protocol:
//! `docs/reference/oxide-trust-quorum.md`
//!
//! See also: `docs/patterns/sans-io-protocol.md`

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::types::NodeId;

/// Sentinel: count overflow when usize does not fit in u32.
const COUNT_OVERFLOW_SENTINEL: u32 = u32::MAX;

// ============================================================================
// Envelope
// ============================================================================

/// A routable message wrapper for protocol communication.
///
/// The protocol state machine produces `Envelope`s; the shell layer drains
/// them and sends via the transport (e.g., Iroh QUIC). Recipients unwrap
/// the envelope and feed the inner message to their protocol state machine.
///
/// The type parameter `M` is the protocol-specific message type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<M> {
    /// Destination node.
    pub to: NodeId,
    /// Source node.
    pub from: NodeId,
    /// Protocol-specific message payload.
    pub msg: M,
}

impl<M> Envelope<M> {
    /// Create a new envelope.
    pub fn new(from: NodeId, to: NodeId, msg: M) -> Self {
        Self { to, from, msg }
    }
}

// ============================================================================
// Alarm
// ============================================================================

/// Protocol-raised alerts for conditions that require operator attention.
///
/// Alarms are not errors — the protocol continues operating. They signal
/// conditions like configuration mismatches, corruption detection, or
/// security events that should be logged and potentially acted upon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alarm {
    /// Two nodes disagree on the configuration for the same epoch.
    ConfigurationMismatch {
        /// Epoch where the mismatch was detected.
        epoch: u64,
        /// Node that reported a different configuration.
        from_node: NodeId,
        /// Description of what differs.
        detail: String,
    },
    /// A share or other cryptographic material failed validation.
    CorruptionDetected {
        /// Epoch of the corrupted material.
        epoch: u64,
        /// Node that sent the corrupted data.
        from_node: NodeId,
        /// What was corrupted.
        detail: String,
    },
    /// A node that should be expunged is still communicating.
    ExpungedNodeActive {
        /// The expunged node's ID.
        node_id: NodeId,
        /// The epoch at which it was expunged.
        expunged_at_epoch: u64,
    },
    /// Rack/cluster secret reconstruction failed.
    SecretReconstructionFailed {
        /// Epoch for which reconstruction was attempted.
        epoch: u64,
        /// Reason for failure.
        detail: String,
    },
    /// Rack/cluster secret decryption failed (encrypted chain).
    SecretDecryptionFailed {
        /// Epoch for which decryption was attempted.
        epoch: u64,
        /// Reason for failure.
        detail: String,
    },
}

// ============================================================================
// ProtocolCtx trait
// ============================================================================

/// Base context trait for sans-IO protocol state machines.
///
/// Every protocol state machine method takes `&mut impl ProtocolCtx<S, M>`
/// (or a protocol-specific sub-trait). The context provides:
///
/// - **Identity**: Which node this is (`platform_id`)
/// - **Persistent state**: Read and update durable state
/// - **Connections**: Which peers are currently connected
/// - **Alarms**: Report conditions requiring operator attention
///
/// Message sending is intentionally NOT on this base trait — each protocol
/// defines its own sending methods (unicast, broadcast, etc.) on a sub-trait.
///
/// # Type Parameters
///
/// - `S`: The persistent state type for this protocol
pub trait ProtocolCtx<S> {
    /// This node's identity.
    fn platform_id(&self) -> &NodeId;

    /// Read the current persistent state.
    fn persistent_state(&self) -> &S;

    /// Mutate persistent state via a closure.
    ///
    /// The closure receives a mutable reference to the state and returns
    /// `true` if the state was actually modified (triggering a disk write
    /// in production). Returns `false` if no change was made.
    ///
    /// In production, the shell flushes to storage after mutations.
    /// In tests, the [`TestCtx`] sets a dirty flag.
    fn update_persistent_state(&mut self, f: impl FnOnce(&mut S) -> bool);

    /// The set of currently connected peer node IDs.
    fn connected(&self) -> &BTreeSet<NodeId>;

    /// Record a new peer connection.
    fn add_connection(&mut self, peer: NodeId);

    /// Remove a peer connection.
    fn remove_connection(&mut self, peer: &NodeId);

    /// Raise an alarm for operator attention.
    ///
    /// Alarms are informational — the protocol continues running.
    fn raise_alarm(&mut self, alarm: Alarm);
}

// ============================================================================
// TestCtx
// ============================================================================

/// In-memory protocol context for deterministic testing.
///
/// Records all side effects (sent envelopes, state mutations, alarms) so
/// tests can assert on protocol behavior without real I/O.
///
/// # Type Parameters
///
/// - `S`: Persistent state type
/// - `M`: Protocol message type
#[derive(Debug)]
pub struct TestCtx<S, M> {
    /// This node's identity.
    id: NodeId,
    /// Persistent state (in-memory).
    state: S,
    /// Whether persistent state has been modified since last check.
    state_dirty: bool,
    /// Currently connected peers.
    connections: BTreeSet<NodeId>,
    /// Outbound message queue.
    envelopes: Vec<Envelope<M>>,
    /// Raised alarms.
    alarms: Vec<Alarm>,
}

impl<S, M> TestCtx<S, M> {
    /// Create a new test context with the given identity and initial state.
    pub fn new(id: NodeId, state: S) -> Self {
        Self {
            id,
            state,
            state_dirty: false,
            connections: BTreeSet::new(),
            envelopes: Vec::new(),
            alarms: Vec::new(),
        }
    }

    // ====================================================================
    // Envelope helpers
    // ====================================================================

    /// Number of envelopes in the outbound queue.
    pub fn num_envelopes(&self) -> u32 {
        u32::try_from(self.envelopes.len()).unwrap_or(COUNT_OVERFLOW_SENTINEL)
    }

    /// View all envelopes in the outbound queue.
    pub fn envelopes(&self) -> &[Envelope<M>] {
        &self.envelopes
    }

    /// Drain all envelopes from the outbound queue.
    pub fn drain_envelopes(&mut self) -> Vec<Envelope<M>> {
core::mem::take(&mut self.envelopes)
    }

    /// Get envelopes addressed to a specific node.
    pub fn envelopes_to(&self, to: NodeId) -> Vec<&Envelope<M>> {
        self.envelopes.iter().filter(|e| e.to == to).collect()
    }

    /// Push an envelope onto the outbound queue.
    ///
    /// Protocols call this (via a sub-trait `send` method) to queue messages.
    pub fn push_envelope(&mut self, envelope: Envelope<M>) {
        self.envelopes.push(envelope);
    }

    // ====================================================================
    // State helpers
    // ====================================================================

    /// Check if persistent state was modified and reset the flag.
    ///
    /// Returns `true` if `update_persistent_state` was called with a closure
    /// that returned `true` since the last check.
    pub fn persistent_state_change_check_and_reset(&mut self) -> bool {
        let was_dirty = self.state_dirty;
        self.state_dirty = false;
        was_dirty
    }

    // ====================================================================
    // Alarm helpers
    // ====================================================================

    /// Number of raised alarms.
    pub fn num_alarms(&self) -> u32 {
        u32::try_from(self.alarms.len()).unwrap_or(COUNT_OVERFLOW_SENTINEL)
    }

    /// Iterate over raised alarms.
    pub fn alarms(&self) -> &[Alarm] {
        &self.alarms
    }

    /// Drain all alarms.
    pub fn drain_alarms(&mut self) -> Vec<Alarm> {
core::mem::take(&mut self.alarms)
    }
}

impl<S, M> ProtocolCtx<S> for TestCtx<S, M> {
    fn platform_id(&self) -> &NodeId {
        &self.id
    }

    fn persistent_state(&self) -> &S {
        &self.state
    }

    fn update_persistent_state(&mut self, f: impl FnOnce(&mut S) -> bool) {
        let changed = f(&mut self.state);
        if changed {
            self.state_dirty = true;
        }
    }

    fn connected(&self) -> &BTreeSet<NodeId> {
        &self.connections
    }

    fn add_connection(&mut self, peer: NodeId) {
        self.connections.insert(peer);
    }

    fn remove_connection(&mut self, peer: &NodeId) {
        self.connections.remove(peer);
    }

    fn raise_alarm(&mut self, alarm: Alarm) {
        self.alarms.push(alarm);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal protocol message for testing.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestMsg {
        Ping,
        Pong,
    }

    // A minimal persistent state for testing.
    #[derive(Debug, Clone, Default, PartialEq)]
    struct TestState {
        counter: u64,
    }

    #[test]
    fn envelope_construction() {
        let env: Envelope<TestMsg> = Envelope::new(NodeId(1), NodeId(2), TestMsg::Ping);
        assert_eq!(env.from, NodeId(1));
        assert_eq!(env.to, NodeId(2));
        assert_eq!(env.msg, TestMsg::Ping);
    }

    #[test]
    fn test_ctx_identity() {
        let ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(42), TestState::default());
        assert_eq!(*ctx.platform_id(), NodeId(42));
    }

    #[test]
    fn test_ctx_persistent_state_read() {
        let ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState { counter: 7 });
        assert_eq!(ctx.persistent_state().counter, 7);
    }

    #[test]
    fn test_ctx_persistent_state_update_dirty_flag() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        // No changes yet.
        assert!(!ctx.persistent_state_change_check_and_reset());

        // Mutate — closure returns true.
        ctx.update_persistent_state(|s| {
            s.counter = 10;
            true
        });
        assert!(ctx.persistent_state_change_check_and_reset());

        // Flag was reset by the check.
        assert!(!ctx.persistent_state_change_check_and_reset());
    }

    #[test]
    fn test_ctx_persistent_state_update_no_change() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        // Closure returns false — no actual change.
        ctx.update_persistent_state(|_s| false);
        assert!(!ctx.persistent_state_change_check_and_reset());
    }

    #[test]
    fn test_ctx_connections() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        assert!(ctx.connected().is_empty());

        ctx.add_connection(NodeId(2));
        ctx.add_connection(NodeId(3));
        assert_eq!(ctx.connected().len(), 2);
        assert!(ctx.connected().contains(&NodeId(2)));
        assert!(ctx.connected().contains(&NodeId(3)));

        ctx.remove_connection(&NodeId(2));
        assert_eq!(ctx.connected().len(), 1);
        assert!(!ctx.connected().contains(&NodeId(2)));
    }

    #[test]
    fn test_ctx_connections_idempotent_add() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());
        ctx.add_connection(NodeId(5));
        ctx.add_connection(NodeId(5));
        assert_eq!(ctx.connected().len(), 1);
    }

    #[test]
    fn test_ctx_connections_remove_nonexistent() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());
        ctx.remove_connection(&NodeId(99)); // no panic
        assert!(ctx.connected().is_empty());
    }

    #[test]
    fn test_ctx_envelopes() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        assert_eq!(ctx.num_envelopes(), 0);

        ctx.push_envelope(Envelope::new(NodeId(1), NodeId(2), TestMsg::Ping));
        ctx.push_envelope(Envelope::new(NodeId(1), NodeId(3), TestMsg::Pong));
        assert_eq!(ctx.num_envelopes(), 2);

        // Filter by destination.
        let to_2 = ctx.envelopes_to(NodeId(2));
        assert_eq!(to_2.len(), 1);
        assert_eq!(to_2[0].msg, TestMsg::Ping);

        let to_3 = ctx.envelopes_to(NodeId(3));
        assert_eq!(to_3.len(), 1);
        assert_eq!(to_3[0].msg, TestMsg::Pong);

        // Drain.
        let drained = ctx.drain_envelopes();
        assert_eq!(drained.len(), 2);
        assert_eq!(ctx.num_envelopes(), 0);
    }

    #[test]
    fn test_ctx_alarms() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        assert_eq!(ctx.num_alarms(), 0);

        ctx.raise_alarm(Alarm::ConfigurationMismatch {
            epoch: 5,
            from_node: NodeId(2),
            detail: "members differ".into(),
        });
        assert_eq!(ctx.num_alarms(), 1);

        ctx.raise_alarm(Alarm::CorruptionDetected {
            epoch: 5,
            from_node: NodeId(3),
            detail: "bad share digest".into(),
        });
        assert_eq!(ctx.num_alarms(), 2);

        // Inspect.
        let alarms = ctx.alarms();
        assert!(matches!(alarms[0], Alarm::ConfigurationMismatch { epoch: 5, .. }));
        assert!(matches!(alarms[1], Alarm::CorruptionDetected { epoch: 5, .. }));

        // Drain.
        let drained = ctx.drain_alarms();
        assert_eq!(drained.len(), 2);
        assert_eq!(ctx.num_alarms(), 0);
    }

    #[test]
    fn test_ctx_full_workflow() {
        // Simulates a minimal protocol interaction using TestCtx.
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        // Connect a peer.
        ctx.add_connection(NodeId(2));

        // "Protocol" logic: if peer connected, send ping and bump counter.
        if ctx.connected().contains(&NodeId(2)) {
            let from = *ctx.platform_id();
            ctx.push_envelope(Envelope::new(from, NodeId(2), TestMsg::Ping));
            ctx.update_persistent_state(|s| {
                s.counter = s.counter.saturating_add(1);
                true
            });
        }

        // Assert side effects.
        assert_eq!(ctx.num_envelopes(), 1);
        assert_eq!(ctx.envelopes()[0].to, NodeId(2));
        assert_eq!(ctx.envelopes()[0].msg, TestMsg::Ping);
        assert!(ctx.persistent_state_change_check_and_reset());
        assert_eq!(ctx.persistent_state().counter, 1);
    }

    // ====================================================================
    // Multi-step protocol simulation
    // ====================================================================

    /// A tiny sans-IO "echo" protocol for testing the full pattern.
    struct EchoNode;

    impl EchoNode {
        fn on_connect(ctx: &mut TestCtx<TestState, TestMsg>, peer: NodeId) {
            ctx.add_connection(peer);
            let from = *ctx.platform_id();
            ctx.push_envelope(Envelope::new(from, peer, TestMsg::Ping));
        }

        fn on_disconnect(ctx: &mut TestCtx<TestState, TestMsg>, peer: &NodeId) {
            ctx.remove_connection(peer);
        }

        fn handle(ctx: &mut TestCtx<TestState, TestMsg>, from: NodeId, msg: TestMsg) {
            match msg {
                TestMsg::Ping => {
                    ctx.update_persistent_state(|s| {
                        s.counter = s.counter.saturating_add(1);
                        true
                    });
                    let me = *ctx.platform_id();
                    ctx.push_envelope(Envelope::new(me, from, TestMsg::Pong));
                }
                TestMsg::Pong => {
                    ctx.update_persistent_state(|s| {
                        s.counter = s.counter.saturating_add(1);
                        true
                    });
                }
            }
        }
    }

    #[test]
    fn test_multi_step_connect_ping_pong() {
        // Node 1
        let mut ctx1: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());
        // Node 2
        let mut ctx2: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(2), TestState::default());

        // Step 1: Node 1 connects to Node 2 → sends Ping.
        EchoNode::on_connect(&mut ctx1, NodeId(2));
        assert_eq!(ctx1.num_envelopes(), 1);
        assert_eq!(ctx1.envelopes()[0].msg, TestMsg::Ping);

        // Step 2: Deliver the Ping to Node 2.
        let env = ctx1.drain_envelopes().pop().unwrap();
        EchoNode::handle(&mut ctx2, env.from, env.msg);
        assert_eq!(ctx2.persistent_state().counter, 1);
        assert_eq!(ctx2.num_envelopes(), 1);
        assert_eq!(ctx2.envelopes()[0].msg, TestMsg::Pong);

        // Step 3: Deliver the Pong back to Node 1.
        let env = ctx2.drain_envelopes().pop().unwrap();
        EchoNode::handle(&mut ctx1, env.from, env.msg);
        assert_eq!(ctx1.persistent_state().counter, 1);

        // Step 4: Disconnect.
        EchoNode::on_disconnect(&mut ctx1, &NodeId(2));
        assert!(!ctx1.connected().contains(&NodeId(2)));
    }

    #[test]
    fn test_multiple_peers_concurrent_messages() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        // Connect 3 peers.
        EchoNode::on_connect(&mut ctx, NodeId(2));
        EchoNode::on_connect(&mut ctx, NodeId(3));
        EchoNode::on_connect(&mut ctx, NodeId(4));
        assert_eq!(ctx.connected().len(), 3);
        assert_eq!(ctx.num_envelopes(), 3); // One Ping to each peer.

        // All envelopes should be from Node 1.
        for env in ctx.envelopes() {
            assert_eq!(env.from, NodeId(1));
            assert_eq!(env.msg, TestMsg::Ping);
        }

        // Filter by destination.
        assert_eq!(ctx.envelopes_to(NodeId(3)).len(), 1);
        assert_eq!(ctx.envelopes_to(NodeId(99)).len(), 0);

        ctx.drain_envelopes();

        // Receive pings from all 3 peers.
        EchoNode::handle(&mut ctx, NodeId(2), TestMsg::Ping);
        EchoNode::handle(&mut ctx, NodeId(3), TestMsg::Ping);
        EchoNode::handle(&mut ctx, NodeId(4), TestMsg::Ping);

        assert_eq!(ctx.persistent_state().counter, 3);
        assert_eq!(ctx.num_envelopes(), 3); // One Pong to each.
    }

    #[test]
    fn test_alarm_during_protocol_flow() {
        let mut ctx: TestCtx<TestState, TestMsg> = TestCtx::new(NodeId(1), TestState::default());

        // Simulate receiving a message that triggers an alarm.
        ctx.raise_alarm(Alarm::CorruptionDetected {
            epoch: 1,
            from_node: NodeId(99),
            detail: "bad share".into(),
        });

        // Protocol continues — alarm doesn't stop processing.
        EchoNode::handle(&mut ctx, NodeId(2), TestMsg::Ping);
        assert_eq!(ctx.persistent_state().counter, 1);
        assert_eq!(ctx.num_envelopes(), 1);
        assert_eq!(ctx.num_alarms(), 1);
    }
}
