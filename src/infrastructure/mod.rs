//! Infrastructure layer types and utilities
//!
//! This module contains infrastructure-specific types that are separate
//! from domain models. These types deal with operational concerns like
//! job assignment, worker tracking, and resource management.

pub mod job_assignment;

pub use job_assignment::{AssignmentStrategy, JobAssignment};
