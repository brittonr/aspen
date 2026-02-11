//! Job queue RPC handlers for Aspen.
//!
//! This crate provides the job and worker handlers extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles job system operations:
//! - JobSubmit, JobGet, JobList, JobCancel: Job lifecycle management
//! - JobUpdateProgress, JobQueueStats: Job monitoring
//! - WorkerStatus, WorkerRegister, WorkerHeartbeat, WorkerDeregister: Worker management
//! - WorkerPollJobs, WorkerCompleteJob: Worker job execution

mod job_handler;
mod worker_handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use job_handler::JobHandler;
pub use worker_handler::WorkerHandler;
