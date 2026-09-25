#![feature(register_tool)]
#![register_tool(tigerstyle)]

#[path = "nativesystemextension/support.rs"]
mod support;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parts/nativesystemextension/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parts/nativesystemextension/p001/body.rs"));
