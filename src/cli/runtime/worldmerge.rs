#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-merge CLI composes explicit commits, root maps, profiles, and bounded operator output"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator commands retain the public world-merge protocol spelling"
)]

use std::collections::BTreeMap;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldmerge/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/runtime/parts/worldmerge/p001/body.rs"));
