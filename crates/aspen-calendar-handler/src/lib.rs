//! Calendar RPC handler for Aspen.
//!
//! Handles all calendar operations (calendars, events, RRULE expansion, free/busy,
//! iCal import/export) via the `CalendarStore` backed by the distributed KV store.

pub mod executor;

use std::sync::Arc;

pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::ServiceHandler;
pub use executor::CalendarServiceExecutor;

/// Factory for creating `CalendarHandler` instances.
///
/// Priority 360 (application layer).
pub struct CalendarHandlerFactory;

impl CalendarHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CalendarHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CalendarHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        let kv_store = ctx.kv_store.clone();
        let executor = Arc::new(CalendarServiceExecutor::new(kv_store));
        Some(Arc::new(ServiceHandler::new(executor)))
    }

    fn name(&self) -> &'static str {
        "CalendarHandler"
    }

    fn priority(&self) -> u32 {
        360
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("calendar")
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(CalendarHandlerFactory);

#[cfg(test)]
mod tests {
    use aspen_rpc_core::ServiceExecutor;

    use super::*;

    #[test]
    fn test_can_handle_all_calendar_variants() {
        let executor = CalendarServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        let handles = executor.handles();

        // All 13 calendar request variants should be handled.
        assert_eq!(handles.len(), 13);
        assert!(handles.contains(&"CalendarCreate"));
        assert!(handles.contains(&"CalendarDelete"));
        assert!(handles.contains(&"CalendarList"));
        assert!(handles.contains(&"CalendarGetEvent"));
        assert!(handles.contains(&"CalendarCreateEvent"));
        assert!(handles.contains(&"CalendarUpdateEvent"));
        assert!(handles.contains(&"CalendarDeleteEvent"));
        assert!(handles.contains(&"CalendarListEvents"));
        assert!(handles.contains(&"CalendarSearchEvents"));
        assert!(handles.contains(&"CalendarImportIcal"));
        assert!(handles.contains(&"CalendarExportIcal"));
        assert!(handles.contains(&"CalendarFreeBusy"));
        assert!(handles.contains(&"CalendarExpandRecurrence"));
    }

    #[test]
    fn test_service_name() {
        let executor = CalendarServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.service_name(), "calendar");
    }

    #[test]
    fn test_priority() {
        let executor = CalendarServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.priority(), 360);
    }

    #[test]
    fn test_app_id() {
        let executor = CalendarServiceExecutor::new(Arc::new(aspen_testing_core::DeterministicKeyValueStore::new()));
        assert_eq!(executor.app_id(), Some("calendar"));
    }
}
