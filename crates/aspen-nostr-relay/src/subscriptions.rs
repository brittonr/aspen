//! Subscription registry for real-time event fan-out.
//!
//! Tracks active subscriptions per connection and broadcasts new events
//! via a `tokio::sync::broadcast` channel. Each connection task receives
//! broadcast events and checks against its active filters locally.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use nostr::prelude::*;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::constants;

/// Unique connection identifier.
pub type ConnectionId = u64;

/// A single subscription held by a connection.
struct ActiveSubscription {
    filters: Vec<Filter>,
}

/// Per-connection subscription state.
struct ConnectionState {
    subscriptions: HashMap<SubscriptionId, ActiveSubscription>,
}

/// Subscription errors.
#[derive(Debug)]
pub enum SubscriptionError {
    /// Connection has too many subscriptions.
    TooManySubscriptions { max: u32, current: u32 },
}

impl fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManySubscriptions { max, current } => {
                write!(f, "too many subscriptions: {current}/{max}")
            }
        }
    }
}

impl std::error::Error for SubscriptionError {}

/// Registry tracking all active subscriptions across connections.
///
/// Each connection gets a `broadcast::Receiver` on connect. When a new event
/// is published, it's sent on the broadcast channel. Connection tasks receive
/// the event and filter locally against their active subscriptions.
pub struct SubscriptionRegistry {
    connections: RwLock<HashMap<ConnectionId, ConnectionState>>,
    event_tx: broadcast::Sender<Arc<Event>>,
    max_subscriptions_per_connection: u32,
}

impl SubscriptionRegistry {
    /// Create a new registry with the given per-connection subscription limit.
    pub fn new(max_subs_per_conn: u32) -> Self {
        let (event_tx, _) = broadcast::channel(constants::BROADCAST_CHANNEL_CAPACITY as usize);
        Self {
            connections: RwLock::new(HashMap::new()),
            event_tx,
            max_subscriptions_per_connection: max_subs_per_conn,
        }
    }

    /// Register a new connection.
    ///
    /// Returns a broadcast receiver for real-time event notifications.
    pub async fn add_connection(&self, conn_id: ConnectionId) -> broadcast::Receiver<Arc<Event>> {
        let mut conns = self.connections.write().await;
        conns.insert(conn_id, ConnectionState {
            subscriptions: HashMap::new(),
        });
        self.event_tx.subscribe()
    }

    /// Remove a connection and all its subscriptions.
    pub async fn remove_connection(&self, conn_id: ConnectionId) {
        let mut conns = self.connections.write().await;
        conns.remove(&conn_id);
    }

    /// Add or replace a subscription for a connection.
    ///
    /// If `sub_id` already exists, it's replaced (no limit check).
    /// If it's new and the connection is at the limit, returns an error.
    pub async fn subscribe(
        &self,
        conn_id: ConnectionId,
        sub_id: SubscriptionId,
        filters: Vec<Filter>,
    ) -> Result<(), SubscriptionError> {
        let mut conns = self.connections.write().await;
        let state = match conns.get_mut(&conn_id) {
            Some(s) => s,
            None => {
                // Connection not registered; create state on the fly
                conns.entry(conn_id).or_insert_with(|| ConnectionState {
                    subscriptions: HashMap::new(),
                })
            }
        };

        let is_replacement = state.subscriptions.contains_key(&sub_id);
        if !is_replacement {
            let current = state.subscriptions.len() as u32;
            if current >= self.max_subscriptions_per_connection {
                return Err(SubscriptionError::TooManySubscriptions {
                    max: self.max_subscriptions_per_connection,
                    current,
                });
            }
        }

        state.subscriptions.insert(sub_id, ActiveSubscription { filters });
        Ok(())
    }

    /// Remove a subscription from a connection.
    pub async fn unsubscribe(&self, conn_id: ConnectionId, sub_id: &SubscriptionId) {
        let mut conns = self.connections.write().await;
        if let Some(state) = conns.get_mut(&conn_id) {
            state.subscriptions.remove(sub_id);
        }
    }

    /// Broadcast an event to all subscribers.
    ///
    /// The event is sent on the broadcast channel; each connection task
    /// checks its own filters locally. Ignores send errors (no receivers).
    pub fn broadcast_event(&self, event: Arc<Event>) {
        let _ = self.event_tx.send(event);
    }

    /// Get the filters for a specific subscription.
    pub async fn get_filters(&self, conn_id: ConnectionId, sub_id: &SubscriptionId) -> Option<Vec<Filter>> {
        let conns = self.connections.read().await;
        conns.get(&conn_id).and_then(|s| s.subscriptions.get(sub_id)).map(|sub| sub.filters.clone())
    }

    /// Get all subscriptions for a connection with their filters.
    pub async fn get_connection_subscriptions(&self, conn_id: ConnectionId) -> Vec<(SubscriptionId, Vec<Filter>)> {
        let conns = self.connections.read().await;
        match conns.get(&conn_id) {
            Some(state) => state.subscriptions.iter().map(|(id, sub)| (id.clone(), sub.filters.clone())).collect(),
            None => Vec::new(),
        }
    }
}
