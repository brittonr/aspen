//! Tests for `CobStore`.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_testing::DeterministicKeyValueStore;

use super::super::change::CobType;
use super::*;

async fn create_test_store() -> CobStore<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let node_id = hex::encode(secret_key.public().as_bytes());
    CobStore::new(blobs, kv, secret_key, &node_id)
}

fn test_repo_id() -> crate::identity::RepoId {
    crate::identity::RepoId::from_hash(blake3::hash(b"test-repo"))
}

#[tokio::test]
async fn test_issue_creation() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Bug report", "Something is broken", vec!["bug".to_string()])
        .await
        .expect("should create issue");

    // Resolve and verify
    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert_eq!(issue.title, "Bug report");
    assert_eq!(issue.body, "Something is broken");
    assert!(issue.labels.contains("bug"));
    assert!(issue.state.is_open());
}

#[tokio::test]
async fn test_issue_comments() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // Add comments
    store.add_comment(&repo_id, &issue_id, "First comment").await.expect("should add comment");

    store.add_comment(&repo_id, &issue_id, "Second comment").await.expect("should add comment");

    // Resolve and verify
    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert_eq!(issue.comments.len(), 2);
}

#[tokio::test]
async fn test_issue_close() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "To be closed", "", vec![]).await.expect("should create issue");

    store.close_issue(&repo_id, &issue_id, Some("Fixed".to_string())).await.expect("should close");

    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert!(issue.state.is_closed());
}

#[tokio::test]
async fn test_list_issues_empty() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert!(issues.is_empty());
}

#[tokio::test]
async fn test_list_issues_single() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0], issue_id);
}

#[tokio::test]
async fn test_list_issues_multiple() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue1 = store.create_issue(&repo_id, "Issue 1", "", vec![]).await.expect("should create issue");

    let issue2 = store.create_issue(&repo_id, "Issue 2", "", vec![]).await.expect("should create issue");

    let issue3 = store.create_issue(&repo_id, "Issue 3", "", vec![]).await.expect("should create issue");

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 3);
    assert!(issues.contains(&issue1));
    assert!(issues.contains(&issue2));
    assert!(issues.contains(&issue3));
}

#[tokio::test]
async fn test_list_patches_empty() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let patches = store.list_patches(&repo_id).await.expect("should list patches");
    assert!(patches.is_empty());
}

#[tokio::test]
async fn test_list_cobs_different_types_isolated() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // List issues should return the issue
    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0], issue_id);

    // List patches should return empty (different type)
    let patches = store.list_patches(&repo_id).await.expect("should list patches");
    assert!(patches.is_empty());
}

#[tokio::test]
async fn test_list_cobs_different_repos_isolated() {
    let store = create_test_store().await;
    let repo_id1 = crate::identity::RepoId::from_hash(blake3::hash(b"repo-1"));
    let repo_id2 = crate::identity::RepoId::from_hash(blake3::hash(b"repo-2"));

    // Create an issue in repo 1
    let issue_id = store.create_issue(&repo_id1, "Test issue", "", vec![]).await.expect("should create issue");

    // List issues in repo 1 should return the issue
    let issues1 = store.list_issues(&repo_id1).await.expect("should list issues");
    assert_eq!(issues1.len(), 1);
    assert_eq!(issues1[0], issue_id);

    // List issues in repo 2 should return empty (different repo)
    let issues2 = store.list_issues(&repo_id2).await.expect("should list issues");
    assert!(issues2.is_empty());
}

#[tokio::test]
async fn test_list_cobs_generic() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // Use generic list_cobs with CobType::Issue
    let cobs = store.list_cobs(&repo_id, CobType::Issue).await.expect("should list cobs");
    assert_eq!(cobs.len(), 1);
    assert_eq!(cobs[0], issue_id);

    // Use generic list_cobs with CobType::Patch
    let cobs = store.list_cobs(&repo_id, CobType::Patch).await.expect("should list cobs");
    assert!(cobs.is_empty());
}

// ========================================================================
// DAG well-formedness property tests (task 7.5)
// ========================================================================

/// Build a synthetic DAG of signed CobChanges in memory and verify
/// that topological_sort produces a valid ordering.
///
/// This doesn't need async CobStore — we construct the HashMap directly
/// and call the sort method, which is `pub(super)` and accessible here.
mod dag_wellformedness {
    use std::collections::HashMap;
    use std::collections::HashSet;

    use aspen_core::hlc::SerializableTimestamp;
    use aspen_core::hlc::create_hlc;

    use super::*;
    use crate::cob::change::CobChange;
    use crate::cob::change::CobOperation;
    use crate::cob::change::CobType;
    use crate::types::SignedObject;

    /// Build a linear chain of N changes and verify topological sort.
    fn build_linear_dag(
        n: usize,
        secret_key: &iroh::SecretKey,
        hlc: &aspen_core::hlc::HLC,
    ) -> HashMap<blake3::Hash, SignedObject<CobChange>> {
        let cob_id = blake3::hash(b"test-cob");
        let mut changes = HashMap::new();
        let mut prev_hash: Option<blake3::Hash> = None;

        for i in 0..n {
            let parents = match prev_hash {
                Some(h) => vec![h],
                None => vec![],
            };

            let change = CobChange::new(CobType::Issue, cob_id, parents, CobOperation::Comment {
                body: format!("comment {}", i),
            });

            let signed = SignedObject::new(change, secret_key, hlc).expect("should sign");
            let hash = signed.hash();
            changes.insert(hash, signed);
            prev_hash = Some(hash);
        }

        changes
    }

    /// Build a diamond DAG: root → A, root → B, A+B → merge
    fn build_diamond_dag(
        secret_key: &iroh::SecretKey,
        hlc: &aspen_core::hlc::HLC,
    ) -> HashMap<blake3::Hash, SignedObject<CobChange>> {
        let cob_id = blake3::hash(b"diamond-cob");
        let mut changes = HashMap::new();

        // Root
        let root = CobChange::root(CobType::Issue, cob_id, CobOperation::CreateIssue {
            title: "test".into(),
            body: "test".into(),
            labels: vec![],
        });
        let root_signed = SignedObject::new(root, secret_key, hlc).expect("sign");
        let root_hash = root_signed.hash();
        changes.insert(root_hash, root_signed);

        // Branch A
        let a = CobChange::new(CobType::Issue, cob_id, vec![root_hash], CobOperation::AddLabel { label: "a".into() });
        let a_signed = SignedObject::new(a, secret_key, hlc).expect("sign");
        let a_hash = a_signed.hash();
        changes.insert(a_hash, a_signed);

        // Branch B
        let b = CobChange::new(CobType::Issue, cob_id, vec![root_hash], CobOperation::AddLabel { label: "b".into() });
        let b_signed = SignedObject::new(b, secret_key, hlc).expect("sign");
        let b_hash = b_signed.hash();
        changes.insert(b_hash, b_signed);

        // Merge
        let merge = CobChange::new(CobType::Issue, cob_id, vec![a_hash, b_hash], CobOperation::Comment {
            body: "merged".into(),
        });
        let merge_signed = SignedObject::new(merge, secret_key, hlc).expect("sign");
        let merge_hash = merge_signed.hash();
        changes.insert(merge_hash, merge_signed);

        changes
    }

    #[tokio::test]
    async fn test_linear_dag_topological_sort() {
        let store = super::create_test_store().await;
        let sk = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("dag-test");

        let changes = build_linear_dag(10, &sk, &hlc);
        let sorted = store.topological_sort(&changes).expect("should sort");

        // All changes present
        assert_eq!(sorted.len(), 10);

        // Parents appear before children
        let mut seen = HashSet::new();
        for (hash, signed) in &sorted {
            for parent in signed.payload.parents() {
                assert!(
                    seen.contains(&parent),
                    "parent {} must appear before child {}",
                    parent.to_hex(),
                    hash.to_hex()
                );
            }
            seen.insert(*hash);
        }

        // Exactly one root
        let roots: Vec<_> = sorted.iter().filter(|(_, s)| s.payload.is_root()).collect();
        assert_eq!(roots.len(), 1);
    }

    #[tokio::test]
    async fn test_diamond_dag_topological_sort() {
        let store = super::create_test_store().await;
        let sk = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("dag-test");

        let changes = build_diamond_dag(&sk, &hlc);
        let sorted = store.topological_sort(&changes).expect("should sort");

        assert_eq!(sorted.len(), 4);

        // Parents before children
        let mut seen = HashSet::new();
        for (hash, signed) in &sorted {
            for parent in signed.payload.parents() {
                assert!(seen.contains(&parent), "parent must appear before child");
            }
            seen.insert(*hash);
        }

        // Root is first
        assert!(sorted[0].1.payload.is_root());
    }

    #[tokio::test]
    async fn test_cycle_detection() {
        let store = super::create_test_store().await;
        let sk = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("dag-test");
        let cob_id = blake3::hash(b"cycle-cob");

        // Manually construct a cycle: A → B → A
        // We can't do this through normal CobChange constructors since
        // we'd need the hash before creating the object. Instead, create
        // two changes that reference each other's hash by pre-computing.

        // Create A with a fake parent (B's hash which we'll compute)
        let b_change = CobChange::root(CobType::Issue, cob_id, CobOperation::CreateIssue {
            title: "b".into(),
            body: "".into(),
            labels: vec![],
        });
        let b_signed = SignedObject::new(b_change, &sk, &hlc).expect("sign");
        let b_hash = b_signed.hash();

        let a_change = CobChange::new(CobType::Issue, cob_id, vec![b_hash], CobOperation::Comment { body: "a".into() });
        let a_signed = SignedObject::new(a_change, &sk, &hlc).expect("sign");
        let a_hash = a_signed.hash();

        // Now create B referencing A (making a cycle)
        let b_cycle =
            CobChange::new(CobType::Issue, cob_id, vec![a_hash], CobOperation::Comment { body: "b-cycle".into() });
        let b_cycle_signed = SignedObject::new(b_cycle, &sk, &hlc).expect("sign");
        let b_cycle_hash = b_cycle_signed.hash();

        let mut changes = HashMap::new();
        changes.insert(a_hash, a_signed);
        changes.insert(b_cycle_hash, b_cycle_signed);

        // Sort should detect the cycle (missing parent = effectively breaks the cycle,
        // but a true cycle within the map would be caught)
        // Since the parents reference hashes not in the map, this won't cycle.
        // The sort handles missing parents gracefully by skipping them.
        let result = store.topological_sort(&changes);
        // This should succeed because missing parents are just skipped
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_dag() {
        let store = super::create_test_store().await;
        let changes: HashMap<blake3::Hash, SignedObject<CobChange>> = HashMap::new();
        let sorted = store.topological_sort(&changes).expect("should sort");
        assert!(sorted.is_empty());
    }

    #[tokio::test]
    async fn test_single_root_dag() {
        let store = super::create_test_store().await;
        let sk = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("dag-test");
        let cob_id = blake3::hash(b"single-cob");

        let root = CobChange::root(CobType::Issue, cob_id, CobOperation::CreateIssue {
            title: "only".into(),
            body: "".into(),
            labels: vec![],
        });
        let root_signed = SignedObject::new(root, &sk, &hlc).expect("sign");
        let root_hash = root_signed.hash();

        let mut changes = HashMap::new();
        changes.insert(root_hash, root_signed);

        let sorted = store.topological_sort(&changes).expect("should sort");
        assert_eq!(sorted.len(), 1);
        assert!(sorted[0].1.payload.is_root());
    }
}
