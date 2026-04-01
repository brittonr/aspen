//! Integration test for the CI trigger pipeline tree walk.
//!
//! Verifies that `walk_tree_for_file` correctly finds `.aspen/ci.ncl`
//! in a commit tree created through the normal ForgeNode API — the same
//! path used when content is pushed via git-remote-aspen (including
//! federation-cloned content).

#[cfg(all(feature = "forge", feature = "blob"))]
mod tests {
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;
    use aspen_forge::ForgeNode;
    use aspen_forge::git::TreeEntry;
    use aspen_testing_core::DeterministicKeyValueStore;

    /// Minimal valid Nickel CI config.
    const MINIMAL_CI_NCL: &str = r#"{
  stages = [
    {
      name = "build",
      jobs = [
        {
          name = "hello",
          command = "echo hello",
        },
      ],
    },
  ],
}"#;

    /// CI config path components (matches CI_CONFIG_PATH in helpers.rs).
    const CI_CONFIG_PATH: &[&str] = &[".aspen", "ci.ncl"];

    async fn create_test_forge() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = DeterministicKeyValueStore::new();
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        ForgeNode::new(blobs, kv, secret_key)
    }

    /// Create a commit with `.aspen/ci.ncl` in its tree and return the commit hash.
    async fn create_commit_with_ci_config(
        forge: &ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore>,
        ci_config_content: &str,
    ) -> blake3::Hash {
        // Store the ci.ncl blob
        let ci_ncl_hash = forge.git.store_blob(ci_config_content.as_bytes().to_vec()).await.expect("store ci.ncl blob");

        // Create .aspen/ subtree with ci.ncl entry
        let aspen_tree_hash =
            forge.git.create_tree(&[TreeEntry::file("ci.ncl", ci_ncl_hash)]).await.expect("create .aspen tree");

        // Create a README blob for a more realistic tree
        let readme_hash = forge.git.store_blob(b"# Test Project".to_vec()).await.expect("store README blob");

        // Create root tree with .aspen/ directory and README
        let root_tree_hash = forge
            .git
            .create_tree(&[
                TreeEntry::directory(".aspen", aspen_tree_hash),
                TreeEntry::file("README.md", readme_hash),
            ])
            .await
            .expect("create root tree");

        // Create commit pointing to root tree
        forge
            .git
            .commit(root_tree_hash, vec![], "initial commit with CI config")
            .await
            .expect("create commit")
    }

    #[tokio::test]
    async fn test_walk_tree_finds_ci_config() {
        let forge = create_test_forge().await;
        let commit_hash = create_commit_with_ci_config(&forge, MINIMAL_CI_NCL).await;

        // Get the commit to find its tree
        let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");

        // Walk the tree — this is the same call handle_trigger_pipeline makes
        let result = aspen_ci_handler::handler::helpers::walk_tree_for_file(&forge.git, &commit.tree(), CI_CONFIG_PATH)
            .await
            .expect("walk_tree_for_file should not error");

        assert!(result.is_some(), ".aspen/ci.ncl should be found in the tree");

        let content = result.unwrap();
        let content_str = String::from_utf8(content).expect("valid utf8");
        assert_eq!(content_str, MINIMAL_CI_NCL);
    }

    #[tokio::test]
    async fn test_walk_tree_missing_aspen_dir() {
        let forge = create_test_forge().await;

        // Create a commit without .aspen/ directory
        let readme_hash = forge.git.store_blob(b"# No CI here".to_vec()).await.expect("store blob");

        let root_tree_hash =
            forge.git.create_tree(&[TreeEntry::file("README.md", readme_hash)]).await.expect("create root tree");

        let commit_hash = forge.git.commit(root_tree_hash, vec![], "no CI config").await.expect("create commit");

        let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");

        let result = aspen_ci_handler::handler::helpers::walk_tree_for_file(&forge.git, &commit.tree(), CI_CONFIG_PATH)
            .await
            .expect("should not error");

        assert!(result.is_none(), ".aspen/ci.ncl should NOT be found");
    }

    #[tokio::test]
    async fn test_walk_tree_aspen_dir_exists_but_no_ci_ncl() {
        let forge = create_test_forge().await;

        // Create .aspen/ with a different file (not ci.ncl)
        let other_blob = forge.git.store_blob(b"some other config".to_vec()).await.expect("store blob");

        let aspen_tree = forge
            .git
            .create_tree(&[TreeEntry::file("config.toml", other_blob)])
            .await
            .expect("create .aspen tree");

        let root_tree = forge
            .git
            .create_tree(&[TreeEntry::directory(".aspen", aspen_tree)])
            .await
            .expect("create root tree");

        let commit_hash = forge.git.commit(root_tree, vec![], "wrong file in .aspen").await.expect("create commit");

        let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");

        let result = aspen_ci_handler::handler::helpers::walk_tree_for_file(&forge.git, &commit.tree(), CI_CONFIG_PATH)
            .await
            .expect("should not error");

        assert!(result.is_none(), "ci.ncl should NOT be found when only config.toml exists");
    }

    #[tokio::test]
    async fn test_walk_tree_aspen_is_file_not_dir() {
        let forge = create_test_forge().await;

        // Create .aspen as a regular file (not a directory)
        let aspen_blob = forge.git.store_blob(b"I am a file, not a directory".to_vec()).await.expect("store blob");

        let root_tree =
            forge.git.create_tree(&[TreeEntry::file(".aspen", aspen_blob)]).await.expect("create root tree");

        let commit_hash = forge.git.commit(root_tree, vec![], ".aspen is a file").await.expect("create commit");

        let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");

        let result = aspen_ci_handler::handler::helpers::walk_tree_for_file(&forge.git, &commit.tree(), CI_CONFIG_PATH)
            .await
            .expect("should not error");

        assert!(result.is_none(), "should return None when .aspen is a file, not directory");
    }
}
