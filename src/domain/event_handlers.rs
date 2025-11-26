//! Event Handler Abstraction
//!
//! This module provides a pluggable event handling system that allows
//! different handlers to process domain events.

use crate::domain::events::DomainEvent;
use crate::domain::errors::DomainResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for event handlers
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: &DomainEvent) -> DomainResult<()>;

    /// Get the name of this handler for logging
    fn name(&self) -> &str;

    /// Check if this handler can handle a specific event type
    fn handles(&self, _event: &DomainEvent) -> bool {
        true // By default, handle all events
    }

    /// Priority for this handler (higher = processed first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Event bus that manages and dispatches events to handlers
pub struct EventBus {
    handlers: RwLock<Vec<Arc<dyn EventHandler>>>,
    metrics: RwLock<EventMetrics>,
}

/// Metrics for event processing
#[derive(Debug, Default)]
struct EventMetrics {
    events_published: u64,
    events_handled: HashMap<String, u64>,
    handler_errors: HashMap<String, u64>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            metrics: RwLock::new(EventMetrics::default()),
        }
    }

    /// Register an event handler
    pub async fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
        // Sort by priority (highest first)
        handlers.sort_by_key(|h| std::cmp::Reverse(h.priority()));
    }

    /// Publish an event to all registered handlers
    pub async fn publish(&self, event: DomainEvent) -> DomainResult<()> {
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.events_published += 1;
        }

        let handlers = self.handlers.read().await;

        for handler in handlers.iter() {
            if handler.handles(&event) {
                let handler_name = handler.name();

                tracing::debug!(
                    event = ?event,
                    handler = handler_name,
                    "Dispatching event to handler"
                );

                match handler.handle(&event).await {
                    Ok(()) => {
                        let mut metrics = self.metrics.write().await;
                        *metrics.events_handled.entry(handler_name.to_string()).or_insert(0) += 1;
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            handler = handler_name,
                            event = ?event,
                            "Handler failed to process event"
                        );

                        let mut metrics = self.metrics.write().await;
                        *metrics.handler_errors.entry(handler_name.to_string()).or_insert(0) += 1;

                        // Continue processing with other handlers even if one fails
                        // This ensures resilience
                    }
                }
            }
        }

        Ok(())
    }

    /// Publish multiple events
    pub async fn publish_batch(&self, events: Vec<DomainEvent>) -> DomainResult<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Get the number of registered handlers
    pub async fn handler_count(&self) -> usize {
        self.handlers.read().await.len()
    }

    /// Get event processing metrics
    pub async fn metrics(&self) -> EventBusMetrics {
        let metrics = self.metrics.read().await;
        EventBusMetrics {
            events_published: metrics.events_published,
            events_handled: metrics.events_handled.clone(),
            handler_errors: metrics.handler_errors.clone(),
        }
    }

    /// Clear all handlers (useful for testing)
    pub async fn clear_handlers(&self) {
        self.handlers.write().await.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Public metrics structure
#[derive(Debug, Clone)]
pub struct EventBusMetrics {
    pub events_published: u64,
    pub events_handled: HashMap<String, u64>,
    pub handler_errors: HashMap<String, u64>,
}

// === Built-in Event Handlers ===

/// Logging event handler (current default behavior)
pub struct LoggingEventHandler {
    log_level: tracing::Level,
}

impl LoggingEventHandler {
    pub fn new() -> Self {
        Self {
            log_level: tracing::Level::INFO,
        }
    }

    pub fn with_level(log_level: tracing::Level) -> Self {
        Self { log_level }
    }
}

impl Default for LoggingEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventHandler for LoggingEventHandler {
    async fn handle(&self, event: &DomainEvent) -> DomainResult<()> {
        match self.log_level {
            tracing::Level::TRACE => tracing::trace!(?event, "Domain event occurred"),
            tracing::Level::DEBUG => tracing::debug!(?event, "Domain event occurred"),
            tracing::Level::INFO => tracing::info!(?event, "Domain event occurred"),
            tracing::Level::WARN => tracing::warn!(?event, "Domain event occurred"),
            tracing::Level::ERROR => tracing::error!(?event, "Domain event occurred"),
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "LoggingEventHandler"
    }

    fn priority(&self) -> u32 {
        50 // Lower priority - log after other handlers
    }
}

/// Metrics event handler for Prometheus/OpenTelemetry
#[cfg(feature = "metrics")]
pub struct MetricsEventHandler {
    // In a real implementation, this would hold a metrics recorder
}

#[cfg(feature = "metrics")]
#[async_trait]
impl EventHandler for MetricsEventHandler {
    async fn handle(&self, event: &DomainEvent) -> DomainResult<()> {
        // Record metrics based on event type
        match event {
            DomainEvent::JobSubmitted { .. } => {
                // metrics::counter!("jobs_submitted_total").increment(1);
            }
            DomainEvent::JobCompleted { .. } => {
                // metrics::counter!("jobs_completed_total").increment(1);
            }
            DomainEvent::JobFailed { .. } => {
                // metrics::counter!("jobs_failed_total").increment(1);
            }
            _ => {}
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "MetricsEventHandler"
    }

    fn priority(&self) -> u32 {
        100 // Higher priority - record metrics early
    }
}

/// Webhook event handler for external notifications
#[cfg(feature = "webhook-events")]
pub struct WebhookEventHandler {
    webhook_urls: Vec<String>,
    // TODO: Add reqwest to dependencies when implementing webhooks
    // client: reqwest::Client,
}

#[cfg(feature = "webhook-events")]
impl WebhookEventHandler {
    pub fn new(webhook_urls: Vec<String>) -> Self {
        Self {
            webhook_urls,
            // TODO: Add reqwest to dependencies when implementing webhooks
            // client: reqwest::Client::new(),
        }
    }
}

#[cfg(feature = "webhook-events")]
#[async_trait]
impl EventHandler for WebhookEventHandler {
    async fn handle(&self, event: &DomainEvent) -> DomainResult<()> {
        let _event_json = serde_json::to_value(event)
            .map_err(|e| crate::domain::errors::DomainError::Generic(e.to_string()))?;

        // TODO: Implement webhook calls when reqwest is added to dependencies
        // for url in &self.webhook_urls {
        //     // Fire and forget - don't wait for response
        //     let client = self.client.clone();
        //     let url = url.clone();
        //     let payload = event_json.clone();
        //
        //     tokio::spawn(async move {
        //         if let Err(e) = client.post(&url).json(&payload).send().await {
        //             tracing::warn!(
        //                 error = %e,
        //                 url = %url,
        //                 "Failed to send webhook notification"
        //             );
        //         }
        //     });
        // }

        tracing::warn!("Webhook support not implemented yet - webhooks configured but not sent");
        Ok(())
    }

    fn name(&self) -> &str {
        "WebhookEventHandler"
    }

    fn handles(&self, event: &DomainEvent) -> bool {
        // Only handle significant events for webhooks
        matches!(
            event,
            DomainEvent::JobCompleted { .. }
                | DomainEvent::JobFailed { .. }
                | DomainEvent::JobCancelled { .. }
        )
    }
}

/// Audit log handler for compliance
pub struct AuditEventHandler;

impl AuditEventHandler {
    pub fn new(_hiqlite: Arc<crate::hiqlite_service::HiqliteService>) -> Self {
        Self
    }
}

#[async_trait]
impl EventHandler for AuditEventHandler {
    async fn handle(&self, event: &DomainEvent) -> DomainResult<()> {
        // Store audit log in Hiqlite
        let event_json = serde_json::to_string(event)
            .map_err(|e| crate::domain::errors::DomainError::Generic(e.to_string()))?;

        let timestamp = chrono::Utc::now().timestamp();

        // This would need an audit_log table in the schema
        // For now, we'll just log it
        tracing::info!(
            event = %event_json,
            timestamp = timestamp,
            "Audit log entry"
        );

        Ok(())
    }

    fn name(&self) -> &str {
        "AuditEventHandler"
    }

    fn priority(&self) -> u32 {
        150 // High priority - audit before other processing
    }
}

/// Create a default event bus with standard handlers
pub fn create_default_event_bus() -> Arc<EventBus> {
    let bus = Arc::new(EventBus::new());

    // Add default handlers based on features
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        bus_clone
            .register_handler(Arc::new(LoggingEventHandler::new()))
            .await;

        #[cfg(feature = "metrics")]
        bus_clone
            .register_handler(Arc::new(MetricsEventHandler {}))
            .await;
    });

    bus
}

