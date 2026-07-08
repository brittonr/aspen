//! Coordination control-plane services.
//!
//! This slice keeps coordination mutations in an explicit control-plane state
//! machine. Ordinary actor/data-plane messages do not call into this module;
//! callers must present canonical coordination requests and receive receipts
//! with Raft/control-registry evidence before dataspace facts are reflected.

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p003/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p004/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p005/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p006/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p008/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/coordination/parts/mod/p007/body.rs"));
