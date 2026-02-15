//! Builder for configuring and creating an AspenDnsClient.

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

use tokio::sync::RwLock;
use tokio::sync::broadcast;

use super::super::ticket::DnsClientTicket;
use super::super::types::SyncStatus;
use super::AspenDnsClient;

/// Builder for configuring and creating an AspenDnsClient.
#[derive(Default)]
pub struct DnsClientBuilder {
    ticket: Option<DnsClientTicket>,
}

impl DnsClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connection ticket.
    pub fn ticket(mut self, ticket: DnsClientTicket) -> Self {
        self.ticket = Some(ticket);
        self
    }

    /// Build the DNS client.
    ///
    /// Note: This creates the client but does not start sync.
    /// Use the returned client with your sync infrastructure.
    pub fn build(self) -> AspenDnsClient {
        let (event_tx, _) = broadcast::channel(256);

        AspenDnsClient {
            cache: RwLock::new(HashMap::new()),
            event_tx,
            status: RwLock::new(SyncStatus::Disconnected),
            ticket: self.ticket,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            sync_entries: AtomicU64::new(0),
            last_sync: RwLock::new(None),
        }
    }
}
