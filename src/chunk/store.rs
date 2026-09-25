mod semantic_store {
    include!("parts/store/p000/body.rs");
    include!("parts/store/p001/body.rs");
    include!("parts/store/p002/body.rs");
    include!("parts/store/p003/body.rs");
    include!("parts/store/p004/body.rs");
    include!("parts/store/p005/body.rs");
    include!("parts/store/p006/body.rs");
    include!("parts/store/p007/body.rs");
    include!("parts/store/p008/body.rs");
    include!("parts/store/p009/body.rs");
    include!("parts/store/p010/body.rs");
    include!("parts/store/p011/body.rs");
    include!("parts/store/p012/body.rs");
    include!("parts/store/p013/body.rs");
    include!("parts/store/p014/body.rs");
    include!("parts/store/p015/body.rs");
    include!("parts/store/p016/body.rs");
    include!("gc.rs");

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;

        fn test_chunk_path(root: &Path, chunk_ref: &str) -> Result<std::path::PathBuf> {
            Ok(root.join("chunks").join(filename_for_ref(chunk_ref)?))
        }

        fn test_manifest_path(root: &Path, manifest_ref: &str) -> Result<std::path::PathBuf> {
            Ok(root.join("manifests").join(filename_for_ref(manifest_ref)?))
        }

        include!("parts/store/tests/m000/p000/body.rs");
        include!("parts/store/tests/m000/p001/body.rs");
        include!("parts/store/tests/m000/p002/body.rs");
        include!("parts/store/tests/m000/p003/body.rs");
    }
}

pub use semantic_store::*;
