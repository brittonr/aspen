#![allow(
    tigerstyle::excessive_file_length,
    reason = "the v2 wire codec keeps every canonical value field and strict decoder in one auditable protocol surface"
)]

use super::super::*;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/wire/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/wire/p001/body.rs"));
