//! Focused strict-Octet compilation surface for world distribution.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

pub mod content_replication;
pub mod dag_sync;
pub mod world_commit;
pub mod world_distribution;
pub mod world_head;
