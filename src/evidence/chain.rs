//! Scoped hash-chain evidence continuity.
//!
//! Chain links in this module are local evidence-continuity records. They are
//! deliberately scoped by `(scope, id, epoch)` and do not provide a global
//! actor-message order, fork-choice protocol, cryptocurrency ledger, or ambient
//! authority. A link's identity is only the Blake3 hash of its canonical
//! Preserves bytes; linking names payload refs without rewriting the payloads.

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p003/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p004/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p005/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p006/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p007/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence/parts/chain/p008/body.rs"));
