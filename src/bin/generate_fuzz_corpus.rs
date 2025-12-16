//! Corpus generator for fuzz targets.
//!
//! Run with: cargo run --bin generate_fuzz_corpus --features fuzzing

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use aspen::api::{
    ChangeMembershipRequest, ClusterNode, DeleteRequest, InitRequest, ReadRequest, ScanRequest,
    WriteCommand, WriteRequest,
};
use aspen::cluster::ticket::AspenClusterTicket;
use aspen::tui_rpc::{TuiRpcRequest, TuiRpcResponse};
use iroh_gossip::proto::TopicId;

fn main() {
    let corpus_dir = Path::new("fuzz/corpus");

    generate_tui_rpc_corpus(corpus_dir);
    generate_raft_rpc_corpus(corpus_dir);
    generate_http_api_corpus(corpus_dir);
    generate_config_corpus(corpus_dir);
    generate_ticket_corpus(corpus_dir);
    generate_integrity_corpus(corpus_dir);
    generate_clock_drift_corpus(corpus_dir);
    generate_metadata_corpus(corpus_dir);

    println!("Corpus generation complete!");
}

fn generate_tui_rpc_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_tui_rpc");
    fs::create_dir_all(&dir).unwrap();

    let requests: Vec<(&str, TuiRpcRequest)> = vec![
        ("get_health", TuiRpcRequest::GetHealth),
        ("get_raft_metrics", TuiRpcRequest::GetRaftMetrics),
        ("get_leader", TuiRpcRequest::GetLeader),
        ("get_node_info", TuiRpcRequest::GetNodeInfo),
        ("get_cluster_ticket", TuiRpcRequest::GetClusterTicket),
        ("init_cluster", TuiRpcRequest::InitCluster),
        (
            "read_key",
            TuiRpcRequest::ReadKey {
                key: "test_key".to_string(),
            },
        ),
        (
            "write_key",
            TuiRpcRequest::WriteKey {
                key: "test_key".to_string(),
                value: b"test_value".to_vec(),
            },
        ),
        ("trigger_snapshot", TuiRpcRequest::TriggerSnapshot),
        (
            "add_learner",
            TuiRpcRequest::AddLearner {
                node_id: 2,
                addr: "127.0.0.1:5302".to_string(),
            },
        ),
        (
            "change_membership",
            TuiRpcRequest::ChangeMembership {
                members: vec![1, 2, 3],
            },
        ),
        ("ping", TuiRpcRequest::Ping),
        ("get_cluster_state", TuiRpcRequest::GetClusterState),
    ];

    for (name, req) in requests {
        if let Ok(bytes) = postcard::to_stdvec(&req) {
            fs::write(dir.join(name), &bytes).unwrap();
            println!("Created: {}/{}", dir.display(), name);
        }
    }

    // Responses
    let responses: Vec<(&str, TuiRpcResponse)> = vec![
        ("resp_pong", TuiRpcResponse::Pong),
        ("resp_leader_some", TuiRpcResponse::Leader(Some(1))),
        ("resp_leader_none", TuiRpcResponse::Leader(None)),
    ];

    for (name, resp) in responses {
        if let Ok(bytes) = postcard::to_stdvec(&resp) {
            fs::write(dir.join(name), &bytes).unwrap();
            println!("Created: {}/{}", dir.display(), name);
        }
    }
}

fn generate_raft_rpc_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_raft_rpc");
    fs::create_dir_all(&dir).unwrap();

    // Generate raw byte patterns that might parse as valid Raft messages
    // Since RaftVoteRequest wraps VoteRequest<AppTypeConfig>, we generate
    // byte patterns that could be valid postcard-encoded messages
    let test_patterns: Vec<(&str, Vec<u8>)> = vec![
        // Empty
        ("empty", vec![]),
        // Small patterns
        ("single_zero", vec![0]),
        ("single_one", vec![1]),
        // Varint patterns (postcard uses varint encoding)
        ("varint_small", vec![0x01]),
        ("varint_medium", vec![0x80, 0x01]),
        ("varint_large", vec![0x80, 0x80, 0x01]),
        // Mixed patterns
        ("pattern_1", vec![0, 0, 0, 0]),
        ("pattern_2", vec![1, 0, 0, 0]),
        ("pattern_3", vec![0, 1, 0, 0]),
        // Longer patterns for message structures
        (
            "longer_1",
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        ),
        (
            "longer_2",
            vec![
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            ],
        ),
    ];

    for (name, bytes) in test_patterns {
        fs::write(dir.join(name), &bytes).unwrap();
        println!("Created: {}/{}", dir.display(), name);
    }
}

fn generate_http_api_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_http_api");
    fs::create_dir_all(&dir).unwrap();

    let requests: Vec<(&str, String)> = vec![
        (
            "init_request",
            serde_json::to_string(&InitRequest {
                initial_members: vec![],
            })
            .unwrap(),
        ),
        (
            "add_learner",
            serde_json::to_string(&aspen::api::AddLearnerRequest {
                learner: ClusterNode {
                    id: 2,
                    addr: "127.0.0.1:5302".to_string(),
                    raft_addr: Some("127.0.0.1:5302".to_string()),
                    iroh_addr: None,
                },
            })
            .unwrap(),
        ),
        (
            "change_membership",
            serde_json::to_string(&ChangeMembershipRequest {
                members: vec![1, 2, 3],
            })
            .unwrap(),
        ),
        (
            "write_request_set",
            serde_json::to_string(&WriteRequest {
                command: WriteCommand::Set {
                    key: "test".to_string(),
                    value: "value".to_string(),
                },
            })
            .unwrap(),
        ),
        (
            "write_request_delete",
            serde_json::to_string(&WriteRequest {
                command: WriteCommand::Delete {
                    key: "test".to_string(),
                },
            })
            .unwrap(),
        ),
        (
            "read_request",
            serde_json::to_string(&ReadRequest {
                key: "test".to_string(),
            })
            .unwrap(),
        ),
        (
            "delete_request",
            serde_json::to_string(&DeleteRequest {
                key: "test".to_string(),
            })
            .unwrap(),
        ),
        (
            "scan_request",
            serde_json::to_string(&ScanRequest {
                prefix: "test".to_string(),
                limit: Some(100),
                continuation_token: None,
            })
            .unwrap(),
        ),
    ];

    for (name, json) in requests {
        fs::write(dir.join(name), json.as_bytes()).unwrap();
        println!("Created: {}/{}", dir.display(), name);
    }
}

fn generate_config_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_config");
    fs::create_dir_all(&dir).unwrap();

    let configs = vec![
        ("minimal", "node_id = 1\n"),
        (
            "basic",
            r#"node_id = 1
data_dir = "./data/node-1"
storage_backend = "sqlite"
"#,
        ),
        (
            "full",
            r#"node_id = 1
data_dir = "./data/node-1"
storage_backend = "sqlite"
cookie = "my-cluster-cookie"
heartbeat_interval_ms = 100
election_timeout_min_ms = 500
election_timeout_max_ms = 1000

[iroh]
bind_port = 4301
"#,
        ),
        (
            "with_secret_key",
            r#"node_id = 1

[iroh]
secret_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"#,
        ),
    ];

    for (name, config) in configs {
        fs::write(dir.join(name), config.as_bytes()).unwrap();
        println!("Created: {}/{}", dir.display(), name);
    }

    // Hex keys
    fs::write(
        dir.join("hex_valid"),
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    fs::write(dir.join("hex_short"), "0123456789abcdef").unwrap();
    fs::write(dir.join("hex_empty"), "").unwrap();
}

fn generate_ticket_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_cluster_ticket");
    fs::create_dir_all(&dir).unwrap();

    let ticket = AspenClusterTicket {
        topic_id: TopicId::from_bytes([0u8; 32]),
        bootstrap: BTreeSet::new(),
        cluster_id: "test-cluster".to_string(),
    };

    if let Ok(bytes) = postcard::to_stdvec(&ticket) {
        fs::write(dir.join("empty_ticket"), &bytes).unwrap();
        println!("Created: {}/empty_ticket", dir.display());
    }

    // Generate a ticket with a bootstrap peer
    let secret_key = iroh::SecretKey::from([42u8; 32]);
    let endpoint_id = secret_key.public();
    let mut ticket_with_peer = AspenClusterTicket {
        topic_id: TopicId::from_bytes([1u8; 32]),
        bootstrap: BTreeSet::new(),
        cluster_id: "test-cluster-with-peer".to_string(),
    };
    ticket_with_peer.bootstrap.insert(endpoint_id);

    if let Ok(bytes) = postcard::to_stdvec(&ticket_with_peer) {
        fs::write(dir.join("ticket_with_peer"), &bytes).unwrap();
        println!("Created: {}/ticket_with_peer", dir.display());
    }
}

fn generate_integrity_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_integrity");
    fs::create_dir_all(&dir).unwrap();

    // Hash seeds
    fs::write(dir.join("genesis"), vec![0u8; 32]).unwrap();
    fs::write(dir.join("random"), (0..32).collect::<Vec<u8>>()).unwrap();
    fs::write(dir.join("all_ones"), vec![0xffu8; 32]).unwrap();

    // Entry data
    fs::write(dir.join("empty_entry"), vec![]).unwrap();
    fs::write(dir.join("small_entry"), b"hello world").unwrap();
    fs::write(dir.join("medium_entry"), vec![0x42u8; 1024]).unwrap();

    println!("Created integrity corpus");
}

fn generate_clock_drift_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_clock_drift");
    fs::create_dir_all(&dir).unwrap();

    // Timestamp tuples as raw bytes
    let write_timestamps = |name: &str, ts: [u64; 5]| {
        let mut bytes = Vec::new();
        for t in ts {
            bytes.extend_from_slice(&t.to_le_bytes());
        }
        fs::write(dir.join(name), &bytes).unwrap();
    };

    write_timestamps("normal", [1000, 1001, 1002, 1003, 0]);
    write_timestamps("zero", [0, 0, 0, 0, 0]);
    write_timestamps(
        "large",
        [
            u64::MAX / 2,
            u64::MAX / 2 + 1,
            u64::MAX / 2 + 2,
            u64::MAX / 2 + 3,
            0,
        ],
    );

    println!("Created clock drift corpus");
}

fn generate_metadata_corpus(corpus_dir: &Path) {
    let dir = corpus_dir.join("fuzz_metadata");
    fs::create_dir_all(&dir).unwrap();

    #[derive(serde::Serialize)]
    struct NodeMeta {
        node_id: u64,
        endpoint_id: String,
        raft_addr: String,
        status: u8,
        last_updated_secs: u64,
    }

    let meta1 = NodeMeta {
        node_id: 1,
        endpoint_id: "abc123".to_string(),
        raft_addr: "127.0.0.1:5301".to_string(),
        status: 2,
        last_updated_secs: 1700000000,
    };

    if let Ok(bytes) = bincode::serialize(&meta1) {
        fs::write(dir.join("node1"), &bytes).unwrap();
        println!("Created: {}/node1", dir.display());
    }
}
