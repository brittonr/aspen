#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

#[path = "worldcommit/mod.rs"]
pub mod world_commit;
pub mod world_snapshot;
