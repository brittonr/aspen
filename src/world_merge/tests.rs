include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p004/body.rs"));

mod migrations {
    use super::*;
    use crate::codec;

    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_merge/parts/tests/p005/body.rs"));
}
