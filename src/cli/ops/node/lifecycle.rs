#[path = "lifecycle/hosting.rs"]
mod hosting;
#[path = "lifecycle/stdio.rs"]
mod stdio;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/ops/node/parts/lifecycle/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/ops/node/parts/lifecycle/p001/body.rs"));
