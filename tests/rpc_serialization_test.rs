//! Tests for RPC message serialization and deserialization.
//!
//! Tests TUI RPC and Raft RPC protocol messages for correct serialization
//! roundtrips using postcard (binary) and JSON formats.
//!
//! Target: Cover protocol message types without requiring network infrastructure.

mod support;

use proptest::prelude::*;

use aspen::tui_rpc::{
    AddLearnerResultResponse, ChangeMembershipResultResponse, ClusterStateResponse,
    ClusterTicketResponse, ErrorResponse, HealthResponse, InitResultResponse, MAX_CLUSTER_NODES,
    MAX_TUI_MESSAGE_SIZE, NodeDescriptor, NodeInfoResponse, RaftMetricsResponse,
    ReadResultResponse, SnapshotResultResponse, TUI_ALPN, TuiRpcRequest, TuiRpcResponse,
    WriteResultResponse,
};

// TUI RPC Request serialization tests

proptest! {
    /// GetHealth request serializes correctly.
    #[test]
    fn test_tui_get_health_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetHealth;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetHealth => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// GetRaftMetrics request serializes correctly.
    #[test]
    fn test_tui_get_raft_metrics_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetRaftMetrics;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetRaftMetrics => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// GetLeader request serializes correctly.
    #[test]
    fn test_tui_get_leader_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetLeader;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetLeader => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// GetNodeInfo request serializes correctly.
    #[test]
    fn test_tui_get_node_info_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetNodeInfo;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetNodeInfo => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// GetClusterTicket request serializes correctly.
    #[test]
    fn test_tui_get_cluster_ticket_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetClusterTicket;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetClusterTicket => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// InitCluster request serializes correctly.
    #[test]
    fn test_tui_init_cluster_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::InitCluster;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::InitCluster => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ReadKey request serializes correctly.
    #[test]
    fn test_tui_read_key_postcard_roundtrip(key in "[a-zA-Z0-9_:/-]{1,100}") {
        let request = TuiRpcRequest::ReadKey { key: key.clone() };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::ReadKey { key: k } => prop_assert_eq!(k, key),
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// WriteKey request serializes correctly.
    #[test]
    fn test_tui_write_key_postcard_roundtrip(
        key in "[a-zA-Z0-9_:/-]{1,100}",
        value in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let request = TuiRpcRequest::WriteKey {
            key: key.clone(),
            value: value.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::WriteKey { key: k, value: v } => {
                prop_assert_eq!(k, key);
                prop_assert_eq!(v, value);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// TriggerSnapshot request serializes correctly.
    #[test]
    fn test_tui_trigger_snapshot_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::TriggerSnapshot;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::TriggerSnapshot => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// AddLearner request serializes correctly.
    #[test]
    fn test_tui_add_learner_postcard_roundtrip(
        node_id in any::<u64>(),
        addr in "[a-zA-Z0-9]{32}"
    ) {
        let request = TuiRpcRequest::AddLearner {
            node_id,
            addr: addr.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::AddLearner { node_id: n, addr: a } => {
                prop_assert_eq!(n, node_id);
                prop_assert_eq!(a, addr);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ChangeMembership request serializes correctly.
    #[test]
    fn test_tui_change_membership_postcard_roundtrip(
        members in prop::collection::vec(any::<u64>(), 1..10)
    ) {
        let request = TuiRpcRequest::ChangeMembership {
            members: members.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::ChangeMembership { members: m } => {
                prop_assert_eq!(m, members);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// Ping request serializes correctly.
    #[test]
    fn test_tui_ping_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::Ping;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::Ping => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// GetClusterState request serializes correctly.
    #[test]
    fn test_tui_get_cluster_state_postcard_roundtrip(_dummy in Just(())) {
        let request = TuiRpcRequest::GetClusterState;
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: TuiRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcRequest::GetClusterState => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }
}

// TUI RPC Response serialization tests

proptest! {
    /// HealthResponse serializes correctly.
    #[test]
    fn test_health_response_postcard_roundtrip(
        status in "(healthy|degraded|unhealthy)",
        node_id in any::<u64>(),
        raft_node_id in prop::option::of(any::<u64>()),
        uptime_seconds in any::<u64>()
    ) {
        let response = TuiRpcResponse::Health(HealthResponse {
            status: status.clone(),
            node_id,
            raft_node_id,
            uptime_seconds,
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::Health(h) => {
                prop_assert_eq!(h.status, status);
                prop_assert_eq!(h.node_id, node_id);
                prop_assert_eq!(h.raft_node_id, raft_node_id);
                prop_assert_eq!(h.uptime_seconds, uptime_seconds);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// RaftMetricsResponse serializes correctly.
    #[test]
    fn test_raft_metrics_response_postcard_roundtrip(
        node_id in any::<u64>(),
        state in "(Leader|Follower|Candidate|Learner)",
        current_leader in prop::option::of(any::<u64>()),
        current_term in any::<u64>(),
        last_log_index in prop::option::of(any::<u64>()),
        last_applied_index in prop::option::of(any::<u64>()),
        snapshot_index in prop::option::of(any::<u64>())
    ) {
        let response = TuiRpcResponse::RaftMetrics(RaftMetricsResponse {
            node_id,
            state: state.clone(),
            current_leader,
            current_term,
            last_log_index,
            last_applied_index,
            snapshot_index,
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::RaftMetrics(m) => {
                prop_assert_eq!(m.node_id, node_id);
                prop_assert_eq!(m.state, state);
                prop_assert_eq!(m.current_leader, current_leader);
                prop_assert_eq!(m.current_term, current_term);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// Leader response serializes correctly.
    #[test]
    fn test_leader_response_postcard_roundtrip(
        leader_id in prop::option::of(any::<u64>())
    ) {
        let response = TuiRpcResponse::Leader(leader_id);
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::Leader(l) => prop_assert_eq!(l, leader_id),
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// NodeInfoResponse serializes correctly.
    #[test]
    fn test_node_info_response_postcard_roundtrip(
        node_id in any::<u64>(),
        endpoint_addr in "[a-zA-Z0-9]{32,64}"
    ) {
        let response = TuiRpcResponse::NodeInfo(NodeInfoResponse {
            node_id,
            endpoint_addr: endpoint_addr.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::NodeInfo(n) => {
                prop_assert_eq!(n.node_id, node_id);
                prop_assert_eq!(n.endpoint_addr, endpoint_addr);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ClusterTicketResponse serializes correctly.
    #[test]
    fn test_cluster_ticket_response_postcard_roundtrip(
        ticket in "[a-zA-Z0-9]{50,200}",
        topic_id in "[a-zA-Z0-9]{32}",
        cluster_id in "[a-zA-Z0-9]{16}",
        endpoint_id in "[a-zA-Z0-9]{32}"
    ) {
        let response = TuiRpcResponse::ClusterTicket(ClusterTicketResponse {
            ticket: ticket.clone(),
            topic_id: topic_id.clone(),
            cluster_id: cluster_id.clone(),
            endpoint_id: endpoint_id.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::ClusterTicket(t) => {
                prop_assert_eq!(t.ticket, ticket);
                prop_assert_eq!(t.topic_id, topic_id);
                prop_assert_eq!(t.cluster_id, cluster_id);
                prop_assert_eq!(t.endpoint_id, endpoint_id);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// InitResultResponse serializes correctly.
    #[test]
    fn test_init_result_response_postcard_roundtrip(
        success in any::<bool>(),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::InitResult(InitResultResponse {
            success,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::InitResult(r) => {
                prop_assert_eq!(r.success, success);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ReadResultResponse serializes correctly.
    #[test]
    fn test_read_result_response_postcard_roundtrip(
        value in prop::option::of(prop::collection::vec(any::<u8>(), 0..100)),
        found in any::<bool>(),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::ReadResult(ReadResultResponse {
            value: value.clone(),
            found,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::ReadResult(r) => {
                prop_assert_eq!(r.value, value);
                prop_assert_eq!(r.found, found);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// WriteResultResponse serializes correctly.
    #[test]
    fn test_write_result_response_postcard_roundtrip(
        success in any::<bool>(),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::WriteResult(WriteResultResponse {
            success,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::WriteResult(r) => {
                prop_assert_eq!(r.success, success);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// SnapshotResultResponse serializes correctly.
    #[test]
    fn test_snapshot_result_response_postcard_roundtrip(
        success in any::<bool>(),
        snapshot_index in prop::option::of(any::<u64>()),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::SnapshotResult(SnapshotResultResponse {
            success,
            snapshot_index,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::SnapshotResult(r) => {
                prop_assert_eq!(r.success, success);
                prop_assert_eq!(r.snapshot_index, snapshot_index);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// AddLearnerResultResponse serializes correctly.
    #[test]
    fn test_add_learner_result_response_postcard_roundtrip(
        success in any::<bool>(),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::AddLearnerResult(AddLearnerResultResponse {
            success,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::AddLearnerResult(r) => {
                prop_assert_eq!(r.success, success);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ChangeMembershipResultResponse serializes correctly.
    #[test]
    fn test_change_membership_result_response_postcard_roundtrip(
        success in any::<bool>(),
        error in prop::option::of("[a-zA-Z0-9 ]{10,50}")
    ) {
        let response = TuiRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
            success,
            error: error.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::ChangeMembershipResult(r) => {
                prop_assert_eq!(r.success, success);
                prop_assert_eq!(r.error, error);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// Pong response serializes correctly.
    #[test]
    fn test_pong_response_postcard_roundtrip(_dummy in Just(())) {
        let response = TuiRpcResponse::Pong;
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::Pong => {}
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// ErrorResponse serializes correctly.
    #[test]
    fn test_error_response_postcard_roundtrip(
        code in "[A-Z_]{3,20}",
        message in "[a-zA-Z0-9 ]{10,100}"
    ) {
        let response = TuiRpcResponse::Error(ErrorResponse {
            code: code.clone(),
            message: message.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::Error(e) => {
                prop_assert_eq!(e.code, code);
                prop_assert_eq!(e.message, message);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }

    /// TuiRpcResponse::error helper works correctly.
    #[test]
    fn test_tui_rpc_response_error_helper(
        code in "[A-Z_]{3,20}",
        message in "[a-zA-Z0-9 ]{10,100}"
    ) {
        let response = TuiRpcResponse::error(code.clone(), message.clone());
        match response {
            TuiRpcResponse::Error(e) => {
                prop_assert_eq!(e.code, code);
                prop_assert_eq!(e.message, message);
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }
}

// Unit tests for specific scenarios and constants

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_tui_alpn_constant() {
        assert_eq!(TUI_ALPN, b"aspen-tui");
    }

    #[test]
    fn test_max_tui_message_size_constant() {
        assert_eq!(MAX_TUI_MESSAGE_SIZE, 1024 * 1024); // 1 MB
    }

    #[test]
    fn test_max_cluster_nodes_constant() {
        assert_eq!(MAX_CLUSTER_NODES, 16);
    }

    #[test]
    fn test_cluster_state_response_with_nodes() {
        let nodes = vec![
            NodeDescriptor {
                node_id: 1,
                endpoint_addr: "node1_addr".to_string(),
                is_voter: true,
                is_learner: false,
                is_leader: true,
            },
            NodeDescriptor {
                node_id: 2,
                endpoint_addr: "node2_addr".to_string(),
                is_voter: true,
                is_learner: false,
                is_leader: false,
            },
            NodeDescriptor {
                node_id: 3,
                endpoint_addr: "node3_addr".to_string(),
                is_voter: false,
                is_learner: true,
                is_leader: false,
            },
        ];
        let response = TuiRpcResponse::ClusterState(ClusterStateResponse {
            nodes: nodes.clone(),
            leader_id: Some(1),
            this_node_id: 2,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            TuiRpcResponse::ClusterState(state) => {
                assert_eq!(state.nodes.len(), 3);
                assert_eq!(state.leader_id, Some(1));
                assert_eq!(state.this_node_id, 2);
                assert!(state.nodes[0].is_leader);
                assert!(state.nodes[2].is_learner);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_node_descriptor_all_fields() {
        let descriptor = NodeDescriptor {
            node_id: 42,
            endpoint_addr: "test_endpoint".to_string(),
            is_voter: true,
            is_learner: false,
            is_leader: true,
        };

        let serialized = postcard::to_stdvec(&descriptor).expect("serialize");
        let deserialized: NodeDescriptor = postcard::from_bytes(&serialized).expect("deserialize");

        assert_eq!(deserialized.node_id, 42);
        assert_eq!(deserialized.endpoint_addr, "test_endpoint");
        assert!(deserialized.is_voter);
        assert!(!deserialized.is_learner);
        assert!(deserialized.is_leader);
    }

    #[test]
    fn test_health_response_json() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 3600,
        };

        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("healthy"));
        assert!(json.contains("3600"));

        let parsed: HealthResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.status, "healthy");
        assert_eq!(parsed.uptime_seconds, 3600);
    }

    #[test]
    fn test_all_request_variants_serialize() {
        // Verify all request variants can serialize without error
        let requests = vec![
            TuiRpcRequest::GetHealth,
            TuiRpcRequest::GetRaftMetrics,
            TuiRpcRequest::GetLeader,
            TuiRpcRequest::GetNodeInfo,
            TuiRpcRequest::GetClusterTicket,
            TuiRpcRequest::InitCluster,
            TuiRpcRequest::ReadKey {
                key: "test".to_string(),
            },
            TuiRpcRequest::WriteKey {
                key: "test".to_string(),
                value: vec![1, 2, 3],
            },
            TuiRpcRequest::TriggerSnapshot,
            TuiRpcRequest::AddLearner {
                node_id: 42,
                addr: "addr".to_string(),
            },
            TuiRpcRequest::ChangeMembership {
                members: vec![1, 2, 3],
            },
            TuiRpcRequest::Ping,
            TuiRpcRequest::GetClusterState,
        ];

        for request in requests {
            let serialized = postcard::to_stdvec(&request);
            assert!(serialized.is_ok(), "Failed to serialize: {:?}", request);
            assert!(
                serialized.unwrap().len() < MAX_TUI_MESSAGE_SIZE,
                "Message too large: {:?}",
                request
            );
        }
    }

    #[test]
    fn test_all_response_variants_serialize() {
        // Verify all response variants can serialize without error
        let responses = vec![
            TuiRpcResponse::Health(HealthResponse {
                status: "healthy".to_string(),
                node_id: 1,
                raft_node_id: None,
                uptime_seconds: 0,
            }),
            TuiRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: 1,
                state: "Leader".to_string(),
                current_leader: Some(1),
                current_term: 1,
                last_log_index: Some(10),
                last_applied_index: Some(10),
                snapshot_index: None,
            }),
            TuiRpcResponse::Leader(Some(1)),
            TuiRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: 1,
                endpoint_addr: "addr".to_string(),
            }),
            TuiRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: "ticket".to_string(),
                topic_id: "topic".to_string(),
                cluster_id: "cluster".to_string(),
                endpoint_id: "endpoint".to_string(),
            }),
            TuiRpcResponse::InitResult(InitResultResponse {
                success: true,
                error: None,
            }),
            TuiRpcResponse::ReadResult(ReadResultResponse {
                value: Some(vec![1, 2, 3]),
                found: true,
                error: None,
            }),
            TuiRpcResponse::WriteResult(WriteResultResponse {
                success: true,
                error: None,
            }),
            TuiRpcResponse::SnapshotResult(SnapshotResultResponse {
                success: true,
                snapshot_index: Some(100),
                error: None,
            }),
            TuiRpcResponse::AddLearnerResult(AddLearnerResultResponse {
                success: true,
                error: None,
            }),
            TuiRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
                success: true,
                error: None,
            }),
            TuiRpcResponse::Pong,
            TuiRpcResponse::ClusterState(ClusterStateResponse {
                nodes: vec![],
                leader_id: None,
                this_node_id: 1,
            }),
            TuiRpcResponse::error("TEST_ERROR", "Test error message"),
        ];

        for response in responses {
            let serialized = postcard::to_stdvec(&response);
            assert!(serialized.is_ok(), "Failed to serialize: {:?}", response);
        }
    }

    #[test]
    fn test_large_cluster_state_within_bounds() {
        // Create MAX_CLUSTER_NODES nodes
        let nodes: Vec<NodeDescriptor> = (0..MAX_CLUSTER_NODES)
            .map(|i| NodeDescriptor {
                node_id: i as u64,
                endpoint_addr: format!("node_{}_endpoint_address_string", i),
                is_voter: i < MAX_CLUSTER_NODES / 2,
                is_learner: i >= MAX_CLUSTER_NODES / 2,
                is_leader: i == 0,
            })
            .collect();

        let response = TuiRpcResponse::ClusterState(ClusterStateResponse {
            nodes,
            leader_id: Some(0),
            this_node_id: 1,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        assert!(serialized.len() < MAX_TUI_MESSAGE_SIZE);

        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            TuiRpcResponse::ClusterState(state) => {
                assert_eq!(state.nodes.len(), MAX_CLUSTER_NODES);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_empty_cluster_state() {
        let response = TuiRpcResponse::ClusterState(ClusterStateResponse {
            nodes: vec![],
            leader_id: None,
            this_node_id: 0,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: TuiRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            TuiRpcResponse::ClusterState(state) => {
                assert!(state.nodes.is_empty());
                assert!(state.leader_id.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }
}
