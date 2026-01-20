//! CI pipeline trigger module.
//!
//! This module handles automatic pipeline triggering based on forge events,
//! specifically RefUpdate announcements from iroh-gossip.

mod service;

pub use service::CiTriggerHandler;
pub use service::TriggerService;
pub use service::TriggerServiceConfig;
