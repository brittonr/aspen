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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::DomainEvent;

    #[tokio::test]
    async fn test_in_memory_publisher_stores_events() {
        // Arrange
        let publisher = InMemoryEventPublisher::new();
        let event = DomainEvent::JobSubmitted {
            job_id: "job-123".to_string(),
            url: "https://example.com".to_string(),
            timestamp: 1000,
        };

        // Act
        publisher.publish(event.clone()).await.unwrap();

        // Assert
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job_id(), "job-123");
    }

    #[tokio::test]
    async fn test_in_memory_publisher_filters_by_job() {
        // Arrange
        let publisher = InMemoryEventPublisher::new();

        publisher.publish(DomainEvent::JobSubmitted {
            job_id: "job-1".to_string(),
            url: "https://example.com".to_string(),
            timestamp: 1000,
        }).await.unwrap();

        publisher.publish(DomainEvent::JobSubmitted {
            job_id: "job-2".to_string(),
            url: "https://example.org".to_string(),
            timestamp: 1001,
        }).await.unwrap();

        publisher.publish(DomainEvent::JobCompleted {
            job_id: "job-1".to_string(),
            worker_id: Some("worker-1".to_string()),
            duration_ms: 5000,
            timestamp: 1002,
        }).await.unwrap();

        // Act
        let job1_events = publisher.get_events_for_job("job-1").await;

        // Assert
        assert_eq!(job1_events.len(), 2); // Two events for job-1
    }

    #[tokio::test]
    async fn test_in_memory_publisher_clear() {
        // Arrange
        let publisher = InMemoryEventPublisher::new();

        publisher.publish(DomainEvent::JobSubmitted {
            job_id: "job-1".to_string(),
            url: "https://example.com".to_string(),
            timestamp: 1000,
        }).await.unwrap();

        assert_eq!(publisher.count().await, 1);

        // Act
        publisher.clear().await;

        // Assert
        assert_eq!(publisher.count().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_publisher_batch() {
        // Arrange
        let publisher = InMemoryEventPublisher::new();
        let events = vec![
            DomainEvent::JobSubmitted {
                job_id: "job-1".to_string(),
                url: "https://example.com".to_string(),
                timestamp: 1000,
            },
            DomainEvent::JobSubmitted {
                job_id: "job-2".to_string(),
                url: "https://example.org".to_string(),
                timestamp: 1001,
            },
        ];

        // Act
        publisher.publish_batch(events).await.unwrap();

        // Assert
        assert_eq!(publisher.count().await, 2);
    }

    #[tokio::test]
    async fn test_logging_publisher_succeeds() {
        // Arrange
        let publisher = LoggingEventPublisher::new();
        let event = DomainEvent::JobSubmitted {
            job_id: "job-123".to_string(),
            url: "https://example.com".to_string(),
            timestamp: 1000,
        };

        // Act & Assert - should not panic
        publisher.publish(event).await.unwrap();
    }

    #[tokio::test]
    async fn test_noop_publisher_succeeds() {
        // Arrange
        let publisher = NoOpEventPublisher::new();
        let event = DomainEvent::JobSubmitted {
            job_id: "job-123".to_string(),
            url: "https://example.com".to_string(),
            timestamp: 1000,
        };

        // Act & Assert - should not panic
        publisher.publish(event).await.unwrap();
    }
}
