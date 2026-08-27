//! Capability-relative executable-extent consumer shell.
//!
//! Molten owns current runtime admission and detached consumer receipts. The
//! shared component owns layout, compatibility, W^X, and Linux mapping mechanics.

mod orchestrator;
mod ports;
pub mod producer;
mod record;
mod store;

pub use orchestrator::ConsumeError;
pub use orchestrator::ConsumeOutcome;
pub use orchestrator::ConsumerRequest;
pub use orchestrator::MappedBundle;
pub use orchestrator::consume_bundle;
pub use ports::AdmissionPortError;
pub use ports::BundleSource;
pub use ports::CurrentAdmissionPort;
pub use ports::SourceError;
pub use record::ConsumerMappingObservation;
pub use record::ConsumerReceipt;
pub use store::CapabilityBundleSource;

/// Immutable executable-extent source repository.
pub const EXECUTABLE_EXTENT_REPOSITORY: &str = "rad://z37R1bP1kHcELs89RNbQRaqbCVKxB";
/// Archived compatibility revision used for durable adoption.
pub const EXECUTABLE_EXTENT_REVISION: &str = "025d9636f0161777710dac37b3c210ca0ad9483f";
/// Immutable Mantle producer repository.
pub const MANTLE_PRODUCER_REPOSITORY: &str = "rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb";
/// Reviewed Mantle producer revision.
pub const MANTLE_PRODUCER_REVISION: &str = "2c636b1b25353a1b0befa5af48dc68615cd686dd";
