//! Event publisher implementations
//!
//! Provides concrete implementations of the EventPublisher trait for various
//! use cases (logging, in-memory storage, external systems).

#![allow(dead_code)] // Event publishers for future testing and production use

use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::events::{DomainEvent, EventPublisher};

/// In-memory event publisher for testing and development
///
/// Stores all published events in memory. Useful for:
/// - Unit testing
/// - Development/debugging
/// - Demonstrating event-driven patterns
///
/// **Note:** Events are lost on restart. For production, use a durable publisher.
#[derive(Clone)]
pub struct InMemoryEventPublisher {
    events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create a new in-memory event publisher
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all published events (for testing)
    pub async fn get_events(&self) -> Vec<DomainEvent> {
        self.events.lock().await.clone()
    }

    /// Get events for a specific job (for testing)
    pub async fn get_events_for_job(&self, job_id: &str) -> Vec<DomainEvent> {
        self.events
            .lock()
            .await
            .iter()
            .filter(|e| e.job_id() == job_id)
            .cloned()
            .collect()
    }

    /// Clear all events (for testing)
    pub async fn clear(&self) {
        self.events.lock().await.clear();
    }

    /// Get event count (for testing)
    pub async fn count(&self) -> usize {
        self.events.lock().await.len()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<()> {
        tracing::debug!(event = ?event, "Event published: {}", event.description());
        self.events.lock().await.push(event);
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<()> {
        let mut store = self.events.lock().await;
        for event in events {
            tracing::debug!(event = ?event, "Event published: {}", event.description());
            store.push(event);
        }
        Ok(())
    }
}

/// Logging event publisher
///
/// Publishes events to the logging system. Useful for:
/// - Production audit trails
/// - Debugging in production
/// - Correlation with application logs
#[derive(Clone)]
pub struct LoggingEventPublisher;

impl LoggingEventPublisher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventPublisher for LoggingEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<()> {
        tracing::info!(
            event_type = ?event,
            job_id = %event.job_id(),
            timestamp = event.timestamp(),
            "Domain event: {}",
            event.description()
        );
        Ok(())
    }
}

/// No-op event publisher (does nothing)
///
/// Useful when you want to disable event publishing without changing code.
#[derive(Clone)]
pub struct NoOpEventPublisher;

impl NoOpEventPublisher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventPublisher for NoOpEventPublisher {
    async fn publish(&self, _event: DomainEvent) -> Result<()> {
        // Do nothing
        Ok(())
    }
}

