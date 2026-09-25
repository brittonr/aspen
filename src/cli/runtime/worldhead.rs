#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-head CLI composes explicit operator records, capability namespaces, and fail-closed adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator command names retain the public world-head protocol spelling"
)]

use std::path::Path;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldhead/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldhead/p001/body.rs"));
