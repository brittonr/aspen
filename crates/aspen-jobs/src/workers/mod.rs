//! Built-in worker types for common distributed tasks.

pub mod blob_processor;
pub mod echo;
pub mod maintenance;
pub mod replication;
#[cfg(feature = "shell-worker")]
pub mod shell_command;
pub mod sql_query;

pub use blob_processor::BlobProcessorWorker;
pub use echo::EchoWorker;
pub use maintenance::MaintenanceWorker;
pub use replication::ReplicationWorker;
#[cfg(feature = "shell-worker")]
pub use shell_command::ShellCommandWorker;
pub use sql_query::SqlQueryWorker;
