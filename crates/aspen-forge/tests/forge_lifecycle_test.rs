//! Integration tests for aspen-forge lifecycle operations.
//!
//! Tests the complete lifecycle of repositories, issues, patches, discussions,
//! and forks to ensure end-to-end functionality works correctly.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_forge::ForgeNode;
use aspen_forge::git::GitMergeStrategy;
use aspen_forge::git::TreeEntry;
use aspen_testing::DeterministicKeyValueStore;

async fn create_test_node() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    ForgeNode::new(blobs, kv, secret_key)
}

/// Create a test repository with an initial commit
async fn create_test_repo(
    node: &ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore>,
    name: &str,
) -> (aspen_forge::identity::RepoIdentity, blake3::Hash) {
    let repo = node.create_repo(name, vec![node.public_key()], 1).await.expect("create repo");
    let repo_id = repo.repo_id();

    // Create initial commit with a README
    let readme_blob = node
        .git
        .store_blob(b"# Initial README\n\nThis is the initial commit.".to_vec())
        .await
        .expect("store blob");
    let tree = node.git.create_tree(&[TreeEntry::file("README.md", readme_blob)]).await.expect("create tree");
    let commit = node.git.commit(tree, vec![], "Initial commit").await.expect("create commit");
    node.refs.set(&repo_id, "heads/main", commit).await.expect("set ref");

    (repo, commit)
}

#[tokio::test]
async fn test_repo_lifecycle() {
    let node = create_test_node().await;

    // Create repo
    let repo = node.create_repo("test-repo", vec![node.public_key()], 1).await.expect("create repo");
    let repo_id = repo.repo_id();

    // Store a blob
    let blob_content = b"Hello World";
    let blob_hash = node.git.store_blob(blob_content.to_vec()).await.expect("store blob");

    // Create tree with one file entry
    let tree_hash = node.git.create_tree(&[TreeEntry::file("README.md", blob_hash)]).await.expect("create tree");

    // Commit with empty parents and message
    let commit_hash = node.git.commit(tree_hash, vec![], "Initial commit").await.expect("create commit");

    // Set heads/main to the commit
    node.refs.set(&repo_id, "heads/main", commit_hash).await.expect("set ref");

    // Assert get_head returns that commit
    let head = node.get_head(&repo_id).await.expect("get head");
    assert_eq!(head, Some(commit_hash), "get_head should return the commit we set");

    // Get the commit object and verify its tree contains the blob
    let commit_obj = node.git.get_commit(&commit_hash).await.expect("get commit");
    let tree_obj = node.git.get_tree(&commit_obj.tree()).await.expect("get tree");

    assert_eq!(tree_obj.entries.len(), 1, "tree should have one entry");
    let entry = &tree_obj.entries[0];
    assert_eq!(entry.name, "README.md");
    assert_eq!(entry.hash(), blob_hash);

    // Verify we can read the blob content back
    let blob_content_back = node.git.get_blob(&blob_hash).await.expect("get blob");
    assert_eq!(blob_content_back, blob_content.to_vec());
}

#[tokio::test]
async fn test_issue_lifecycle() {
    let node = create_test_node().await;
    let (repo, _commit) = create_test_repo(&node, "issue-test").await;
    let repo_id = repo.repo_id();

    // Create issue with labels
    let issue_id = node
        .cobs
        .create_issue(&repo_id, "Bug: crash on startup", "It crashes", vec!["bug".to_string(), "critical".to_string()])
        .await
        .expect("create issue");

    // Add 2 comments
    let _comment1 = node
        .cobs
        .add_comment(&repo_id, &issue_id, "I can reproduce this issue")
        .await
        .expect("add comment 1");
    let _comment2 = node.cobs.add_comment(&repo_id, &issue_id, "Here's a potential fix").await.expect("add comment 2");

    // Close issue with reason
    node.cobs.close_issue(&repo_id, &issue_id, Some("Fixed".to_string())).await.expect("close issue");

    // Reopen issue
    node.cobs.reopen_issue(&repo_id, &issue_id).await.expect("reopen issue");

    // Resolve issue and verify state
    let issue = node.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve issue");

    assert!(issue.state.is_open(), "issue should be open after reopening");
    assert_eq!(issue.comments.len(), 2, "should have 2 comments");
    assert!(issue.labels.contains("bug"), "should have bug label");
    assert!(issue.labels.contains("critical"), "should have critical label");
    assert_eq!(issue.title, "Bug: crash on startup");
    assert_eq!(issue.body, "It crashes");
}

#[tokio::test]
async fn test_patch_lifecycle() {
    let node = create_test_node().await;
    let (repo, c0) = create_test_repo(&node, "patch-test").await;
    let repo_id = repo.repo_id();

    // Create a new blob, tree, commit c1 (with parent c0)
    let new_blob = node.git.store_blob(b"New file content".to_vec()).await.expect("store new blob");
    let new_tree = node
        .git
        .create_tree(&[
            TreeEntry::file(
                "README.md",
                node.git
                    .store_blob(b"# Initial README\n\nThis is the initial commit.".to_vec())
                    .await
                    .expect("store readme"),
            ),
            TreeEntry::file("new_file.txt", new_blob),
        ])
        .await
        .expect("create new tree");
    let c1 = node.git.commit(new_tree, vec![c0], "Add new file").await.expect("create patch commit");

    // Create patch with base=c0, head=c1
    let patch_id = node
        .cobs
        .create_patch(&repo_id, "Add new file", "This adds a new file", c0, c1)
        .await
        .expect("create patch");

    // Approve the patch
    node.cobs
        .approve_patch(&repo_id, &patch_id, c1, Some("Looks good!".to_string()))
        .await
        .expect("approve patch");

    // Merge the patch (strategy: MergeCommit)
    let merge_hash = node
        .merge_patch(&repo_id, &patch_id, GitMergeStrategy::MergeCommit, None)
        .await
        .expect("merge patch");

    // Verify get_head returns the merge commit
    let head = node.get_head(&repo_id).await.expect("get head");
    assert_eq!(head, Some(merge_hash), "head should point to merge commit");

    // Verify resolve_patch shows state Merged
    let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.expect("resolve patch");
    assert!(patch.state.is_merged(), "patch should be merged");
    assert_eq!(patch.state.merged_commit(), Some(*merge_hash.as_bytes()));
}

#[tokio::test]
async fn test_discussion_lifecycle() {
    let node = create_test_node().await;
    let (repo, _commit) = create_test_repo(&node, "discussion-test").await;
    let repo_id = repo.repo_id();

    // Create discussion
    let discussion_id = node
        .cobs
        .create_discussion(&repo_id, "RFC: API v2", "Let's discuss the new API design", vec!["rfc".to_string()])
        .await
        .expect("create discussion");

    // Add 2 top-level replies
    let reply1_id = node.cobs.add_reply(&repo_id, &discussion_id, "Great idea!", None).await.expect("add reply 1");
    let reply2_id = node
        .cobs
        .add_reply(&repo_id, &discussion_id, "I have some concerns", None)
        .await
        .expect("add reply 2");

    // Add 1 threaded reply (parent_reply = change hash of first reply)
    let _reply3_id = node
        .cobs
        .add_reply(&repo_id, &discussion_id, "Can you elaborate on the concerns?", Some(*reply2_id.as_bytes()))
        .await
        .expect("add threaded reply");

    // Resolve a thread (using first reply)
    node.cobs
        .resolve_thread(&repo_id, &discussion_id, *reply1_id.as_bytes())
        .await
        .expect("resolve thread");

    // Close discussion with reason
    node.cobs
        .close_discussion(&repo_id, &discussion_id, Some("Consensus reached".to_string()))
        .await
        .expect("close discussion");

    // Resolve discussion and verify
    let discussion = node.cobs.resolve_discussion(&repo_id, &discussion_id).await.expect("resolve discussion");

    assert!(discussion.state.is_closed(), "discussion should be closed");
    assert_eq!(discussion.replies.len(), 3, "should have 3 replies");
    assert_eq!(discussion.resolved_threads.len(), 1, "should have 1 resolved thread");
    assert_eq!(discussion.title, "RFC: API v2");
    assert!(discussion.labels.contains("rfc"));
}

#[tokio::test]
async fn test_fork_lifecycle() {
    let node = create_test_node().await;
    let (upstream_repo, mut last_commit) = create_test_repo(&node, "upstream").await;
    let upstream_id = upstream_repo.repo_id();

    // Add 2 more commits to main (total 3 commits)
    for i in 1..=2 {
        let blob = node.git.store_blob(format!("Content of file{}.txt", i).into_bytes()).await.expect("store blob");
        let readme_blob = node
            .git
            .store_blob(b"# Initial README\n\nThis is the initial commit.".to_vec())
            .await
            .expect("store readme");

        let mut entries = vec![
            TreeEntry::file("README.md", readme_blob),
            TreeEntry::file(&format!("file{}.txt", i), blob),
        ];

        // Include previous files
        for j in 1..i {
            let prev_blob = node
                .git
                .store_blob(format!("Content of file{}.txt", j).into_bytes())
                .await
                .expect("store prev blob");
            entries.push(TreeEntry::file(&format!("file{}.txt", j), prev_blob));
        }

        let tree = node.git.create_tree(&entries).await.expect("create tree");
        let commit = node
            .git
            .commit(tree, vec![last_commit], &format!("Add file{}.txt", i))
            .await
            .expect("create commit");
        node.refs.set(&upstream_id, "heads/main", commit).await.expect("set ref");
        last_commit = commit;
    }

    // Fork it as "my-fork"
    let fork = node.fork_repo(&upstream_id, "my-fork", vec![node.public_key()], 1).await.expect("fork repo");
    let fork_id = fork.repo_id();

    // Verify fork's get_head matches upstream's get_head
    let upstream_head = node.get_head(&upstream_id).await.expect("get upstream head");
    let fork_head = node.get_head(&fork_id).await.expect("get fork head");
    assert_eq!(fork_head, upstream_head, "fork head should match upstream head");

    // Verify fork identity has fork_info pointing to upstream
    assert!(fork.is_fork(), "should be a fork");
    assert_eq!(fork.fork_info.as_ref().unwrap().upstream_repo_id, upstream_id);
    assert!(fork.fork_info.as_ref().unwrap().upstream_cluster.is_none(), "should be same cluster");
}

#[tokio::test]
async fn test_multi_cob() {
    let node = create_test_node().await;
    let identity = node.create_repo("multi-cob", vec![node.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Init commit
    let blob = node.git.store_blob(b"content".to_vec()).await.expect("store blob");
    let tree = node.git.create_tree(&[TreeEntry::file("file.txt", blob)]).await.expect("create tree");
    let c0 = node.git.commit(tree, vec![], "init").await.expect("commit");
    node.refs.set(&repo_id, "heads/main", c0).await.expect("set ref");

    // Create an issue
    let issue_id = node
        .cobs
        .create_issue(&repo_id, "Bug", "description", vec!["bug".to_string()])
        .await
        .expect("create issue");

    // Create a patch
    let blob2 = node.git.store_blob(b"new content".to_vec()).await.expect("store blob");
    let tree2 = node.git.create_tree(&[TreeEntry::file("file.txt", blob2)]).await.expect("create tree");
    let c1 = node.git.commit(tree2, vec![c0], "change").await.expect("commit");
    let patch_id = node.cobs.create_patch(&repo_id, "Fix bug", "Fixes the bug", c0, c1).await.expect("create patch");

    // Create a discussion
    let discussion_id = node
        .cobs
        .create_discussion(&repo_id, "RFC: Architecture", "Let's discuss", vec!["rfc".to_string()])
        .await
        .expect("create discussion");

    // Resolve all three independently
    let issue = node.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve issue");
    assert_eq!(issue.title, "Bug");
    assert!(issue.state.is_open());

    let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.expect("resolve patch");
    assert_eq!(patch.title, "Fix bug");
    assert!(patch.state.is_open());

    let discussion = node.cobs.resolve_discussion(&repo_id, &discussion_id).await.expect("resolve discussion");
    assert_eq!(discussion.title, "RFC: Architecture");
    assert!(discussion.state.is_open());
    assert!(discussion.labels.contains("rfc"));
}
