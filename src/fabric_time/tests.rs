mod replay;
#[path = "tests/adapters/mod.rs"]
mod scheduler_adapters;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_time/parts/tests/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_time/parts/tests/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_time/parts/tests/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_time/parts/tests/p003/body.rs"));
