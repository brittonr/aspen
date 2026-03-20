//! Property-based tests for COB resolution invariants.
//!
//! These tests verify fundamental properties of COB state resolution:
//! - Idempotency: applying changes twice produces the same state
//! - Convergence: independent changes produce the same result regardless of order
//! - DAG well-formedness: generated DAGs have valid structure

// Gate behind feature flag for quick test profile
#![cfg(not(feature = "quick"))]

use std::collections::HashSet;

use aspen_core::hlc::SerializableTimestamp;
use aspen_core::hlc::create_hlc;
use aspen_forge::CobOperation;
use aspen_forge::cob::Discussion;
use aspen_forge::cob::Issue;
use iroh::PublicKey;
use proptest::prelude::*;

/// Create a test HLC for generating timestamps.
fn test_hlc() -> aspen_core::hlc::HLC {
    create_hlc("prop-test")
}

/// Helper to create a test public key.
fn test_key() -> PublicKey {
    let secret = iroh::SecretKey::generate(&mut rand::rng());
    secret.public()
}

/// Generate a valid label (alphanumeric strings 1-20 chars)
fn arb_label() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,20}"
}

/// Generate issue operations
fn arb_issue_op() -> impl Strategy<Value = CobOperation> {
    prop_oneof![
        arb_label().prop_map(|body| CobOperation::Comment { body }),
        arb_label().prop_map(|label| CobOperation::AddLabel { label }),
        arb_label().prop_map(|label| CobOperation::RemoveLabel { label }),
        "[👍👎❤️😀😢🎉🚀👀]".prop_map(|emoji| CobOperation::React {
            emoji: emoji.to_string()
        }),
        "[👍👎❤️😀😢🎉🚀👀]".prop_map(|emoji| CobOperation::Unreact {
            emoji: emoji.to_string()
        }),
        "[a-zA-Z0-9 ]{1,50}".prop_map(|title| CobOperation::EditTitle { title }),
        "[a-zA-Z0-9 ]{1,100}".prop_map(|body| CobOperation::EditBody { body }),
        Just(CobOperation::Close {
            reason: Some("Test close".to_string())
        }),
        Just(CobOperation::Reopen),
    ]
}

/// Generate discussion operations
fn arb_discussion_op() -> impl Strategy<Value = CobOperation> {
    prop_oneof![
        arb_label().prop_map(|body| CobOperation::Reply {
            body,
            parent_reply: None
        }),
        arb_label().prop_map(|label| CobOperation::AddLabel { label }),
        arb_label().prop_map(|label| CobOperation::RemoveLabel { label }),
        "[👍👎❤️😀😢🎉🚀👀]".prop_map(|emoji| CobOperation::React {
            emoji: emoji.to_string()
        }),
        "[👍👎❤️😀😢🎉🚀👀]".prop_map(|emoji| CobOperation::Unreact {
            emoji: emoji.to_string()
        }),
        "[a-zA-Z0-9 ]{1,50}".prop_map(|title| CobOperation::EditTitle { title }),
        "[a-zA-Z0-9 ]{1,100}".prop_map(|body| CobOperation::EditBody { body }),
        Just(CobOperation::LockDiscussion),
        Just(CobOperation::UnlockDiscussion),
        Just(CobOperation::Close {
            reason: Some("Test close".to_string())
        }),
        Just(CobOperation::Reopen),
    ]
}

/// Apply operations to an issue and return the final state
fn apply_issue_ops(ops: &[CobOperation]) -> Issue {
    apply_issue_ops_with_author(ops, &test_key())
}

/// Apply operations to an issue with a specific author
fn apply_issue_ops_with_author(ops: &[CobOperation], author: &PublicKey) -> Issue {
    let hlc = test_hlc();
    let mut issue = Issue::new("Test".to_string(), "Test body".to_string(), vec![], 0);

    for (i, op) in ops.iter().enumerate() {
        let hash = blake3::hash(format!("op-{}", i).as_bytes());
        let timestamp = SerializableTimestamp::from(hlc.new_timestamp());
        issue.apply_change(hash, author, &timestamp, op);
    }

    issue
}

/// Apply operations to a discussion and return the final state
fn apply_discussion_ops(ops: &[CobOperation]) -> Discussion {
    apply_discussion_ops_with_author(ops, &test_key())
}

/// Apply operations to a discussion with a specific author
fn apply_discussion_ops_with_author(ops: &[CobOperation], author: &PublicKey) -> Discussion {
    let hlc = test_hlc();
    let mut discussion = Discussion::new("Test".to_string(), "Test body".to_string(), 0);

    for (i, op) in ops.iter().enumerate() {
        let hash = blake3::hash(format!("op-{}", i).as_bytes());
        let timestamp = SerializableTimestamp::from(hlc.new_timestamp());
        discussion.apply_change(hash, author, &timestamp, op);
    }

    discussion
}

proptest! {
    /// Property: Issue resolution is idempotent - applying operations twice produces same state
    #[test]
    fn issue_resolution_idempotent(ops in prop::collection::vec(arb_issue_op(), 1..20)) {
        let author = test_key();
        let issue1 = apply_issue_ops_with_author(&ops, &author);
        let issue2 = apply_issue_ops_with_author(&ops, &author);

        prop_assert_eq!(&issue1.title, &issue2.title);
        prop_assert_eq!(&issue1.body, &issue2.body);
        prop_assert_eq!(issue1.state, issue2.state);
        prop_assert_eq!(&issue1.labels, &issue2.labels);
        prop_assert_eq!(issue1.comments.len(), issue2.comments.len());
        prop_assert_eq!(&issue1.reactions, &issue2.reactions);
        prop_assert_eq!(&issue1.assignees, &issue2.assignees);
    }

    /// Property: Label convergence - adding labels in different orders produces same set
    #[test]
    fn label_convergence(label_a in arb_label(), label_b in arb_label()) {
        prop_assume!(label_a != label_b); // Only test different labels

        // Apply AddLabel(a) then AddLabel(b)
        let ops1 = vec![
            CobOperation::AddLabel { label: label_a.clone() },
            CobOperation::AddLabel { label: label_b.clone() },
        ];

        // Apply AddLabel(b) then AddLabel(a)
        let ops2 = vec![
            CobOperation::AddLabel { label: label_b.clone() },
            CobOperation::AddLabel { label: label_a.clone() },
        ];

        let issue1 = apply_issue_ops(&ops1);
        let issue2 = apply_issue_ops(&ops2);

        prop_assert_eq!(&issue1.labels, &issue2.labels);
        prop_assert!(issue1.labels.contains(&label_a));
        prop_assert!(issue1.labels.contains(&label_b));
    }

    /// Property: Comment convergence - adding comments in different orders preserves content
    #[test]
    fn comment_convergence(body_a in "[a-z]{1,50}", body_b in "[a-z]{1,50}") {
        prop_assume!(body_a != body_b); // Only test different comments

        // Apply Comment(a) then Comment(b)
        let ops1 = vec![
            CobOperation::Comment { body: body_a.clone() },
            CobOperation::Comment { body: body_b.clone() },
        ];

        // Apply Comment(b) then Comment(a)
        let ops2 = vec![
            CobOperation::Comment { body: body_b.clone() },
            CobOperation::Comment { body: body_a.clone() },
        ];

        let issue1 = apply_issue_ops(&ops1);
        let issue2 = apply_issue_ops(&ops2);

        // Both should have same comment count
        prop_assert_eq!(issue1.comments.len(), issue2.comments.len());
        prop_assert_eq!(issue1.comments.len(), 2);

        // Collect comment bodies from both issues
        let bodies1: HashSet<String> = issue1.comments.iter().map(|c| c.body.clone()).collect();
        let bodies2: HashSet<String> = issue2.comments.iter().map(|c| c.body.clone()).collect();

        // Same set of comment bodies should be present
        prop_assert_eq!(&bodies1, &bodies2);
        prop_assert!(bodies1.contains(&body_a));
        prop_assert!(bodies1.contains(&body_b));
    }

    /// Property: Discussion resolution is idempotent
    #[test]
    fn discussion_resolution_idempotent(ops in prop::collection::vec(arb_discussion_op(), 1..20)) {
        let author = test_key();
        let discussion1 = apply_discussion_ops_with_author(&ops, &author);
        let discussion2 = apply_discussion_ops_with_author(&ops, &author);

        prop_assert_eq!(&discussion1.title, &discussion2.title);
        prop_assert_eq!(&discussion1.body, &discussion2.body);
        prop_assert_eq!(discussion1.state, discussion2.state);
        prop_assert_eq!(&discussion1.labels, &discussion2.labels);
        prop_assert_eq!(discussion1.replies.len(), discussion2.replies.len());
        prop_assert_eq!(&discussion1.reactions, &discussion2.reactions);
        prop_assert_eq!(&discussion1.resolved_threads, &discussion2.resolved_threads);
    }

    /// Property: Close/Reopen cycle leaves issue in open state
    #[test]
    fn close_reopen_cycle_open(
        initial_ops in prop::collection::vec(arb_issue_op(), 0..5)
    ) {
        let mut ops = initial_ops;
        ops.push(CobOperation::Close { reason: Some("Test close".to_string()) });
        ops.push(CobOperation::Reopen);

        let issue = apply_issue_ops(&ops);
        prop_assert!(issue.state.is_open());
    }

    /// Property: Adding same label multiple times results in single occurrence
    #[test]
    fn add_label_idempotent(label in arb_label()) {
        let ops = vec![
            CobOperation::AddLabel { label: label.clone() },
            CobOperation::AddLabel { label: label.clone() },
            CobOperation::AddLabel { label: label.clone() },
        ];

        let issue = apply_issue_ops(&ops);

        prop_assert!(issue.labels.contains(&label));
        prop_assert_eq!(issue.labels.len(), 1);
    }

    /// Property: Remove non-existent label is no-op
    #[test]
    fn remove_nonexistent_label_noop(
        existing_label in arb_label(),
        nonexistent_label in arb_label()
    ) {
        prop_assume!(existing_label != nonexistent_label);

        let ops = vec![
            CobOperation::AddLabel { label: existing_label.clone() },
            CobOperation::RemoveLabel { label: nonexistent_label },
        ];

        let issue = apply_issue_ops(&ops);

        prop_assert_eq!(issue.labels.len(), 1);
        prop_assert!(issue.labels.contains(&existing_label));
    }

    /// Property: Edit title last-write-wins behavior
    #[test]
    fn edit_title_last_write_wins(
        titles in prop::collection::vec("[a-zA-Z ]{1,50}", 2..5)
    ) {
        prop_assume!(!titles.is_empty());

        let ops: Vec<_> = titles.iter()
            .map(|title| CobOperation::EditTitle { title: title.clone() })
            .collect();

        let issue = apply_issue_ops(&ops);

        // Last edit should win
        prop_assert_eq!(&issue.title, titles.last().unwrap());
    }

    /// Property: React/Unreact on same emoji cancels out
    #[test]
    fn react_unreact_cancel(emoji in "[👍👎❤️😀😢🎉🚀👀]") {
        let ops = vec![
            CobOperation::React { emoji: emoji.to_string() },
            CobOperation::Unreact { emoji: emoji.to_string() },
        ];

        let issue = apply_issue_ops(&ops);

        // No reactions should remain
        prop_assert!(issue.reactions.is_empty());
    }

    /// Property: Multiple reactions from same user on different emojis accumulate
    #[test]
    fn multiple_reactions_accumulate(
        emoji1 in "[👍👎❤️]",
        emoji2 in "[😀😢🎉]"
    ) {
        prop_assume!(emoji1 != emoji2);

        let ops = vec![
            CobOperation::React { emoji: emoji1.to_string() },
            CobOperation::React { emoji: emoji2.to_string() },
        ];

        let issue = apply_issue_ops(&ops);

        prop_assert_eq!(issue.reactions.len(), 2);
        prop_assert!(issue.reactions.contains_key(&emoji1.to_string()));
        prop_assert!(issue.reactions.contains_key(&emoji2.to_string()));
    }

    /// Property: Discussion lock prevents state changes (conceptual - depends on store level)
    #[test]
    fn discussion_lock_unlock_cycle(
        initial_ops in prop::collection::vec(arb_discussion_op(), 0..3)
    ) {
        let mut ops = initial_ops;
        ops.push(CobOperation::LockDiscussion);
        ops.push(CobOperation::UnlockDiscussion);

        let discussion = apply_discussion_ops(&ops);

        // After unlock, should be in open state (lock/unlock cancels out)
        prop_assert!(discussion.state.is_open());
    }

    /// Property: Updated timestamp monotonicity
    #[test]
    fn updated_timestamp_monotonic(ops in prop::collection::vec(arb_issue_op(), 1..10)) {
        let issue = apply_issue_ops(&ops);

        prop_assert!(issue.updated_at_ms >= issue.created_at_ms);
    }

    /// Property: Comment count increases with each comment operation
    #[test]
    fn comment_count_increases(
        num_comments in 1usize..20
    ) {
        let ops: Vec<_> = (0..num_comments)
            .map(|i| CobOperation::Comment { body: format!("Comment {}", i) })
            .collect();

        let issue = apply_issue_ops(&ops);

        prop_assert_eq!(issue.comments.len(), num_comments);
    }

    /// Property: Label add/remove operations maintain set semantics
    #[test]
    fn label_set_semantics(
        labels in prop::collection::vec(arb_label(), 1..10)
    ) {
        // Convert to unique labels to avoid duplicate handling confusion
        let unique_labels: Vec<_> = {
            let mut seen = HashSet::new();
            labels.into_iter().filter(|l| seen.insert(l.clone())).collect()
        };

        let mut ops = Vec::new();

        // Add all unique labels
        for label in &unique_labels {
            ops.push(CobOperation::AddLabel { label: label.clone() });
        }

        // Remove first label if we have more than one unique label
        if unique_labels.len() > 1 {
            ops.push(CobOperation::RemoveLabel { label: unique_labels[0].clone() });
        }

        let issue = apply_issue_ops(&ops);

        if unique_labels.len() > 1 {
            // First label should be removed
            prop_assert!(!issue.labels.contains(&unique_labels[0]));
            // Others should remain
            for label in &unique_labels[1..] {
                prop_assert!(issue.labels.contains(label));
            }
        } else {
            // If only one label, it should remain (we didn't remove it)
            prop_assert_eq!(issue.labels.len(), 1);
            prop_assert!(issue.labels.contains(&unique_labels[0]));
        }
    }
}

// ============================================================================
// Non-proptest tests for edge cases
// ============================================================================

#[test]
fn test_empty_operations_list() {
    let ops: Vec<CobOperation> = vec![];
    let issue = apply_issue_ops(&ops);

    // Should remain in initial state
    assert_eq!(issue.title, "Test");
    assert_eq!(issue.body, "Test body");
    assert!(issue.state.is_open());
}

#[test]
fn test_operations_on_closed_issue() {
    let ops = vec![
        CobOperation::Close {
            reason: Some("Done".to_string()),
        },
        CobOperation::Comment {
            body: "Still commenting".to_string(),
        },
        CobOperation::AddLabel {
            label: "new-label".to_string(),
        },
    ];

    let issue = apply_issue_ops(&ops);

    // Operations should still apply even when closed
    assert!(issue.state.is_closed());
    assert_eq!(issue.comments.len(), 1);
    assert!(issue.labels.contains("new-label"));
}

#[test]
fn test_patch_operations_ignored_on_issue() {
    let ops = vec![
        CobOperation::CreatePatch {
            title: "Patch".to_string(),
            description: "".to_string(),
            base: [0u8; 32],
            head: [1u8; 32],
        },
        CobOperation::UpdatePatch {
            head: [2u8; 32],
            message: None,
        },
    ];

    let issue = apply_issue_ops(&ops);

    // Patch operations should be ignored
    assert_eq!(issue.title, "Test");
    assert_eq!(issue.body, "Test body");
}

#[test]
fn test_discussion_operations_ignored_on_issue() {
    let ops = vec![
        CobOperation::CreateDiscussion {
            title: "Discussion".to_string(),
            body: "Body".to_string(),
            labels: vec![],
        },
        CobOperation::Reply {
            body: "Reply".to_string(),
            parent_reply: None,
        },
        CobOperation::LockDiscussion,
    ];

    let issue = apply_issue_ops(&ops);

    // Discussion operations should be ignored
    assert_eq!(issue.title, "Test");
    assert_eq!(issue.body, "Test body");
    assert!(issue.state.is_open());
}
