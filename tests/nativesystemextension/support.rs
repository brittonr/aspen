#![allow(
    tigerstyle::excessive_file_length,
    reason = "the integration cohort keeps all exact profile, manifest, authority, and executable fixtures together"
)]

use molten::fabric::*;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/nativesystemextension/parts/support/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/nativesystemextension/parts/support/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/nativesystemextension/parts/support/p002/body.rs"));
