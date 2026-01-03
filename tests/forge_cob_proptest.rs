//! Property-based tests for COB (Collaborative Object) resolution.
//!
//! Tests fundamental properties of the COB system:
//! - Determinism: Same changes produce same state
//! - Idempotence: Duplicate operations don't cause issues
//! - State transitions: Close/Reopen work correctly
//! - Label semantics: Set-based operations

mod support;

use iroh::PublicKey;
use proptest::prelude::*;

use aspen::forge::cob::Issue;
use aspen::forge::{CobChange, CobOperation};

use support::proptest_generators::{
    arbitrary_cob_linear_dag, arbitrary_issue_child_operation, arbitrary_issue_title, arbitrary_label, arbitrary_labels,
};

/// Helper to create a test public key.
fn test_key() -> PublicKey {
    let secret = iroh::SecretKey::generate(&mut rand::rng());
    secret.public()
}

/// Compute a deterministic hash for a CobChange.
fn compute_change_hash(change: &CobChange) -> blake3::Hash {
    let bytes = postcard::to_allocvec(change).expect("serialization should not fail");
    blake3::hash(&bytes)
}

/// Apply a sequence of changes to an Issue and return the final state.
fn apply_changes(changes: &[CobChange]) -> Issue {
    let author = test_key();
    let mut issue = Issue::default();
    let mut timestamp = 1000u64;

    for change in changes {
        let hash = compute_change_hash(change);
        issue.apply_change(hash, &author, timestamp, &change.op);
        timestamp += 1000;
    }

    issue
}

proptest! {
    /// Property: Resolution is deterministic - same changes always produce same state.
    #[test]
    fn prop_resolution_deterministic(
        (_, changes) in arbitrary_cob_linear_dag(10)
    ) {
        let state1 = apply_changes(&changes);
        let state2 = apply_changes(&changes);

        prop_assert_eq!(&state1.title, &state2.title);
        prop_assert_eq!(&state1.body, &state2.body);
        prop_assert_eq!(state1.state, state2.state);
        prop_assert_eq!(&state1.labels, &state2.labels);
        prop_assert_eq!(state1.comments.len(), state2.comments.len());
    }

    /// Property: Close followed by Reopen results in Open state.
    #[test]
    fn prop_close_reopen_returns_open(
        title in arbitrary_issue_title(),
        labels in arbitrary_labels(),
    ) {
        let author = test_key();
        let _cob_id = blake3::hash(b"issue-test");
        let mut issue = Issue::default();

        // Create issue
        issue.apply_change(
            blake3::hash(b"create"),
            &author,
            1000,
            &CobOperation::CreateIssue {
                title,
                body: "Test body".to_string(),
                labels,
            },
        );

        prop_assert!(issue.state.is_open(), "New issue should be open");

        // Close issue
        issue.apply_change(
            blake3::hash(b"close"),
            &author,
            2000,
            &CobOperation::Close {
                reason: Some("Test close".to_string()),
            },
        );

        prop_assert!(issue.state.is_closed(), "Issue should be closed after Close");

        // Reopen issue
        issue.apply_change(blake3::hash(b"reopen"), &author, 3000, &CobOperation::Reopen);

        prop_assert!(issue.state.is_open(), "Issue should be open after Reopen");
    }

    /// Property: Adding the same label twice results in the label appearing once.
    #[test]
    fn prop_label_add_idempotent(
        label in arbitrary_label(),
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        // Add label twice
        issue.apply_change(
            blake3::hash(b"add1"),
            &author,
            1000,
            &CobOperation::AddLabel { label: label.clone() },
        );
        issue.apply_change(
            blake3::hash(b"add2"),
            &author,
            2000,
            &CobOperation::AddLabel { label: label.clone() },
        );

        // Label should appear exactly once
        prop_assert!(
            issue.labels.contains(&label),
            "Label should be present"
        );

        // Count occurrences (should be 1 since it's a HashSet)
        let count = issue.labels.iter().filter(|l| **l == label).count();
        prop_assert_eq!(count, 1, "Label should appear exactly once");
    }

    /// Property: Removing a non-existent label is a no-op.
    #[test]
    fn prop_label_remove_nonexistent_noop(
        label in arbitrary_label(),
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        let labels_before = issue.labels.len();

        // Remove non-existent label
        issue.apply_change(
            blake3::hash(b"remove"),
            &author,
            1000,
            &CobOperation::RemoveLabel { label },
        );

        // Labels should be unchanged
        prop_assert_eq!(issue.labels.len(), labels_before);
    }

    /// Property: Adding then removing a label leaves it absent.
    #[test]
    fn prop_label_add_remove_absent(
        label in arbitrary_label(),
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        // Add label
        issue.apply_change(
            blake3::hash(b"add"),
            &author,
            1000,
            &CobOperation::AddLabel { label: label.clone() },
        );

        prop_assert!(issue.labels.contains(&label));

        // Remove label
        issue.apply_change(
            blake3::hash(b"remove"),
            &author,
            2000,
            &CobOperation::RemoveLabel { label: label.clone() },
        );

        prop_assert!(!issue.labels.contains(&label));
    }

    /// Property: Comments accumulate in order.
    #[test]
    fn prop_comments_accumulate(
        num_comments in 1usize..10,
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        // Add N comments
        for i in 0..num_comments {
            let hash = blake3::hash(format!("comment-{}", i).as_bytes());
            issue.apply_change(
                hash,
                &author,
                (i as u64 + 1) * 1000,
                &CobOperation::Comment {
                    body: format!("Comment {}", i),
                },
            );
        }

        prop_assert_eq!(issue.comments.len(), num_comments);

        // Verify order (by checking bodies)
        for (i, comment) in issue.comments.iter().enumerate() {
            prop_assert_eq!(&comment.body, &format!("Comment {}", i));
        }
    }

    /// Property: EditTitle replaces the title.
    #[test]
    fn prop_edit_title_replaces(
        initial_title in arbitrary_issue_title(),
        new_title in arbitrary_issue_title(),
    ) {
        let author = test_key();
        let mut issue = Issue::new(initial_title.clone(), "".to_string(), vec![], 0);

        issue.apply_change(
            blake3::hash(b"edit"),
            &author,
            1000,
            &CobOperation::EditTitle { title: new_title.clone() },
        );

        prop_assert_eq!(issue.title, new_title);
    }

    /// Property: Multiple edits result in the last edit's value.
    #[test]
    fn prop_last_edit_wins(
        titles in prop::collection::vec(arbitrary_issue_title(), 2..5),
    ) {
        let author = test_key();
        let mut issue = Issue::new("Initial".to_string(), "".to_string(), vec![], 0);

        // Apply all title edits
        for (i, title) in titles.iter().enumerate() {
            let hash = blake3::hash(format!("edit-{}", i).as_bytes());
            issue.apply_change(
                hash,
                &author,
                (i as u64 + 1) * 1000,
                &CobOperation::EditTitle { title: title.clone() },
            );
        }

        // Last edit should win
        let last_title = titles.last().unwrap();
        prop_assert_eq!(&issue.title, last_title);
    }

    /// Property: Applying child operations doesn't crash.
    #[test]
    fn prop_child_operations_dont_crash(
        op in arbitrary_issue_child_operation(),
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "Test body".to_string(), vec![], 0);

        // Apply the operation - should not panic
        issue.apply_change(blake3::hash(b"op"), &author, 1000, &op);

        // Issue should still be in a valid state (updated_at should be set)
        prop_assert!(issue.updated_at_ms >= 1000);
    }

    /// Property: Updated timestamp is always >= created timestamp.
    #[test]
    fn prop_updated_at_monotonic(
        (_, changes) in arbitrary_cob_linear_dag(10)
    ) {
        let issue = apply_changes(&changes);

        prop_assert!(
            issue.updated_at_ms >= issue.created_at_ms,
            "updated_at should be >= created_at"
        );
    }

    /// Property: Reactions are accumulated per user.
    #[test]
    fn prop_reactions_per_user(
        emoji1 in prop_oneof![Just("👍"), Just("👎"), Just("❤️")],
        emoji2 in prop_oneof![Just("👍"), Just("👎"), Just("❤️")],
    ) {
        let author = test_key();
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        // Add first reaction
        issue.apply_change(
            blake3::hash(b"react1"),
            &author,
            1000,
            &CobOperation::React { emoji: emoji1.to_string() },
        );

        // Add second reaction (same user)
        issue.apply_change(
            blake3::hash(b"react2"),
            &author,
            2000,
            &CobOperation::React { emoji: emoji2.to_string() },
        );

        // Both reactions should be present (or just one if same emoji)
        if emoji1 == emoji2 {
            // Same emoji - user should appear once
            let reactors = issue.reactions.get(&emoji1.to_string());
            prop_assert!(reactors.is_some());
            prop_assert_eq!(reactors.unwrap().len(), 1);
        } else {
            // Different emojis - user should appear in both
            prop_assert!(issue.reactions.contains_key(&emoji1.to_string()));
            prop_assert!(issue.reactions.contains_key(&emoji2.to_string()));
        }
    }
}

// ============================================================================
// Non-proptest tests for edge cases
// ============================================================================

#[test]
fn test_empty_issue_defaults() {
    let issue = Issue::default();

    assert!(issue.title.is_empty());
    assert!(issue.body.is_empty());
    assert!(issue.state.is_open());
    assert!(issue.labels.is_empty());
    assert!(issue.comments.is_empty());
    assert!(issue.reactions.is_empty());
    assert!(issue.assignees.is_empty());
}

#[test]
fn test_patch_operations_ignored_on_issue() {
    let author = test_key();
    let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

    // Apply a patch operation - should be ignored
    issue.apply_change(
        blake3::hash(b"patch"),
        &author,
        1000,
        &CobOperation::CreatePatch {
            title: "Patch".to_string(),
            description: "".to_string(),
            base: [0u8; 32],
            head: [1u8; 32],
        },
    );

    // Issue should be unchanged except for timestamp
    assert_eq!(issue.title, "Test");
}
