/// Heartbeat reject vote test - ported from OpenRaft.
///
/// This test validates that a follower receiving heartbeats from a leader will reject
/// vote requests until the leader lease expires. This ensures that a healthy cluster
/// with an active leader doesn't experience spurious elections.
///
/// The test:
/// 1. Creates a cluster with 3 voters and 1 learner
/// 2. Verifies that heartbeats update the vote_last_modified timestamp (leader lease)
/// 3. Shows that vote requests are rejected while leader lease is active
/// 4. Disables heartbeat and waits for lease to expire
/// 5. Shows that vote requests are granted after lease expires
///
/// Original: openraft/tests/tests/append_entries/t61_heartbeat_reject_vote.rs
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use anyhow::Result;
use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use openraft::Config;
use openraft::TokioInstant;
use openraft::Vote;
use openraft::raft::VoteRequest;
use openraft::testing::log_id;
use tokio::time::sleep;

fn timeout() -> Option<Duration> {
    Some(Duration::from_millis(1000))
}

/// Test that heartbeat prevents vote requests by maintaining leader lease.
///
/// When a follower receives heartbeat from the leader, it should reject vote
/// requests from other nodes until the leader lease expires. This prevents
/// unnecessary elections when the cluster is healthy.
#[tokio::test]
async fn test_heartbeat_reject_vote() -> Result<()> {
    let config = Arc::new(
        Config {
            heartbeat_interval: 200,
            election_timeout_min: 1000,
            election_timeout_max: 1001,
            ..Default::default()
        }
        .validate()?,
    );
    let mut router = AspenRouter::new(config.clone());

    let now = TokioInstant::now();
    sleep(Duration::from_millis(1)).await;

    let log_index = router
        .new_cluster(BTreeSet::from([NodeId(0), NodeId(1), NodeId(2)]), BTreeSet::from([NodeId(3)]))
        .await?;

    let vote_modified_time = Arc::new(StdMutex::new(Some(TokioInstant::now())));

    tracing::info!(log_index, "--- leader lease is set by heartbeat");
    {
        let m = vote_modified_time.clone();

        router
            .external_request(1, move |state| {
                let mut l = m.lock().unwrap();
                *l = state.vote_last_modified();
                assert!(state.vote_last_modified() > Some(now));
            })
            .await?;

        let now = TokioInstant::now();
        sleep(Duration::from_millis(700)).await;

        let m = vote_modified_time.clone();

        router
            .external_request(1, move |state| {
                let l = m.lock().unwrap();
                assert!(state.vote_last_modified() > Some(now));
                assert!(state.vote_last_modified() > *l);
            })
            .await?;
    }

    let node0 = router.get_raft_handle(0)?;
    let node1 = router.get_raft_handle(1)?;

    tracing::info!(log_index, "--- leader lease rejects vote request");
    {
        let res = node1
            .vote(VoteRequest::new(Vote::new(10, NodeId::from(2)), Some(log_id(10, NodeId(1), 10))))
            .await?;
        assert!(!res.is_granted_to(&Vote::new(10, NodeId::from(2))), "vote is rejected while leader lease active");
    }

    tracing::info!(log_index, "--- ensures no more blank-log heartbeat is used");
    {
        // This verifies that blank-log heartbeats (if any) don't write extra log entries
        sleep(Duration::from_millis(1500)).await;
        router.wait(1, timeout()).applied_index(Some(log_index), "no log is written").await?;
    }

    tracing::info!(log_index, "--- disable heartbeat, vote request will be granted");
    {
        node0.runtime_config().heartbeat(false);
        sleep(Duration::from_millis(1500)).await;

        router.wait(1, timeout()).applied_index(Some(log_index), "no log is written").await?;

        let res = node1
            .vote(VoteRequest::new(Vote::new(10, NodeId::from(2)), Some(log_id(10, NodeId(1), 10))))
            .await?;
        assert!(res.is_granted_to(&Vote::new(10, NodeId::from(2))), "vote is granted after leader lease expired");
    }

    Ok(())
}
