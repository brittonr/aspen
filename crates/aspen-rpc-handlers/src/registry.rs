//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.
//!
//! # Plugin Architecture
//!
//! All handlers self-register using the `inventory` crate via `HandlerFactory`.
//! The registry collects all registered factories, checks runtime preconditions,
//! and creates handler instances sorted by priority.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::collect_handler_factories;
use tracing::debug;
use tracing::trace;

use crate::context::ClientProtocolContext;

/// Registry of request handlers.
///
/// Dispatches requests to the appropriate handler based on request type.
/// Handlers are tried in order; the first handler that `can_handle` the
/// request will process it.
///
/// # Tiger Style
///
/// - Bounded number of handlers (statically known)
/// - O(n) dispatch where n is small and fixed
/// - Clear error when no handler matches
/// - Clone-friendly for sharing across async tasks
#[derive(Clone)]
pub struct HandlerRegistry {
    handlers: Arc<Vec<Arc<dyn RequestHandler>>>,
}

impl HandlerRegistry {
    /// Create a new handler registry with all domain handlers.
    ///
    /// All handlers self-register via `HandlerFactory` + `submit_handler_factory!`
    /// and are collected via `inventory`. Each factory's `create()` method checks
    /// runtime preconditions (e.g., whether required services are available).
    ///
    /// Handlers are sorted by priority (lower = checked first) before being stored.
    pub fn new(ctx: &ClientProtocolContext) -> Self {
        // Collect handlers with their priorities for sorting
        let mut handlers_with_priority: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();

        // All handlers self-register via submit_handler_factory! and are collected here.
        // Each factory checks runtime preconditions in create() and returns None if
        // the handler should not be registered (e.g., required service unavailable).
        let plugin_factories = collect_handler_factories();
        for factory in plugin_factories {
            match factory.create(ctx) {
                Some(handler) => {
                    trace!(factory = factory.name(), priority = factory.priority(), "handler registered via inventory");
                    handlers_with_priority.push((handler, factory.priority()));
                }
                None => {
                    trace!(factory = factory.name(), "handler factory skipped (preconditions not met)");
                }
            }
        }

        // Sort by priority (lower = checked first)
        handlers_with_priority.sort_by_key(|(_, priority)| *priority);

        // Extract handlers in sorted order
        let handlers: Vec<Arc<dyn RequestHandler>> = handlers_with_priority.into_iter().map(|(h, _)| h).collect();

        debug!(handler_count = handlers.len(), "handler registry initialized");

        Self {
            handlers: Arc::new(handlers),
        }
    }

    /// Dispatch a request to the appropriate handler.
    ///
    /// # Errors
    ///
    /// Returns an error if no handler can process the request.
    pub async fn dispatch(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        for handler in self.handlers.iter() {
            if handler.can_handle(&request) {
                debug!(
                    handler = handler.name(),
                    request = ?std::mem::discriminant(&request),
                    "dispatching request to handler"
                );
                return handler.handle(request, ctx).await;
            }
        }

        // No handler found - this shouldn't happen if all request types are covered
        Err(anyhow::anyhow!("no handler found for request type: {:?}", std::mem::discriminant(&request)))
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry").field("handler_count", &self.handlers.len()).finish()
    }
}
