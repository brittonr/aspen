#![allow(
    tigerstyle::excessive_file_length,
    reason = "the journal keeps one canonical instance codec beside its memory and durability-port adapters"
)]

use super::super::*;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/journal/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/journal/p001/body.rs"));
