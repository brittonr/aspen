//! Focused strict-Octet compilation surface for world commits and mutable heads.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

pub mod world_commit;
pub mod world_head;
