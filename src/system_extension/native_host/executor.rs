#![allow(
    tigerstyle::excessive_file_length,
    reason = "the one-shot callback transaction keeps intent, materialization, execution, publication, observation, and reconciliation ordering visible"
)]

use super::super::*;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/executor/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/executor/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_extension/native_host/parts/executor/p002/body.rs"));
