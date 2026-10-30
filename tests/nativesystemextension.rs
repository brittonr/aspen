#![feature(register_tool)]
#![register_tool(tigerstyle)]

#[path = "nativesystemextension/hosting.rs"]
mod hosting;
#[cfg(target_os = "linux")]
#[path = "nativesystemextension/recovery.rs"]
mod recovery;
#[path = "nativesystemextension/support.rs"]
mod support;
#[path = "src/test/support.rs"]
mod test_support;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parts/nativesystemextension/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parts/nativesystemextension/p001/body.rs"));
