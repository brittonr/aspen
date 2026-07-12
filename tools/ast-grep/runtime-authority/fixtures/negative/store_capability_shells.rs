#![allow(dead_code)]

fn reviewed_store_bootstrap(root: &std::path::Path) -> crate::local_store::ChunkStoreRoot {
    crate::local_store::ChunkStoreRoot::open(root).expect("reviewed root bootstrap")
}

fn capability_child_read(root: &crate::local_store::ChunkStoreRoot, path: &crate::local_store::LocalStorePath) {
    let _ = root.root().read(path);
}
