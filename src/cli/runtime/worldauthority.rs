#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the world-authority CLI keeps operator DTO and denial-receipt ownership explicit"
)]

use std::path::Path;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldauthority/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldauthority/p001/body.rs"));
