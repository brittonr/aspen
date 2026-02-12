//! Built-in worker types for common distributed tasks.

#[cfg(feature = "blob")]
pub mod blob_processor;
pub mod echo;
#[cfg(feature = "blob")]
pub mod maintenance;
#[cfg(feature = "blob")]
pub mod replication;
#[cfg(feature = "shell-worker")]
pub mod shell_command;
#[cfg(feature = "blob")]
pub mod sql_query;

#[cfg(feature = "blob")]
pub use blob_processor::BlobProcessorWorker;
pub use echo::EchoWorker;
#[cfg(feature = "blob")]
pub use maintenance::MaintenanceWorker;
#[cfg(feature = "blob")]
pub use replication::ReplicationWorker;
#[cfg(feature = "shell-worker")]
pub use shell_command::INLINE_OUTPUT_THRESHOLD;
#[cfg(feature = "shell-worker")]
pub use shell_command::OutputRef;
#[cfg(feature = "shell-worker")]
pub use shell_command::ShellCommandWorker;
#[cfg(feature = "blob")]
pub use sql_query::SqlQueryWorker;
