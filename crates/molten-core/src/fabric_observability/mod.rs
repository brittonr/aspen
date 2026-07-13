//! Pure, bounded fabric observability and read-only integrity laws.
//!
//! This module receives in-memory facts only. It does not read clocks, files,
//! stores, exporters, processes, networks, environment state, or adapter
//! handles, and it never performs repair or other mutation.

mod adapter;
mod aggregation;
mod health;
mod integrity;
mod model;
mod validation;

pub use adapter::*;
pub use aggregation::*;
pub use health::*;
pub use integrity::*;
pub use model::*;
pub use validation::*;

#[cfg(test)]
mod tests;
