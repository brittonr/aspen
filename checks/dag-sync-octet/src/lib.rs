//! Focused strict-Octet compilation surface for the DAG-sync functional core.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

#[path = "../../../crates/molten-core/src/dag_sync/mod.rs"]
pub mod dag_sync;
