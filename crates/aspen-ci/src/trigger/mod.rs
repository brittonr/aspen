//! CI pipeline trigger module.
//!
//! This module handles automatic pipeline triggering based on forge events,
//! specifically RefUpdate announcements from iroh-gossip.
//!
//! This module requires the `nickel` feature for loading Nickel configuration files.

#[cfg(feature = "nickel")]
mod service;

#[cfg(feature = "nickel")]
pub use service::CiTriggerHandler;
#[cfg(feature = "nickel")]
pub use service::ConfigFetcher;
#[cfg(feature = "nickel")]
pub use service::PipelineStarter;
#[cfg(feature = "nickel")]
pub use service::TriggerEvent;
#[cfg(feature = "nickel")]
pub use service::TriggerService;
#[cfg(feature = "nickel")]
pub use service::TriggerServiceConfig;
