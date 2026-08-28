//! Focused strict-Octet surface for coordination delivery and its pure time dependency.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

pub mod fabric;
pub mod fabric_time {
    //! Compatibility re-export for the accepted product module path.
    pub use crate::fabric::time::*;
}
pub mod coordination_delivery;
