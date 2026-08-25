#![allow(
    tigerstyle::excessive_file_length,
    reason = "the compatibility authority module keeps generated Preserves parsing parts stable during nominal admission"
)]

pub mod nominal;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/authority/parts/mod/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/authority/parts/mod/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/authority/parts/mod/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/authority/parts/mod/p003/body.rs"));
