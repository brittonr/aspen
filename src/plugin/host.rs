//! Plugin host lifecycle records and deterministic local lifecycle runner.
//!
//! The host in this first slice is deliberately receipt-first: install does
//! not grant runtime authority, activation requires separate permission and
//! executor evidence, and hostcalls are admitted only through declared refs.

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p003/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p004/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p005/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/plugin/parts/host/p006/body.rs"));
