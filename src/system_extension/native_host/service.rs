#![allow(
    tigerstyle::excessive_file_length,
    reason = "operator methods keep lifecycle, ingress, effect, journal, and status ordering in one explicit shell"
)]

use super::super::*;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/service/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/service/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/service/p002/body.rs"));
