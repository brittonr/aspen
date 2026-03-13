//! Lightweight transport wrapper for patchbay test nodes.
//!
//! Implements `NetworkTransport` with an iroh endpoint directly,
//! without the full `IrohEndpointManager` machinery (router, relay server,
//! gossip, etc.).

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;

/// Minimal iroh transport for patchbay test nodes.
///
/// Wraps a bare iroh endpoint — no protocol router, no relay server,
/// no gossip. Sufficient for Raft RPC over QUIC.
pub struct TestTransport {
    endpoint: Endpoint,
    addr: EndpointAddr,
    secret_key: SecretKey,
}

impl TestTransport {
    /// Create a new test transport.
    pub fn new(endpoint: Endpoint, addr: EndpointAddr, secret_key: SecretKey) -> Self {
        Self {
            endpoint,
            addr,
            secret_key,
        }
    }
}

impl fmt::Debug for TestTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestTransport").field("addr", &self.addr).finish()
    }
}

#[async_trait]
impl aspen_core::NetworkTransport for TestTransport {
    type Endpoint = Endpoint;
    type Address = EndpointAddr;
    type SecretKey = SecretKey;
    type Gossip = iroh_gossip::net::Gossip;

    fn node_addr(&self) -> &EndpointAddr {
        &self.addr
    }

    fn node_id_string(&self) -> String {
        self.endpoint.id().to_string()
    }

    fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn gossip(&self) -> Option<&Arc<iroh_gossip::net::Gossip>> {
        None
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}
