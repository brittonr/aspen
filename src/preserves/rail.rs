include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p003/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p004/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p005/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p006/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p007/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/p008/body.rs"));

// The thin `std` bridge between the `molten-preserves-core` kernel and `preserves::IOValue`.
#[path = "bridge/refs.rs"]
mod identity_bridge;
#[path = "bridge/bounds.rs"]
pub mod packed_bounds;
#[path = "bridge/scan.rs"]
mod scan_bridge;
#[path = "bridge/encode.rs"]
mod stream_bridge;
#[path = "bridge/values.rs"]
mod value_bridge;

#[cfg(test)]
#[path = "bridge/tests/denials.rs"]
mod denial_tests;
#[cfg(test)]
#[path = "bridge/tests/differential.rs"]
mod differential_tests;
#[cfg(test)]
#[path = "bridge/tests/throughput.rs"]
mod encoding_throughput;
