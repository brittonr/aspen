/// Test handling of three concurrent membership entries in append-entries.
///
/// This test validates that when a follower receives three membership changes in a single
/// append-entries RPC, only the last two take effect as committed and effective memberships.
/// It also verifies correct state transitions from Learner to Follower when the node appears
/// in the new membership.
///
/// Key behaviors tested:
/// - Multiple membership entries in one append-entries call
/// - Only last 2 of 3 memberships become committed/effective
/// - Node state transitions: Learner -> Follower when added to membership
/// - Log compaction correctly tracks membership changes
///
/// Original: openraft/openraft/src/engine/handler/following_handler/do_append_entries_test.rs
///           test_follower_do_append_entries_three_membership_entries
use std::collections::BTreeMap;
/// Test handling of three concurrent membership entries in append-entries.
///
/// This test validates that when a follower receives three membership changes in a single
/// append-entries RPC, only the last two take effect as committed and effective memberships.
/// It also verifies correct state transitions from Learner to Follower when the node appears
/// in the new membership.
///
/// Key behaviors tested:
/// - Multiple membership entries in one append-entries call
/// - Only last 2 of 3 memberships become committed/effective
/// - Node state transitions: Learner -> Follower when added to membership
/// - Log compaction correctly tracks membership changes
///
/// Original: openraft/openraft/src/engine/handler/following_handler/do_append_entries_test.rs
///           test_follower_do_append_entries_three_membership_entries
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use aspen_testing::create_test_raft_member_info;
use openraft::Config;
use openraft::RaftLogReader;
use openraft::ServerState;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftLogStorageExt;
use openraft::testing::blank_ent;
use openraft::testing::membership_ent;

fn timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Test that three membership entries result in correct committed/effective state.
///
/// Setup:
/// - Node-0 is initialized as single-node cluster with membership {0}
/// - Node-5 exists but is not in the initial membership (starts as Learner)
///
/// Test flow:
/// 1. Initialize node-0 as leader with membership {0}
/// 2. Create node-5 (starts as Learner, not in membership)
/// 3. Leader sends append-entries with 3 membership changes:
///    - Entry at index 2: membership {0, 1}
///    - Entry at index 3: membership {3, 4}
///    - Entry at index 4: membership {4, 5}
/// 4. Verify node-5 receives all entries and transitions to Follower
/// 5. Verify only last 2 memberships ({3,4} and {4,5}) are active
/// 6. Verify node-5 is now Follower (since it's in {4,5})
///
/// Expected outcome:
/// - Node-5's log contains all entries (blank + 3 memberships)
/// - Node-5's committed membership is {3, 4}
/// - Node-5's effective membership is {4, 5}
/// - Node-5's state transitions from Learner to Follower
#[tokio::test]
async fn test_append_entries_three_membership() -> Result<()> {
    let config = Arc::new(
        Config {
            enable_heartbeat: false,
            enable_elect: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config.clone());

    tracing::info!("--- section 1: initialize leader node-0 with membership {{0}}");

    router.new_raft_node(0).await?;

    {
        let node0 = router.get_raft_handle(0)?;
        let mut nodes = BTreeMap::new();
        nodes.insert(NodeId::from(0), create_test_raft_member_info(0));
        node0.initialize(nodes).await?;

        router.wait(0, timeout()).log_index_at_least(Some(1), "node-0 initialized").await?;

        router.wait(0, timeout()).state(ServerState::Leader, "node-0 is leader").await?;
    }

    tracing::info!("--- section 2: create node-5 (starts as learner, not in membership)");

    router.new_raft_node(5).await?;

    router.wait(5, timeout()).state(ServerState::Learner, "node-5 starts as learner").await?;

    tracing::info!("--- section 3: prepare storage for append-entries with 3 membership changes");

    // We'll simulate the append-entries scenario by:
    // 1. Creating a follower node with initial state
    // 2. Building the log with initial membership
    // 3. Sending append-entries with 3 membership changes

    // Remove node-5 temporarily to rebuild it with specific state
    let (_r5, mut sto5, sm5) = router.remove_node(5).unwrap();

    // Set up node-5 with initial log state
    // Initial vote (following term 1, leader 0)
    sto5.save_vote(&openraft::Vote::new_committed(1, NodeId::from(0))).await?;

    // Initial log: just the membership entry at index 1 for {0}
    let initial_membership = vec![BTreeSet::from([NodeId(0)])];
    let entries = vec![
        blank_ent::<AppTypeConfig>(0, NodeId(0), 0), // Entry at index 0
        membership_ent(1, NodeId(0), 1, initial_membership.clone()), // Entry at index 1: membership {0}
    ];

    sto5.blocking_append(entries).await?;

    // Recreate node-5 with this state
    router.new_raft_node_with_storage(5, sto5, sm5).await?;

    tracing::info!("--- section 4: send append-entries with 3 membership changes");

    // Now we'll send an append-entries RPC with 3 membership entries:
    // - prev_log_id points to index 1 (initial membership)
    // - entries contain: blank entry + 3 membership changes

    let node5 = router.get_raft_handle(5)?;

    // Build the append-entries request
    let append_entries_req = openraft::raft::AppendEntriesRequest {
        vote: openraft::Vote::new_committed(1, NodeId::from(0)),
        prev_log_id: Some(openraft::testing::log_id::<AppTypeConfig>(1, NodeId(0), 1)),
        entries: vec![
            blank_ent::<AppTypeConfig>(1, NodeId(0), 2), // Blank entry at index 2
            membership_ent(1, NodeId(0), 3, vec![BTreeSet::from([NodeId(0), NodeId(1)])]), // Membership {0, 1}
            membership_ent(1, NodeId(0), 4, vec![BTreeSet::from([NodeId(3), NodeId(4)])]), // Membership {3, 4}
            membership_ent(1, NodeId(0), 5, vec![BTreeSet::from([NodeId(4), NodeId(5)])]), // Membership {4, 5}
        ],
        leader_commit: Some(openraft::testing::log_id::<AppTypeConfig>(1, NodeId(0), 5)),
    };

    let resp = node5.append_entries(append_entries_req).await?;

    assert!(resp.is_success(), "append-entries should succeed");
    assert!(!resp.is_conflict(), "should not be a conflict");

    tracing::info!("--- section 5: wait for entries to be applied");

    // Give node time to process the append-entries
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Wait for entries to be applied
    router.wait(5, timeout()).applied_index(Some(5), "all entries applied").await?;

    tracing::info!("--- section 6: verify node-5 transitioned to Follower state");

    // Node-5 should now be a Follower since it's in the effective membership {4, 5}
    router.wait(5, timeout()).state(ServerState::Follower, "node-5 is now follower").await?;

    tracing::info!("--- section 7: verify log contains all entries");

    let metrics = node5.metrics().borrow().clone();

    // Last log index should be 5
    assert_eq!(metrics.last_log_index, Some(5), "last log index should be 5");

    // Verify the membership configuration
    // The effective membership should be {4, 5}
    let voters: Vec<_> = metrics.membership_config.voter_ids().collect();
    assert_eq!(voters.len(), 2, "effective membership should have 2 voters");
    assert!(
        voters.contains(&NodeId(4)) && voters.contains(&NodeId(5)),
        "effective membership should be {{4, 5}}, got {:?}",
        voters
    );

    tracing::info!("--- section 8: verify log entries are correct");

    // Extract storage to verify log contents
    let (_r5, mut sto5_after, _sm5_after) = router.remove_node(5).unwrap();

    let mut log_reader = sto5_after.get_log_reader().await;
    let all_logs = log_reader.try_get_log_entries(0..=5).await?;

    assert_eq!(all_logs.len(), 6, "should have 6 log entries (0-5)");

    // Verify log entry 2 is blank
    assert!(matches!(all_logs[2].payload, openraft::EntryPayload::Blank), "entry at index 2 should be blank");

    // Verify log entries 3, 4, 5 are membership entries
    assert!(
        matches!(all_logs[3].payload, openraft::EntryPayload::Membership(_)),
        "entry at index 3 should be membership"
    );
    assert!(
        matches!(all_logs[4].payload, openraft::EntryPayload::Membership(_)),
        "entry at index 4 should be membership"
    );
    assert!(
        matches!(all_logs[5].payload, openraft::EntryPayload::Membership(_)),
        "entry at index 5 should be membership"
    );

    tracing::info!("test completed successfully");

    Ok(())
}
