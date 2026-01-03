//! Tests for RPC message serialization and deserialization.
//!
//! Tests Client RPC and Raft RPC protocol messages for correct serialization
//! roundtrips using postcard (binary) and JSON formats.
//!
//! Target: Cover protocol message types without requiring network infrastructure.

mod support;

use aspen::CLIENT_ALPN;
use aspen::client_rpc::AddLearnerResultResponse;
use aspen::client_rpc::ChangeMembershipResultResponse;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::ClusterStateResponse;
use aspen::client_rpc::ClusterTicketResponse;
use aspen::client_rpc::ErrorResponse;
use aspen::client_rpc::HealthResponse;
use aspen::client_rpc::InitResultResponse;
use aspen::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use aspen::client_rpc::MAX_CLUSTER_NODES;
use aspen::client_rpc::NodeDescriptor;
use aspen::client_rpc::NodeInfoResponse;
use aspen::client_rpc::RaftMetricsResponse;
use aspen::client_rpc::ReadResultResponse;
use aspen::client_rpc::SnapshotResultResponse;
use aspen::client_rpc::WriteResultResponse;
use bolero::check;
use support::bolero_generators::ClusterTicketString;
use support::bolero_generators::EndpointIdString;
use support::bolero_generators::ErrorCode;
use support::bolero_generators::ErrorMessage;
use support::bolero_generators::HexAddrString;
use support::bolero_generators::MembershipList;
use support::bolero_generators::OptionalBinaryValue;
use support::bolero_generators::OptionalErrorString;
use support::bolero_generators::RaftStateString;
use support::bolero_generators::StatusString;
use support::bolero_generators::TopicIdString;
use support::bolero_generators::TuiBinaryData;
use support::bolero_generators::TuiRpcKey;

// ============================================================================
// TUI RPC Request serialization tests
// ============================================================================

/// GetHealth request serializes correctly.
#[test]
fn test_client_get_health_postcard_roundtrip() {
    let request = ClientRpcRequest::GetHealth;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetHealth));
}

/// GetRaftMetrics request serializes correctly.
#[test]
fn test_client_get_raft_metrics_postcard_roundtrip() {
    let request = ClientRpcRequest::GetRaftMetrics;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetRaftMetrics));
}

/// GetLeader request serializes correctly.
#[test]
fn test_client_get_leader_postcard_roundtrip() {
    let request = ClientRpcRequest::GetLeader;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetLeader));
}

/// GetNodeInfo request serializes correctly.
#[test]
fn test_client_get_node_info_postcard_roundtrip() {
    let request = ClientRpcRequest::GetNodeInfo;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetNodeInfo));
}

/// GetClusterTicket request serializes correctly.
#[test]
fn test_client_get_cluster_ticket_postcard_roundtrip() {
    let request = ClientRpcRequest::GetClusterTicket;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetClusterTicket));
}

/// InitCluster request serializes correctly.
#[test]
fn test_client_init_cluster_postcard_roundtrip() {
    let request = ClientRpcRequest::InitCluster;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::InitCluster));
}

/// ReadKey request serializes correctly.
#[test]
fn test_client_read_key_postcard_roundtrip() {
    check!().with_type::<TuiRpcKey>().for_each(|key| {
        let request = ClientRpcRequest::ReadKey { key: key.0.clone() };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcRequest::ReadKey { key: k } => assert_eq!(k, key.0),
            _ => panic!("Wrong variant"),
        }
    });
}

/// WriteKey request serializes correctly.
#[test]
fn test_client_write_key_postcard_roundtrip() {
    check!().with_type::<(TuiRpcKey, TuiBinaryData)>().for_each(|(key, value)| {
        let request = ClientRpcRequest::WriteKey {
            key: key.0.clone(),
            value: value.0.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcRequest::WriteKey { key: k, value: v } => {
                assert_eq!(k, key.0);
                assert_eq!(v, value.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// TriggerSnapshot request serializes correctly.
#[test]
fn test_client_trigger_snapshot_postcard_roundtrip() {
    let request = ClientRpcRequest::TriggerSnapshot;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::TriggerSnapshot));
}

/// AddLearner request serializes correctly.
#[test]
fn test_client_add_learner_postcard_roundtrip() {
    check!().with_type::<(u64, HexAddrString)>().for_each(|(node_id, addr)| {
        let request = ClientRpcRequest::AddLearner {
            node_id: *node_id,
            addr: addr.0.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcRequest::AddLearner { node_id: n, addr: a } => {
                assert_eq!(n, *node_id);
                assert_eq!(a, addr.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// ChangeMembership request serializes correctly.
#[test]
fn test_client_change_membership_postcard_roundtrip() {
    check!().with_type::<MembershipList>().for_each(|members| {
        let request = ClientRpcRequest::ChangeMembership {
            members: members.0.clone(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcRequest::ChangeMembership { members: m } => {
                assert_eq!(m, members.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// Ping request serializes correctly.
#[test]
fn test_client_ping_postcard_roundtrip() {
    let request = ClientRpcRequest::Ping;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::Ping));
}

/// GetClusterState request serializes correctly.
#[test]
fn test_client_get_cluster_state_postcard_roundtrip() {
    let request = ClientRpcRequest::GetClusterState;
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcRequest::GetClusterState));
}

// ============================================================================
// TUI RPC Response serialization tests
// ============================================================================

/// HealthResponse serializes correctly.
#[test]
fn test_health_response_postcard_roundtrip() {
    check!().with_type::<(StatusString, u64, Option<u64>, u64)>().for_each(
        |(status, node_id, raft_node_id, uptime_seconds)| {
            let response = ClientRpcResponse::Health(HealthResponse {
                status: status.0.clone(),
                node_id: *node_id,
                raft_node_id: *raft_node_id,
                uptime_seconds: *uptime_seconds,
            });
            let serialized = postcard::to_stdvec(&response).expect("serialize");
            let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
            match deserialized {
                ClientRpcResponse::Health(h) => {
                    assert_eq!(h.status, status.0);
                    assert_eq!(h.node_id, *node_id);
                    assert_eq!(h.raft_node_id, *raft_node_id);
                    assert_eq!(h.uptime_seconds, *uptime_seconds);
                }
                _ => panic!("Wrong variant"),
            }
        },
    );
}

/// RaftMetricsResponse serializes correctly.
#[test]
fn test_raft_metrics_response_postcard_roundtrip() {
    check!()
        .with_type::<(u64, RaftStateString, Option<u64>, u64, Option<u64>, Option<u64>, Option<u64>)>()
        .for_each(
            |(node_id, state, current_leader, current_term, last_log_index, last_applied_index, snapshot_index)| {
                let response = ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                    node_id: *node_id,
                    state: state.0.clone(),
                    current_leader: *current_leader,
                    current_term: *current_term,
                    last_log_index: *last_log_index,
                    last_applied_index: *last_applied_index,
                    snapshot_index: *snapshot_index,
                    replication: None,
                });
                let serialized = postcard::to_stdvec(&response).expect("serialize");
                let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
                match deserialized {
                    ClientRpcResponse::RaftMetrics(m) => {
                        assert_eq!(m.node_id, *node_id);
                        assert_eq!(m.state, state.0);
                        assert_eq!(m.current_leader, *current_leader);
                        assert_eq!(m.current_term, *current_term);
                    }
                    _ => panic!("Wrong variant"),
                }
            },
        );
}

/// Leader response serializes correctly.
#[test]
fn test_leader_response_postcard_roundtrip() {
    check!().with_type::<Option<u64>>().for_each(|leader_id| {
        let response = ClientRpcResponse::Leader(*leader_id);
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::Leader(l) => assert_eq!(l, *leader_id),
            _ => panic!("Wrong variant"),
        }
    });
}

/// NodeInfoResponse serializes correctly.
#[test]
fn test_node_info_response_postcard_roundtrip() {
    check!().with_type::<(u64, EndpointIdString)>().for_each(|(node_id, endpoint_addr)| {
        let response = ClientRpcResponse::NodeInfo(NodeInfoResponse {
            node_id: *node_id,
            endpoint_addr: endpoint_addr.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::NodeInfo(n) => {
                assert_eq!(n.node_id, *node_id);
                assert_eq!(n.endpoint_addr, endpoint_addr.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// ClusterTicketResponse serializes correctly.
#[test]
fn test_cluster_ticket_response_postcard_roundtrip() {
    check!()
        .with_type::<(ClusterTicketString, TopicIdString, TopicIdString, EndpointIdString)>()
        .for_each(|(ticket, topic_id, cluster_id, endpoint_id)| {
            let response = ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket.0.clone(),
                topic_id: topic_id.0.clone(),
                cluster_id: cluster_id.0.clone(),
                endpoint_id: endpoint_id.0.clone(),
                bootstrap_peers: Some(1),
            });
            let serialized = postcard::to_stdvec(&response).expect("serialize");
            let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
            match deserialized {
                ClientRpcResponse::ClusterTicket(t) => {
                    assert_eq!(t.ticket, ticket.0);
                    assert_eq!(t.topic_id, topic_id.0);
                    assert_eq!(t.cluster_id, cluster_id.0);
                    assert_eq!(t.endpoint_id, endpoint_id.0);
                }
                _ => panic!("Wrong variant"),
            }
        });
}

/// InitResultResponse serializes correctly.
#[test]
fn test_init_result_response_postcard_roundtrip() {
    check!().with_type::<(bool, OptionalErrorString)>().for_each(|(success, error)| {
        let response = ClientRpcResponse::InitResult(InitResultResponse {
            success: *success,
            error: error.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::InitResult(r) => {
                assert_eq!(r.success, *success);
                assert_eq!(r.error, error.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// ReadResultResponse serializes correctly.
#[test]
fn test_read_result_response_postcard_roundtrip() {
    check!()
        .with_type::<(OptionalBinaryValue, bool, OptionalErrorString)>()
        .for_each(|(value, found, error)| {
            let response = ClientRpcResponse::ReadResult(ReadResultResponse {
                value: value.0.clone(),
                found: *found,
                error: error.0.clone(),
            });
            let serialized = postcard::to_stdvec(&response).expect("serialize");
            let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
            match deserialized {
                ClientRpcResponse::ReadResult(r) => {
                    assert_eq!(r.value, value.0);
                    assert_eq!(r.found, *found);
                    assert_eq!(r.error, error.0);
                }
                _ => panic!("Wrong variant"),
            }
        });
}

/// WriteResultResponse serializes correctly.
#[test]
fn test_write_result_response_postcard_roundtrip() {
    check!().with_type::<(bool, OptionalErrorString)>().for_each(|(success, error)| {
        let response = ClientRpcResponse::WriteResult(WriteResultResponse {
            success: *success,
            error: error.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::WriteResult(r) => {
                assert_eq!(r.success, *success);
                assert_eq!(r.error, error.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// SnapshotResultResponse serializes correctly.
#[test]
fn test_snapshot_result_response_postcard_roundtrip() {
    check!()
        .with_type::<(bool, Option<u64>, OptionalErrorString)>()
        .for_each(|(success, snapshot_index, error)| {
            let response = ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                success: *success,
                snapshot_index: *snapshot_index,
                error: error.0.clone(),
            });
            let serialized = postcard::to_stdvec(&response).expect("serialize");
            let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
            match deserialized {
                ClientRpcResponse::SnapshotResult(r) => {
                    assert_eq!(r.success, *success);
                    assert_eq!(r.snapshot_index, *snapshot_index);
                    assert_eq!(r.error, error.0);
                }
                _ => panic!("Wrong variant"),
            }
        });
}

/// AddLearnerResultResponse serializes correctly.
#[test]
fn test_add_learner_result_response_postcard_roundtrip() {
    check!().with_type::<(bool, OptionalErrorString)>().for_each(|(success, error)| {
        let response = ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
            success: *success,
            error: error.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::AddLearnerResult(r) => {
                assert_eq!(r.success, *success);
                assert_eq!(r.error, error.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// ChangeMembershipResultResponse serializes correctly.
#[test]
fn test_change_membership_result_response_postcard_roundtrip() {
    check!().with_type::<(bool, OptionalErrorString)>().for_each(|(success, error)| {
        let response = ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
            success: *success,
            error: error.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::ChangeMembershipResult(r) => {
                assert_eq!(r.success, *success);
                assert_eq!(r.error, error.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// Pong response serializes correctly.
#[test]
fn test_pong_response_postcard_roundtrip() {
    let response = ClientRpcResponse::Pong;
    let serialized = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
    assert!(matches!(deserialized, ClientRpcResponse::Pong));
}

/// ErrorResponse serializes correctly.
#[test]
fn test_error_response_postcard_roundtrip() {
    check!().with_type::<(ErrorCode, ErrorMessage)>().for_each(|(code, message)| {
        let response = ClientRpcResponse::Error(ErrorResponse {
            code: code.0.clone(),
            message: message.0.clone(),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, code.0);
                assert_eq!(e.message, message.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

/// ClientRpcResponse::error helper works correctly.
#[test]
fn test_client_rpc_response_error_helper() {
    check!().with_type::<(ErrorCode, ErrorMessage)>().for_each(|(code, message)| {
        let response = ClientRpcResponse::error(code.0.clone(), message.0.clone());
        match response {
            ClientRpcResponse::Error(e) => {
                assert_eq!(e.code, code.0);
                assert_eq!(e.message, message.0);
            }
            _ => panic!("Wrong variant"),
        }
    });
}

// ============================================================================
// Unit tests for specific scenarios and constants
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_client_alpn_constant() {
        assert_eq!(CLIENT_ALPN, b"aspen-client");
    }

    #[test]
    fn test_max_tui_message_size_constant() {
        assert_eq!(MAX_CLIENT_MESSAGE_SIZE, 1024 * 1024); // 1 MB
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
        let response = ClientRpcResponse::ClusterState(ClusterStateResponse {
            nodes: nodes.clone(),
            leader_id: Some(1),
            this_node_id: 2,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcResponse::ClusterState(state) => {
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
            ClientRpcRequest::GetHealth,
            ClientRpcRequest::GetRaftMetrics,
            ClientRpcRequest::GetLeader,
            ClientRpcRequest::GetNodeInfo,
            ClientRpcRequest::GetClusterTicket,
            ClientRpcRequest::InitCluster,
            ClientRpcRequest::ReadKey {
                key: "test".to_string(),
            },
            ClientRpcRequest::WriteKey {
                key: "test".to_string(),
                value: vec![1, 2, 3],
            },
            ClientRpcRequest::TriggerSnapshot,
            ClientRpcRequest::AddLearner {
                node_id: 42,
                addr: "addr".to_string(),
            },
            ClientRpcRequest::ChangeMembership { members: vec![1, 2, 3] },
            ClientRpcRequest::Ping,
            ClientRpcRequest::GetClusterState,
        ];

        for request in requests {
            let serialized = postcard::to_stdvec(&request);
            assert!(serialized.is_ok(), "Failed to serialize: {:?}", request);
            assert!(serialized.unwrap().len() < MAX_CLIENT_MESSAGE_SIZE, "Message too large: {:?}", request);
        }
    }

    #[test]
    fn test_all_response_variants_serialize() {
        // Verify all response variants can serialize without error
        let responses = vec![
            ClientRpcResponse::Health(HealthResponse {
                status: "healthy".to_string(),
                node_id: 1,
                raft_node_id: None,
                uptime_seconds: 0,
            }),
            ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: 1,
                state: "Leader".to_string(),
                current_leader: Some(1),
                current_term: 1,
                last_log_index: Some(10),
                last_applied_index: Some(10),
                snapshot_index: None,
                replication: None,
            }),
            ClientRpcResponse::Leader(Some(1)),
            ClientRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: 1,
                endpoint_addr: "addr".to_string(),
            }),
            ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: "ticket".to_string(),
                topic_id: "topic".to_string(),
                cluster_id: "cluster".to_string(),
                endpoint_id: "endpoint".to_string(),
                bootstrap_peers: Some(1),
            }),
            ClientRpcResponse::InitResult(InitResultResponse {
                success: true,
                error: None,
            }),
            ClientRpcResponse::ReadResult(ReadResultResponse {
                value: Some(vec![1, 2, 3]),
                found: true,
                error: None,
            }),
            ClientRpcResponse::WriteResult(WriteResultResponse {
                success: true,
                error: None,
            }),
            ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                success: true,
                snapshot_index: Some(100),
                error: None,
            }),
            ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
                success: true,
                error: None,
            }),
            ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
                success: true,
                error: None,
            }),
            ClientRpcResponse::Pong,
            ClientRpcResponse::ClusterState(ClusterStateResponse {
                nodes: vec![],
                leader_id: None,
                this_node_id: 1,
            }),
            ClientRpcResponse::error("TEST_ERROR", "Test error message"),
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

        let response = ClientRpcResponse::ClusterState(ClusterStateResponse {
            nodes,
            leader_id: Some(0),
            this_node_id: 1,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        assert!(serialized.len() < MAX_CLIENT_MESSAGE_SIZE);

        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
        match deserialized {
            ClientRpcResponse::ClusterState(state) => {
                assert_eq!(state.nodes.len(), MAX_CLUSTER_NODES);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_empty_cluster_state() {
        let response = ClientRpcResponse::ClusterState(ClusterStateResponse {
            nodes: vec![],
            leader_id: None,
            this_node_id: 0,
        });

        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcResponse::ClusterState(state) => {
                assert!(state.nodes.is_empty());
                assert!(state.leader_id.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    // ============================================================================
    // Phase 3: AddPeer and GetClusterTicketCombined serialization tests
    // ============================================================================

    #[test]
    fn test_add_peer_request_postcard_roundtrip() {
        let request = ClientRpcRequest::AddPeer {
            node_id: 42,
            endpoint_addr: r#"{"id":"test_endpoint_id","addrs":[]}"#.to_string(),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcRequest::AddPeer { node_id, endpoint_addr } => {
                assert_eq!(node_id, 42);
                assert!(endpoint_addr.contains("test_endpoint_id"));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_get_cluster_ticket_combined_request_no_ids() {
        let request = ClientRpcRequest::GetClusterTicketCombined { endpoint_ids: None };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
                assert!(endpoint_ids.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_get_cluster_ticket_combined_request_with_ids() {
        let ids = "abc123,def456,ghi789".to_string();
        let request = ClientRpcRequest::GetClusterTicketCombined {
            endpoint_ids: Some(ids.clone()),
        };
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
                assert_eq!(endpoint_ids, Some(ids));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_add_peer_result_response_success() {
        use aspen::client_rpc::AddPeerResultResponse;

        let response = ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
            success: true,
            error: None,
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcResponse::AddPeerResult(r) => {
                assert!(r.success);
                assert!(r.error.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_add_peer_result_response_failure() {
        use aspen::client_rpc::AddPeerResultResponse;

        let response = ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
            success: false,
            error: Some("invalid endpoint_addr format".to_string()),
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcResponse::AddPeerResult(r) => {
                assert!(!r.success);
                assert!(r.error.as_ref().unwrap().contains("invalid"));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cluster_ticket_response_with_multiple_bootstrap_peers() {
        let response = ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
            ticket: "aspen_multi_peer_ticket_data".to_string(),
            topic_id: "topic_abc123".to_string(),
            cluster_id: "test-cluster".to_string(),
            endpoint_id: "local_endpoint_id".to_string(),
            bootstrap_peers: Some(5), // Multiple bootstrap peers
        });
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");

        match deserialized {
            ClientRpcResponse::ClusterTicket(t) => {
                assert_eq!(t.bootstrap_peers, Some(5));
                assert_eq!(t.cluster_id, "test-cluster");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
