//! Corpus generator for fuzz targets.
//!
//! Run with: cargo run --bin generate_fuzz_corpus --features fuzzing
//!
//! Tiger Style: All operations use proper error handling with context.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterNode;
use aspen::api::DeleteRequest;
use aspen::api::InitRequest;
use aspen::api::ReadRequest;
use aspen::api::ScanRequest;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::cluster::ticket::AspenClusterTicket;
use iroh_gossip::proto::TopicId;

fn main() -> Result<()> {
    let corpus_dir = Path::new("fuzz/corpus");

    generate_client_rpc_corpus(corpus_dir)?;
    generate_raft_rpc_corpus(corpus_dir)?;
    generate_http_api_corpus(corpus_dir)?;
    generate_config_corpus(corpus_dir)?;
    generate_ticket_corpus(corpus_dir)?;
    generate_integrity_corpus(corpus_dir)?;
    generate_clock_drift_corpus(corpus_dir)?;
    generate_metadata_corpus(corpus_dir)?;

    // High-risk targets with empty corpuses
    generate_protocol_handler_corpus(corpus_dir)?;
    generate_snapshot_json_corpus(corpus_dir)?;
    generate_log_entries_corpus(corpus_dir)?;
    generate_gossip_corpus(corpus_dir)?;
    generate_differential_corpus(corpus_dir)?;
    generate_roundtrip_corpus(corpus_dir)?;

    println!("Corpus generation complete!");
    Ok(())
}

fn generate_client_rpc_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_client_rpc");
    fs::create_dir_all(&dir).context("failed to create fuzz_client_rpc directory")?;

    let requests: Vec<(&str, ClientRpcRequest)> = vec![
        ("get_health", ClientRpcRequest::GetHealth),
        ("get_raft_metrics", ClientRpcRequest::GetRaftMetrics),
        ("get_leader", ClientRpcRequest::GetLeader),
        ("get_node_info", ClientRpcRequest::GetNodeInfo),
        ("get_cluster_ticket", ClientRpcRequest::GetClusterTicket),
        ("init_cluster", ClientRpcRequest::InitCluster),
        ("read_key", ClientRpcRequest::ReadKey {
            key: "test_key".to_string(),
        }),
        ("write_key", ClientRpcRequest::WriteKey {
            key: "test_key".to_string(),
            value: b"test_value".to_vec(),
        }),
        ("trigger_snapshot", ClientRpcRequest::TriggerSnapshot),
        ("add_learner", ClientRpcRequest::AddLearner {
            node_id: 2,
            addr: "127.0.0.1:1234".to_string(),
        }),
        ("change_membership", ClientRpcRequest::ChangeMembership { members: vec![1, 2, 3] }),
        ("ping", ClientRpcRequest::Ping),
        ("get_cluster_state", ClientRpcRequest::GetClusterState),
    ];

    for (name, req) in requests {
        if let Ok(bytes) = postcard::to_stdvec(&req) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }

    // Responses
    let responses: Vec<(&str, ClientRpcResponse)> = vec![
        ("resp_pong", ClientRpcResponse::Pong),
        ("resp_leader_some", ClientRpcResponse::Leader(Some(1))),
        ("resp_leader_none", ClientRpcResponse::Leader(None)),
    ];

    for (name, resp) in responses {
        if let Ok(bytes) = postcard::to_stdvec(&resp) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }
    Ok(())
}

fn generate_raft_rpc_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_raft_rpc");
    fs::create_dir_all(&dir).context("failed to create fuzz_raft_rpc directory")?;

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
        ("longer_1", vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        ("longer_2", vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
    ];

    for (name, bytes) in test_patterns {
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_http_api_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_http_api");
    fs::create_dir_all(&dir).context("failed to create fuzz_http_api directory")?;

    let requests: Vec<(&str, String)> = vec![
        (
            "init_request",
            serde_json::to_string(&InitRequest {
                initial_members: vec![],
            })
            .context("failed to serialize init_request")?,
        ),
        (
            "add_learner",
            serde_json::to_string(&aspen::api::AddLearnerRequest {
                learner: ClusterNode {
                    id: 2,
                    addr: "127.0.0.1:5302".to_string(),
                    raft_addr: Some("127.0.0.1:5302".to_string()),
                    node_addr: None,
                },
            })
            .context("failed to serialize add_learner")?,
        ),
        (
            "change_membership",
            serde_json::to_string(&ChangeMembershipRequest { members: vec![1, 2, 3] })
                .context("failed to serialize change_membership")?,
        ),
        (
            "write_request_set",
            serde_json::to_string(&WriteRequest {
                command: WriteCommand::Set {
                    key: "test".to_string(),
                    value: "value".to_string(),
                },
            })
            .context("failed to serialize write_request_set")?,
        ),
        (
            "write_request_delete",
            serde_json::to_string(&WriteRequest {
                command: WriteCommand::Delete {
                    key: "test".to_string(),
                },
            })
            .context("failed to serialize write_request_delete")?,
        ),
        (
            "read_request",
            serde_json::to_string(&ReadRequest::new("test".to_string())).context("failed to serialize read_request")?,
        ),
        (
            "delete_request",
            serde_json::to_string(&DeleteRequest {
                key: "test".to_string(),
            })
            .context("failed to serialize delete_request")?,
        ),
        (
            "scan_request",
            serde_json::to_string(&ScanRequest {
                prefix: "test".to_string(),
                limit: Some(100),
                continuation_token: None,
            })
            .context("failed to serialize scan_request")?,
        ),
    ];

    for (name, json) in requests {
        fs::write(dir.join(name), json.as_bytes()).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_config_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_config");
    fs::create_dir_all(&dir).context("failed to create fuzz_config directory")?;

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
        fs::write(dir.join(name), config.as_bytes()).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }

    // Hex keys
    fs::write(dir.join("hex_valid"), "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        .context("failed to write hex_valid")?;
    fs::write(dir.join("hex_short"), "0123456789abcdef").context("failed to write hex_short")?;
    fs::write(dir.join("hex_empty"), "").context("failed to write hex_empty")?;
    Ok(())
}

fn generate_ticket_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_cluster_ticket");
    fs::create_dir_all(&dir).context("failed to create fuzz_cluster_ticket directory")?;

    let ticket = AspenClusterTicket {
        topic_id: TopicId::from_bytes([0u8; 32]),
        bootstrap: BTreeSet::new(),
        cluster_id: "test-cluster".to_string(),
    };

    if let Ok(bytes) = postcard::to_stdvec(&ticket) {
        fs::write(dir.join("empty_ticket"), &bytes).context("failed to write empty_ticket")?;
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
        fs::write(dir.join("ticket_with_peer"), &bytes).context("failed to write ticket_with_peer")?;
        println!("Created: {}/ticket_with_peer", dir.display());
    }
    Ok(())
}

fn generate_integrity_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_integrity");
    fs::create_dir_all(&dir).context("failed to create fuzz_integrity directory")?;

    // Hash seeds
    fs::write(dir.join("genesis"), vec![0u8; 32]).context("failed to write genesis")?;
    fs::write(dir.join("random"), (0..32).collect::<Vec<u8>>()).context("failed to write random")?;
    fs::write(dir.join("all_ones"), vec![0xffu8; 32]).context("failed to write all_ones")?;

    // Entry data
    fs::write(dir.join("empty_entry"), vec![]).context("failed to write empty_entry")?;
    fs::write(dir.join("small_entry"), b"hello world").context("failed to write small_entry")?;
    fs::write(dir.join("medium_entry"), vec![0x42u8; 1024]).context("failed to write medium_entry")?;

    println!("Created integrity corpus");
    Ok(())
}

fn generate_clock_drift_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_clock_drift");
    fs::create_dir_all(&dir).context("failed to create fuzz_clock_drift directory")?;

    // Timestamp tuples as raw bytes
    let write_timestamps = |name: &str, ts: [u64; 5]| -> Result<()> {
        let mut bytes = Vec::new();
        for t in ts {
            bytes.extend_from_slice(&t.to_le_bytes());
        }
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))
    };

    write_timestamps("normal", [1000, 1001, 1002, 1003, 0])?;
    write_timestamps("zero", [0, 0, 0, 0, 0])?;
    write_timestamps("large", [u64::MAX / 2, u64::MAX / 2 + 1, u64::MAX / 2 + 2, u64::MAX / 2 + 3, 0])?;

    println!("Created clock drift corpus");
    Ok(())
}

fn generate_metadata_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_metadata");
    fs::create_dir_all(&dir).context("failed to create fuzz_metadata directory")?;

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
        fs::write(dir.join("node1"), &bytes).context("failed to write node1")?;
        println!("Created: {}/node1", dir.display());
    }
    Ok(())
}

fn generate_protocol_handler_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_protocol_handler");
    fs::create_dir_all(&dir).context("failed to create fuzz_protocol_handler directory")?;

    // Size boundary patterns - critical for protocol handlers
    // These test size limit enforcement paths
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        // Empty and minimal
        ("empty", vec![]),
        ("single_byte", vec![0]),
        // Near Client message size limit (1 MB)
        ("client_limit_under", vec![0x42; 1024 * 1024 - 1]),
        ("client_limit_exact", vec![0x42; 1024 * 1024]),
        // Varint boundary patterns (postcard encoding)
        ("varint_1byte", vec![0x7f]),
        ("varint_2byte", vec![0x80, 0x01]),
        ("varint_3byte", vec![0x80, 0x80, 0x01]),
        ("varint_max", vec![0xff, 0xff, 0xff, 0xff, 0x0f]),
        // Valid postcard enum discriminants
        ("enum_0", vec![0x00]),
        ("enum_1", vec![0x01]),
        ("enum_2", vec![0x02]),
        ("enum_3", vec![0x03]),
        // Truncated message patterns
        ("truncated_short", vec![0x01, 0x00]),
        ("truncated_medium", vec![0x01, 0x00, 0x00, 0x00, 0x00]),
    ];

    for (name, bytes) in patterns {
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }

    // Also serialize actual Client requests for valid baseline
    let client_requests: Vec<(&str, ClientRpcRequest)> = vec![
        ("valid_ping", ClientRpcRequest::Ping),
        ("valid_get_health", ClientRpcRequest::GetHealth),
        ("valid_get_leader", ClientRpcRequest::GetLeader),
    ];

    for (name, req) in client_requests {
        if let Ok(bytes) = postcard::to_stdvec(&req) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }
    Ok(())
}

fn generate_snapshot_json_corpus(corpus_dir: &Path) -> Result<()> {
    use std::collections::BTreeMap;

    let dir = corpus_dir.join("fuzz_snapshot_json");
    fs::create_dir_all(&dir).context("failed to create fuzz_snapshot_json directory")?;

    // Valid JSON BTreeMap structures
    let empty_map: BTreeMap<String, String> = BTreeMap::new();
    fs::write(dir.join("empty_map"), serde_json::to_vec(&empty_map).context("failed to serialize empty_map")?)
        .context("failed to write empty_map")?;

    let single_key = BTreeMap::from([("key".to_string(), "value".to_string())]);
    fs::write(dir.join("single_key"), serde_json::to_vec(&single_key).context("failed to serialize single_key")?)
        .context("failed to write single_key")?;

    let multiple_keys = BTreeMap::from([
        ("key1".to_string(), "value1".to_string()),
        ("key2".to_string(), "value2".to_string()),
        ("key3".to_string(), "value3".to_string()),
    ]);
    fs::write(
        dir.join("multiple_keys"),
        serde_json::to_vec(&multiple_keys).context("failed to serialize multiple_keys")?,
    )
    .context("failed to write multiple_keys")?;

    // Edge cases for JSON parsing
    let edge_cases: Vec<(&str, &str)> = vec![
        ("json_empty_object", "{}"),
        ("json_empty_string_key", r#"{"": "value"}"#),
        ("json_empty_string_value", r#"{"key": ""}"#),
        ("json_unicode_key", r#"{"键": "value"}"#),
        ("json_unicode_value", r#"{"key": "值"}"#),
        ("json_escaped_chars", r#"{"key": "value\n\t\r"}"#),
        ("json_special_chars", r#"{"key\"": "value\\"}"#),
        // Malformed JSON - should be rejected gracefully
        ("json_unclosed_brace", "{"),
        ("json_unclosed_string", r#"{"key"#),
        ("json_array_not_object", "[]"),
        ("json_null", "null"),
        ("json_number", "123"),
        ("json_string", r#""string""#),
        // Deeply nested (stress test)
        ("json_deep_nested", r#"{"a":{"b":{"c":{"d":"e"}}}}"#),
    ];

    for (name, json) in edge_cases {
        fs::write(dir.join(name), json.as_bytes()).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_log_entries_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_log_entries");
    fs::create_dir_all(&dir).context("failed to create fuzz_log_entries directory")?;

    // Raw byte patterns for bincode parsing
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        // Empty
        ("empty", vec![]),
        // Minimal bincode
        ("single_zero", vec![0]),
        ("single_one", vec![1]),
        // Common bincode patterns (length-prefixed data)
        ("bincode_empty_vec", vec![0, 0, 0, 0, 0, 0, 0, 0]),
        ("bincode_small_vec", vec![1, 0, 0, 0, 0, 0, 0, 0, 42]),
        // u64 patterns (log index, term)
        ("u64_zero", vec![0, 0, 0, 0, 0, 0, 0, 0]),
        ("u64_one", vec![1, 0, 0, 0, 0, 0, 0, 0]),
        ("u64_max", vec![0xff; 8]),
        // Multiple u64s (typical log entry header)
        ("log_entry_header", vec![1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]),
        // Truncated patterns
        ("truncated_1", vec![0]),
        ("truncated_4", vec![0, 0, 0, 0]),
        ("truncated_7", vec![0, 0, 0, 0, 0, 0, 0]),
        // Larger patterns
        ("medium_entry", (0..64).collect::<Vec<u8>>()),
    ];

    for (name, bytes) in patterns {
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_gossip_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_gossip");
    fs::create_dir_all(&dir).context("failed to create fuzz_gossip directory")?;

    // Simplified peer announcement structure for fuzzing
    #[derive(serde::Serialize)]
    struct FuzzPeerAnnouncement {
        node_id: u64,
        endpoint_addr_bytes: Vec<u8>,
        timestamp_micros: u64,
    }

    // Valid announcements
    let announcements: Vec<(&str, FuzzPeerAnnouncement)> = vec![
        ("valid_node_1", FuzzPeerAnnouncement {
            node_id: 1,
            endpoint_addr_bytes: vec![127, 0, 0, 1, 0x15, 0xb3], // 127.0.0.1:5555
            timestamp_micros: 1700000000000000,
        }),
        ("valid_node_max", FuzzPeerAnnouncement {
            node_id: u64::MAX,
            endpoint_addr_bytes: vec![0, 0, 0, 0, 0, 0],
            timestamp_micros: 0,
        }),
        ("empty_addr", FuzzPeerAnnouncement {
            node_id: 1,
            endpoint_addr_bytes: vec![],
            timestamp_micros: 1000,
        }),
    ];

    for (name, announcement) in announcements {
        if let Ok(bytes) = postcard::to_stdvec(&announcement) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }

    // Raw NodeId patterns
    let node_id_patterns: Vec<(&str, Vec<u8>)> = vec![
        ("node_id_zero", 0u64.to_le_bytes().to_vec()),
        ("node_id_one", 1u64.to_le_bytes().to_vec()),
        ("node_id_max", u64::MAX.to_le_bytes().to_vec()),
    ];

    for (name, bytes) in node_id_patterns {
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_differential_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_differential");
    fs::create_dir_all(&dir).context("failed to create fuzz_differential directory")?;

    // Operation sequences for differential testing
    // We generate arbitrary-style input data

    // The fuzz target uses #[derive(Arbitrary)] so we generate raw bytes
    // that can be interpreted as operation sequences
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        // Empty operation sequence
        ("empty", vec![0, 0, 0, 0]),
        // Single Set operation (enum variant 0)
        ("single_set", vec![
            1, 0, 0, 0, // 1 operation
            0, // Set variant
            4, // key length
            b't', b'e', b's', b't', // "test"
            5, 0, 0, 0, // value length
            b'v', b'a', b'l', b'u', b'e', // "value"
        ]),
        // Single Delete operation (enum variant 1)
        ("single_delete", vec![
            1, 0, 0, 0, // 1 operation
            1, // Delete variant
            4, // key length
            b't', b'e', b's', b't', // "test"
        ]),
        // Multiple operations
        ("multi_ops", vec![3, 0, 0, 0, 0, 2, b'a', b'b', 2, 0, 0, 0, b'c', b'd']),
    ];

    for (name, bytes) in patterns {
        fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
        println!("Created: {}/{}", dir.display(), name);
    }
    Ok(())
}

fn generate_roundtrip_corpus(corpus_dir: &Path) -> Result<()> {
    let dir = corpus_dir.join("fuzz_roundtrip");
    fs::create_dir_all(&dir).context("failed to create fuzz_roundtrip directory")?;

    // Serialize valid messages for roundtrip testing
    // Client RPC messages
    let client_requests: Vec<(&str, ClientRpcRequest)> = vec![
        ("client_ping", ClientRpcRequest::Ping),
        ("client_get_health", ClientRpcRequest::GetHealth),
        ("client_get_leader", ClientRpcRequest::GetLeader),
        ("client_init_cluster", ClientRpcRequest::InitCluster),
        ("client_read_key", ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }),
    ];

    for (name, req) in client_requests {
        if let Ok(bytes) = postcard::to_stdvec(&req) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }

    let client_responses: Vec<(&str, ClientRpcResponse)> = vec![
        ("client_resp_pong", ClientRpcResponse::Pong),
        ("client_resp_leader_some", ClientRpcResponse::Leader(Some(1))),
        ("client_resp_leader_none", ClientRpcResponse::Leader(None)),
    ];

    for (name, resp) in client_responses {
        if let Ok(bytes) = postcard::to_stdvec(&resp) {
            fs::write(dir.join(name), &bytes).with_context(|| format!("failed to write {}", name))?;
            println!("Created: {}/{}", dir.display(), name);
        }
    }

    // Cluster ticket for roundtrip
    let ticket = AspenClusterTicket {
        topic_id: TopicId::from_bytes([0u8; 32]),
        bootstrap: BTreeSet::new(),
        cluster_id: "roundtrip-test".to_string(),
    };

    if let Ok(bytes) = postcard::to_stdvec(&ticket) {
        fs::write(dir.join("cluster_ticket"), &bytes).context("failed to write cluster_ticket")?;
        println!("Created: {}/cluster_ticket", dir.display());
    }
    Ok(())
}
