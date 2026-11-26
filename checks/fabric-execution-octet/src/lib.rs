//! Focused strict-Octet compilation surface for the bounded execution core.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod fabric;
pub mod fabric_execution;
