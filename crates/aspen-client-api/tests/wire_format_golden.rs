//! Comprehensive postcard wire format stability tests.
//!
//! These tests pin the postcard discriminant of every variant in
//! `ClientRpcRequest` and `ClientRpcResponse` via golden snapshot files.
//! Any insertion, removal, or reordering of variants causes a test failure.
//!
//! Background: postcard encodes enum variants as varint discriminants (0, 1, 2, ...).
//! If a variant is inserted in the middle, ALL subsequent discriminants shift,
//! silently breaking wire compatibility between CLI/server/TUI built at
//! different commits.
//!
//! See napkin entry 2026-02-26: feature-gated variants shifted discriminants
//! and caused deserialization crashes ("Found a bool that wasn't 0 or 1").

use std::collections::HashMap;

use aspen_client_api::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decode a postcard varint from the front of `bytes`.
/// Returns (value, bytes_consumed).
fn decode_varint(bytes: &[u8]) -> (u32, usize) {
    let mut result: u32 = 0;
    let mut shift = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return (result, i + 1);
        }
        shift += 7;
    }
    panic!("unterminated varint");
}

/// Serialize a value with postcard and return the discriminant varint.
fn discriminant_of<T: serde::Serialize>(val: &T) -> u32 {
    let bytes = postcard::to_stdvec(val).expect("postcard serialize");
    let (disc, _) = decode_varint(&bytes);
    disc
}

/// Load a golden discriminant file and return Vec<(discriminant, name)>.
fn load_golden(filename: &str) -> Vec<(u32, String)> {
    let path = format!("{}/tests/golden/{filename}", env!("CARGO_MANIFEST_DIR"));
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));

    content
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let mut parts = line.splitn(2, ' ');
            let disc: u32 = parts.next().expect("discriminant").parse().expect("parse discriminant");
            let name = parts.next().expect("variant name").to_string();
            (disc, name)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Variant count tests
// ---------------------------------------------------------------------------

/// Assert exact variant counts. If someone adds or removes a variant,
/// they must update both the count here AND the golden file.
#[test]
fn test_request_variant_count() {
    let golden = load_golden("request_discriminants.txt");
    assert_eq!(
        golden.len(),
        337,
        "ClientRpcRequest variant count changed! \
         Expected 337, golden file has {}. \
         Update the golden file AND this count.",
        golden.len()
    );
}

#[test]
fn test_response_variant_count() {
    let golden = load_golden("response_discriminants.txt");
    assert_eq!(
        golden.len(),
        270,
        "ClientRpcResponse variant count changed! \
         Expected 270, golden file has {}. \
         Update the golden file AND this count.",
        golden.len()
    );
}

// ---------------------------------------------------------------------------
// Request discriminant pinning
//
// We construct a representative variant from each section boundary and
// verify its postcard discriminant matches the golden file.
// ---------------------------------------------------------------------------

/// Build a map of variant_name → expected_discriminant from the golden file.
fn golden_map(filename: &str) -> HashMap<String, u32> {
    load_golden(filename).into_iter().map(|(disc, name)| (name, disc)).collect()
}

/// Assert that a constructed variant matches its golden discriminant.
fn assert_request_disc(map: &HashMap<String, u32>, req: &ClientRpcRequest) {
    let name = req.variant_name();
    let expected = map
        .get(name)
        .unwrap_or_else(|| panic!("Variant {name} not in golden file — did you forget to add it?"));
    let actual = discriminant_of(req);
    assert_eq!(
        actual, *expected,
        "WIRE FORMAT BREAK: {name} discriminant is {actual}, golden says {expected}. \
         A variant was inserted, removed, or reordered."
    );
}

fn assert_response_disc(map: &HashMap<String, u32>, resp: &ClientRpcResponse) {
    let actual = discriminant_of(resp);
    // We need the variant name for the error message. Match on a few key patterns.
    let name = response_variant_name(resp);
    let expected = map
        .get(name)
        .unwrap_or_else(|| panic!("Variant {name} not in golden file — did you forget to add it?"));
    assert_eq!(
        actual, *expected,
        "WIRE FORMAT BREAK: {name} discriminant is {actual}, golden says {expected}. \
         A variant was inserted, removed, or reordered."
    );
}

/// Extract variant name from a response (no variant_name() method on response).
fn response_variant_name(resp: &ClientRpcResponse) -> &'static str {
    // Use postcard discriminant to look up name. We serialize, get disc,
    // then return the name from our known mapping.
    //
    // Since we can't match on all 267 variants here without duplicating
    // the enum, we use a trick: we know the discriminant from serialization,
    // and the golden file maps disc→name. But we need the name BEFORE
    // looking it up. So we do the matching for the variants we test.
    match resp {
        ClientRpcResponse::Health(_) => "Health",
        ClientRpcResponse::RaftMetrics(_) => "RaftMetrics",
        ClientRpcResponse::Leader(_) => "Leader",
        ClientRpcResponse::NodeInfo(_) => "NodeInfo",
        ClientRpcResponse::ClusterTicket(_) => "ClusterTicket",
        ClientRpcResponse::InitResult(_) => "InitResult",
        ClientRpcResponse::ReadResult(_) => "ReadResult",
        ClientRpcResponse::WriteResult(_) => "WriteResult",
        ClientRpcResponse::Pong => "Pong",
        ClientRpcResponse::ClusterState(_) => "ClusterState",
        ClientRpcResponse::Error(_) => "Error",
        ClientRpcResponse::DeleteResult(_) => "DeleteResult",
        ClientRpcResponse::ScanResult(_) => "ScanResult",
        ClientRpcResponse::AddBlobResult(_) => "AddBlobResult",
        ClientRpcResponse::DocsSetResult(_) => "DocsSetResult",
        ClientRpcResponse::AddPeerClusterResult(_) => "AddPeerClusterResult",
        ClientRpcResponse::SqlResult(_) => "SqlResult",
        ClientRpcResponse::LockResult(_) => "LockResult",
        ClientRpcResponse::CounterResult(_) => "CounterResult",
        ClientRpcResponse::BatchReadResult(_) => "BatchReadResult",
        ClientRpcResponse::WatchCreateResult(_) => "WatchCreateResult",
        ClientRpcResponse::LeaseGrantResult(_) => "LeaseGrantResult",
        ClientRpcResponse::BarrierEnterResult(_) => "BarrierEnterResult",
        ClientRpcResponse::SemaphoreAcquireResult(_) => "SemaphoreAcquireResult",
        ClientRpcResponse::RWLockAcquireReadResult(_) => "RWLockAcquireReadResult",
        ClientRpcResponse::QueueCreateResult(_) => "QueueCreateResult",
        ClientRpcResponse::ServiceRegisterResult(_) => "ServiceRegisterResult",
        ClientRpcResponse::TopologyResult(_) => "TopologyResult",
        ClientRpcResponse::ForgeRepoResult(_) => "ForgeRepoResult",
        ClientRpcResponse::FederationStatus(_) => "FederationStatus",
        ClientRpcResponse::GitBridgeListRefs(_) => "GitBridgeListRefs",
        ClientRpcResponse::GitBridgeFetchStart(_) => "GitBridgeFetchStart",
        ClientRpcResponse::GitBridgeFetchChunk(_) => "GitBridgeFetchChunk",
        ClientRpcResponse::GitBridgeFetchComplete(_) => "GitBridgeFetchComplete",
        ClientRpcResponse::JobSubmitResult(_) => "JobSubmitResult",
        ClientRpcResponse::HookListResult(_) => "HookListResult",
        ClientRpcResponse::CiTriggerPipelineResult(_) => "CiTriggerPipelineResult",
        ClientRpcResponse::CacheQueryResult(_) => "CacheQueryResult",
        ClientRpcResponse::CacheMigrationStartResult(_) => "CacheMigrationStartResult",
        ClientRpcResponse::SecretsKvReadResult(_) => "SecretsKvReadResult",
        ClientRpcResponse::SnixDirectoryGetResult(_) => "SnixDirectoryGetResult",
        ClientRpcResponse::WorkerPollJobsResult(_) => "WorkerPollJobsResult",
        ClientRpcResponse::TraceIngestResult(_) => "TraceIngestResult",
        ClientRpcResponse::MetricIngestResult(_) => "MetricIngestResult",
        ClientRpcResponse::AlertCreateResult(_) => "AlertCreateResult",
        ClientRpcResponse::IndexCreateResult(_) => "IndexCreateResult",
        ClientRpcResponse::ClusterDeployResult(_) => "ClusterDeployResult",
        ClientRpcResponse::CapabilityUnavailable(_) => "CapabilityUnavailable",
        ClientRpcResponse::PluginReloadResult(_) => "PluginReloadResult",
        ClientRpcResponse::NetPublishResult(_) => "NetPublishResult",
        ClientRpcResponse::ContactsBookResult(_) => "ContactsBookResult",
        ClientRpcResponse::CalendarResult(_) => "CalendarResult",
        ClientRpcResponse::AutomergeCreateResult(_) => "AutomergeCreateResult",
        ClientRpcResponse::AutomergeReceiveSyncMessageResult(_) => "AutomergeReceiveSyncMessageResult",
        ClientRpcResponse::HashCheckResult(_) => "HashCheckResult",
        _ => panic!("Add this response variant to response_variant_name()"),
    }
}

// ---------------------------------------------------------------------------
// Request discriminant golden test
// ---------------------------------------------------------------------------

/// Pin discriminants for ~80 request variants spanning every section boundary.
/// Any insertion between pinned variants shifts at least one pin, failing the test.
#[test]
fn test_request_discriminants_golden() {
    let map = golden_map("request_discriminants.txt");
    let s = String::new;
    let v: fn() -> Vec<u8> = Vec::new;

    // Section: core cluster operations (0-14)
    assert_request_disc(&map, &ClientRpcRequest::GetHealth);
    assert_request_disc(&map, &ClientRpcRequest::GetRaftMetrics);
    assert_request_disc(&map, &ClientRpcRequest::GetLeader);
    assert_request_disc(&map, &ClientRpcRequest::InitCluster);
    assert_request_disc(&map, &ClientRpcRequest::ReadKey { key: s() });
    assert_request_disc(&map, &ClientRpcRequest::WriteKey { key: s(), value: v() });
    assert_request_disc(&map, &ClientRpcRequest::TriggerSnapshot);
    assert_request_disc(&map, &ClientRpcRequest::Ping);
    assert_request_disc(&map, &ClientRpcRequest::GetClusterState);

    // Section: migrated HTTP operations (15-25)
    assert_request_disc(&map, &ClientRpcRequest::DeleteKey { key: s() });
    assert_request_disc(&map, &ClientRpcRequest::ScanKeys {
        prefix: s(),
        limit: None,
        continuation_token: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::GetMetrics);
    assert_request_disc(&map, &ClientRpcRequest::CheckpointWal);
    assert_request_disc(&map, &ClientRpcRequest::ListVaults);
    assert_request_disc(&map, &ClientRpcRequest::GetClientTicket {
        access: s(),
        priority: 0,
    });

    // Section: blob operations (26-41)
    assert_request_disc(&map, &ClientRpcRequest::AddBlob { data: v(), tag: None });
    assert_request_disc(&map, &ClientRpcRequest::GetBlob { hash: s() });
    assert_request_disc(&map, &ClientRpcRequest::RunBlobRepairCycle);

    // Section: docs operations (42-46)
    assert_request_disc(&map, &ClientRpcRequest::DocsSet { key: s(), value: v() });
    assert_request_disc(&map, &ClientRpcRequest::DocsStatus);

    // Section: peer cluster (47-54)
    assert_request_disc(&map, &ClientRpcRequest::AddPeerCluster { ticket: s() });
    assert_request_disc(&map, &ClientRpcRequest::GetKeyOrigin { key: s() });

    // Section: SQL (55)
    assert_request_disc(&map, &ClientRpcRequest::ExecuteSql {
        query: s(),
        params: s(),
        consistency: s(),
        limit: None,
        timeout_ms: None,
    });

    // Section: coordination - locks (56-59)
    assert_request_disc(&map, &ClientRpcRequest::LockAcquire {
        key: s(),
        holder_id: s(),
        ttl_ms: 0,
        timeout_ms: 0,
    });
    assert_request_disc(&map, &ClientRpcRequest::LockRenew {
        key: s(),
        holder_id: s(),
        fencing_token: 0,
        ttl_ms: 0,
    });

    // Section: counters (60-68)
    assert_request_disc(&map, &ClientRpcRequest::CounterGet { key: s() });
    assert_request_disc(&map, &ClientRpcRequest::CounterIncrement { key: s() });
    assert_request_disc(&map, &ClientRpcRequest::SignedCounterAdd { key: s(), amount: 0 });

    // Section: sequences (69-71)
    assert_request_disc(&map, &ClientRpcRequest::SequenceNext { key: s() });

    // Section: rate limiter (72-75)
    assert_request_disc(&map, &ClientRpcRequest::RateLimiterTryAcquire {
        key: s(),
        tokens: 0,
        capacity_tokens: 0,
        refill_rate: 0.0,
    });

    // Section: batch (76-78)
    assert_request_disc(&map, &ClientRpcRequest::BatchRead { keys: Vec::new() });

    // Section: watch (79-81)
    assert_request_disc(&map, &ClientRpcRequest::WatchCreate {
        prefix: s(),
        start_index: 0,
        should_include_prev_value: false,
    });

    // Section: lease (82-87)
    assert_request_disc(&map, &ClientRpcRequest::LeaseGrant {
        ttl_seconds: 0,
        lease_id: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::WriteKeyWithLease {
        key: s(),
        value: v(),
        lease_id: 0,
    });

    // Section: barrier (88-90)
    assert_request_disc(&map, &ClientRpcRequest::BarrierEnter {
        name: s(),
        participant_id: s(),
        required_count: 0,
        timeout_ms: 0,
    });

    // Section: semaphore (91-94)
    assert_request_disc(&map, &ClientRpcRequest::SemaphoreAcquire {
        name: s(),
        holder_id: s(),
        permits: 0,
        capacity_permits: 0,
        ttl_ms: 0,
        timeout_ms: 0,
    });

    // Section: rwlock (95-102)
    assert_request_disc(&map, &ClientRpcRequest::RWLockAcquireRead {
        name: s(),
        holder_id: s(),
        ttl_ms: 0,
        timeout_ms: 0,
    });
    assert_request_disc(&map, &ClientRpcRequest::RWLockStatus { name: s() });

    // Section: queue (103-115)
    assert_request_disc(&map, &ClientRpcRequest::QueueCreate {
        queue_name: s(),
        default_visibility_timeout_ms: None,
        default_ttl_ms: None,
        max_delivery_attempts: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::QueueRedriveDLQ {
        queue_name: s(),
        item_id: 0,
    });

    // Section: service registry (116-123)
    assert_request_disc(&map, &ClientRpcRequest::ServiceRegister {
        service_name: s(),
        instance_id: s(),
        address: s(),
        version: s(),
        tags: s(),
        weight: 0,
        custom_metadata: s(),
        ttl_ms: 0,
        lease_id: None,
    });

    // Section: topology (124)
    assert_request_disc(&map, &ClientRpcRequest::GetTopology { client_version: None });

    // Section: forge (125-167)
    assert_request_disc(&map, &ClientRpcRequest::ForgeCreateRepo {
        name: s(),
        description: None,
        default_branch: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::ForgeGetRepo { repo_id: s() });
    assert_request_disc(&map, &ClientRpcRequest::ForgeListBranches { repo_id: s() });
    assert_request_disc(&map, &ClientRpcRequest::ForgeCreateDiscussion {
        repo_id: s(),
        title: s(),
        body: s(),
        labels: Vec::new(),
    });
    assert_request_disc(&map, &ClientRpcRequest::ForgeGetDelegateKey { repo_id: s() });

    // Section: federation (168-184)
    assert_request_disc(&map, &ClientRpcRequest::GetFederationStatus);
    assert_request_disc(&map, &ClientRpcRequest::ListFederatedRepositories);
    assert_request_disc(&map, &ClientRpcRequest::FederationGrant {
        audience: s(),
        capabilities: s(),
        lifetime_secs: 0,
    });
    assert_request_disc(&map, &ClientRpcRequest::FederationGitListRefs {
        origin_key: s(),
        repo_id: s(),
        origin_addr_hint: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::FederationGitFetch {
        origin_key: s(),
        repo_id: s(),
        want: Vec::new(),
        have: Vec::new(),
        origin_addr_hint: None,
    });

    // Section: git bridge (185-191)
    assert_request_disc(&map, &ClientRpcRequest::GitBridgeListRefs { repo_id: s() });
    assert_request_disc(&map, &ClientRpcRequest::GitBridgeProbeObjects {
        repo_id: s(),
        sha1s: Vec::new(),
    });
    assert_request_disc(&map, &ClientRpcRequest::GitBridgeFetchStart {
        repo_id: s(),
        want: Vec::new(),
        have: Vec::new(),
    });
    assert_request_disc(&map, &ClientRpcRequest::GitBridgeFetchChunk {
        session_id: s(),
        chunk_id: 0,
    });
    assert_request_disc(&map, &ClientRpcRequest::GitBridgeFetchComplete { session_id: s() });

    // Section: jobs (192-201)
    assert_request_disc(&map, &ClientRpcRequest::JobSubmit {
        job_type: s(),
        payload: s(),
        priority: None,
        timeout_ms: None,
        max_retries: None,
        retry_delay_ms: None,
        schedule: None,
        tags: Vec::new(),
    });
    assert_request_disc(&map, &ClientRpcRequest::WorkerDeregister { worker_id: s() });

    // Section: hooks (202-204)
    assert_request_disc(&map, &ClientRpcRequest::HookList);

    // Section: CI (205-216)
    assert_request_disc(&map, &ClientRpcRequest::CiTriggerPipeline {
        repo_id: s(),
        ref_name: s(),
        commit_hash: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::CiGetJobOutput {
        run_id: s(),
        job_id: s(),
    });

    // Section: secrets (217-249)
    assert_request_disc(&map, &ClientRpcRequest::SecretsKvRead {
        mount: s(),
        path: s(),
        version: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::SecretsPkiGenerateRoot {
        mount: s(),
        common_name: s(),
        ttl_days: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::SecretsNixCacheListKeys { mount: s() });

    // Section: automerge [feature-gated] (250-260)
    assert_request_disc(&map, &ClientRpcRequest::AutomergeCreate {
        document_id: None,
        namespace: None,
        title: None,
        description: None,
        tags: Vec::new(),
    });
    assert_request_disc(&map, &ClientRpcRequest::AutomergeReceiveSyncMessage {
        document_id: s(),
        peer_id: s(),
        message: s(),
        sync_state: None,
    });

    // Section: nix cache (261-264)
    assert_request_disc(&map, &ClientRpcRequest::CacheQuery { store_hash: s() });
    assert_request_disc(&map, &ClientRpcRequest::NixCacheGetPublicKey);

    // Section: cache migration [feature-gated] (265-268)
    assert_request_disc(&map, &ClientRpcRequest::CacheMigrationStart {
        batch_size: None,
        batch_delay_ms: None,
        dry_run: false,
    });
    assert_request_disc(&map, &ClientRpcRequest::CacheMigrationValidate { max_report: None });

    // Section: snix (269-272)
    assert_request_disc(&map, &ClientRpcRequest::SnixDirectoryGet { digest: s() });
    assert_request_disc(&map, &ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes: s() });

    // Section: worker polling (273-274)
    assert_request_disc(&map, &ClientRpcRequest::WorkerPollJobs {
        worker_id: s(),
        job_types: Vec::new(),
        max_jobs: 0,
        visibility_timeout_secs: 0,
    });

    // Section: observability (275-290)
    assert_request_disc(&map, &ClientRpcRequest::TraceIngest { spans: Vec::new() });
    assert_request_disc(&map, &ClientRpcRequest::MetricIngest {
        data_points: Vec::new(),
        ttl_seconds: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::IndexList);

    // Section: net (291-294)
    assert_request_disc(&map, &ClientRpcRequest::NetPublish {
        name: s(),
        endpoint_id: s(),
        port: 0,
        proto: s(),
        tags: Vec::new(),
    });

    // Section: contacts (295-310)
    assert_request_disc(&map, &ClientRpcRequest::ContactsCreateBook {
        name: s(),
        description: None,
    });

    // Section: calendar (311-323)
    assert_request_disc(&map, &ClientRpcRequest::CalendarCreate {
        name: s(),
        color: None,
        timezone: None,
        description: None,
    });

    // Section: deploy (324-328)
    assert_request_disc(&map, &ClientRpcRequest::ClusterDeploy {
        artifact: s(),
        strategy: "rolling".into(),
        max_concurrent: 1,
        health_timeout_secs: 120,
        expected_binary: None,
    });
    assert_request_disc(&map, &ClientRpcRequest::NodeRollback { deploy_id: s() });

    // Section: plugin (329)
    assert_request_disc(&map, &ClientRpcRequest::PluginReload { name: None });

    // Section: nostr (330-332)
    assert_request_disc(&map, &ClientRpcRequest::NostrAuthChallenge { npub_hex: s() });

    // Section: hash check (333) — LAST variant
    assert_request_disc(&map, &ClientRpcRequest::HashCheck {
        key: s(),
        expected_hash: [0u8; 32],
    });
}

// ---------------------------------------------------------------------------
// Response discriminant golden test
// ---------------------------------------------------------------------------

/// Pin discriminants for ~50 response variants spanning every section.
#[test]
fn test_response_discriminants_golden() {
    let map = golden_map("response_discriminants.txt");

    // Section: core (0-14) — the most critical for CLI compatibility
    assert_response_disc(
        &map,
        &ClientRpcResponse::Health(HealthResponse {
            status: String::new(),
            node_id: 0,
            raft_node_id: None,
            uptime_seconds: 0,
            is_initialized: false,
            membership_node_count: None,
            iroh_node_id: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::ReadResult(ReadResultResponse {
            value: None,
            was_found: false,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: None,
        }),
    );
    assert_response_disc(&map, &ClientRpcResponse::Pong);
    assert_response_disc(
        &map,
        &ClientRpcResponse::ClusterState(ClusterStateResponse {
            nodes: Vec::new(),
            leader_id: None,
            this_node_id: 0,
        }),
    );
    assert_response_disc(&map, &ClientRpcResponse::error("X", "X"));

    // Section: migrated HTTP responses (15-24)
    assert_response_disc(
        &map,
        &ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key: String::new(),
            was_deleted: false,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::ScanResult(ScanResultResponse {
            entries: Vec::new(),
            count: 0,
            is_truncated: false,
            continuation_token: None,
            error: None,
        }),
    );

    // Section: blob responses (25-40)
    assert_response_disc(
        &map,
        &ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
            is_success: false,
            hash: None,
            size_bytes: None,
            was_new: None,
            error: None,
        }),
    );

    // Section: docs responses (41-45)
    assert_response_disc(
        &map,
        &ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            is_success: false,
            key: None,
            size_bytes: None,
            error: None,
        }),
    );

    // Section: peer cluster responses (46-53)
    assert_response_disc(
        &map,
        &ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
            is_success: false,
            cluster_id: None,
            priority: None,
            error: None,
        }),
    );

    // Section: SQL (54)
    assert_response_disc(
        &map,
        &ClientRpcResponse::SqlResult(SqlResultResponse {
            is_success: false,
            columns: None,
            rows: None,
            row_count: None,
            is_truncated: None,
            execution_time_ms: None,
            error: None,
        }),
    );

    // Section: coordination (55-59)
    assert_response_disc(
        &map,
        &ClientRpcResponse::LockResult(LockResultResponse {
            is_success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::CounterResult(CounterResultResponse {
            is_success: false,
            value: None,
            error: None,
        }),
    );

    // Section: batch (60-62)
    assert_response_disc(
        &map,
        &ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
            is_success: false,
            values: None,
            error: None,
        }),
    );

    // Section: watch (63-66)
    assert_response_disc(
        &map,
        &ClientRpcResponse::WatchCreateResult(WatchCreateResultResponse {
            is_success: false,
            watch_id: None,
            current_index: None,
            error: None,
        }),
    );

    // Section: lease (67-71)
    assert_response_disc(
        &map,
        &ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
            is_success: false,
            lease_id: None,
            ttl_seconds: None,
            error: None,
        }),
    );

    // Section: barrier (72-74)
    assert_response_disc(
        &map,
        &ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
            is_success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: None,
        }),
    );

    // Section: semaphore (75-78)
    assert_response_disc(
        &map,
        &ClientRpcResponse::SemaphoreAcquireResult(SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: None,
            capacity_permits: None,
            retry_after_ms: None,
            error: None,
        }),
    );

    // Section: rwlock (79-86)
    assert_response_disc(
        &map,
        &ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
            is_success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: None,
        }),
    );

    // Section: queue (87-98)
    assert_response_disc(
        &map,
        &ClientRpcResponse::QueueCreateResult(QueueCreateResultResponse {
            is_success: false,
            was_created: false,
            error: None,
        }),
    );

    // Section: service registry (99-106)
    assert_response_disc(
        &map,
        &ClientRpcResponse::ServiceRegisterResult(ServiceRegisterResultResponse {
            is_success: false,
            fencing_token: None,
            deadline_ms: None,
            error: None,
        }),
    );

    // Section: topology (107)
    assert_response_disc(
        &map,
        &ClientRpcResponse::TopologyResult(TopologyResultResponse {
            is_success: false,
            version: 0,
            was_updated: false,
            topology_data: None,
            shard_count: 0,
            error: None,
        }),
    );

    // Section: forge (108-125)
    assert_response_disc(
        &map,
        &ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: false,
            repo: None,
            error: None,
        }),
    );

    // Section: federation (126-142)
    assert_response_disc(
        &map,
        &ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled: false,
            gossip_enabled: false,
            discovered_clusters: 0,
            federated_repos: 0,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
            is_success: false,
            refs: Vec::new(),
            head: None,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::GitBridgeFetchStart(GitBridgeFetchStartResponse {
            session_id: String::new(),
            total_objects: 0,
            total_chunks: 0,
            is_success: false,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::GitBridgeFetchChunk(GitBridgeFetchChunkResponse {
            session_id: String::new(),
            chunk_id: 0,
            objects: Vec::new(),
            chunk_hash: [0u8; 32],
            is_success: false,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::GitBridgeFetchComplete(GitBridgeFetchCompleteResponse {
            session_id: String::new(),
            is_success: false,
            error: None,
        }),
    );

    // Section: jobs (150-159)
    assert_response_disc(
        &map,
        &ClientRpcResponse::JobSubmitResult(JobSubmitResultResponse {
            is_success: false,
            job_id: None,
            error: None,
        }),
    );

    // Section: hooks (160-162)
    assert_response_disc(
        &map,
        &ClientRpcResponse::HookListResult(HookListResultResponse {
            is_enabled: false,
            handlers: Vec::new(),
        }),
    );

    // Section: CI (163-174)
    assert_response_disc(
        &map,
        &ClientRpcResponse::CiTriggerPipelineResult(CiTriggerPipelineResponse {
            is_success: false,
            run_id: None,
            error: None,
        }),
    );

    // Section: cache (175-178)
    assert_response_disc(
        &map,
        &ClientRpcResponse::CacheQueryResult(CacheQueryResultResponse {
            was_found: false,
            entry: None,
            error: None,
        }),
    );

    // Section: cache migration [feature-gated] (179-182)
    assert_response_disc(
        &map,
        &ClientRpcResponse::CacheMigrationStartResult(ci::CacheMigrationStartResultResponse {
            started: false,
            status: None,
            error: None,
        }),
    );

    // Section: secrets (183-202)
    assert_response_disc(
        &map,
        &ClientRpcResponse::SecretsKvReadResult(SecretsKvReadResultResponse {
            is_success: false,
            data: None,
            metadata: None,
            error: None,
        }),
    );

    // Section: snix (203-206)
    assert_response_disc(
        &map,
        &ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
            was_found: false,
            directory_bytes: None,
            error: None,
        }),
    );

    // Section: worker polling (207-208)
    assert_response_disc(
        &map,
        &ClientRpcResponse::WorkerPollJobsResult(WorkerPollJobsResultResponse {
            is_success: false,
            worker_id: String::new(),
            jobs: Vec::new(),
            error: None,
        }),
    );

    // Section: observability (209-224)
    assert_response_disc(
        &map,
        &ClientRpcResponse::TraceIngestResult(TraceIngestResultResponse {
            is_success: false,
            accepted_count: 0,
            dropped_count: 0,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::MetricIngestResult(MetricIngestResultResponse {
            is_success: false,
            accepted_count: 0,
            dropped_count: 0,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::AlertCreateResult(AlertRuleResultResponse {
            is_success: false,
            rule_name: String::new(),
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::IndexCreateResult(IndexCreateResultResponse {
            is_success: false,
            name: String::new(),
            error: None,
        }),
    );

    // Section: deploy (225-229)
    assert_response_disc(
        &map,
        &ClientRpcResponse::ClusterDeployResult(ClusterDeployResultResponse {
            is_accepted: false,
            deploy_id: None,
            error: None,
        }),
    );

    // Section: capability + plugin (230-231)
    assert_response_disc(
        &map,
        &ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
            required_app: String::new(),
            message: String::new(),
            hints: Vec::new(),
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
            is_success: false,
            plugin_count: 0,
            error: None,
            message: String::new(),
        }),
    );

    // Section: net (235-238)
    assert_response_disc(
        &map,
        &ClientRpcResponse::NetPublishResult(net::NetPublishResponse {
            is_success: false,
            error: None,
        }),
    );

    // Section: contacts (239-246)
    assert_response_disc(
        &map,
        &ClientRpcResponse::ContactsBookResult(ContactsBookResponse {
            is_success: false,
            book_id: None,
            name: None,
            error: None,
        }),
    );

    // Section: calendar (247-254)
    assert_response_disc(
        &map,
        &ClientRpcResponse::CalendarResult(CalendarResponse {
            is_success: false,
            calendar_id: None,
            name: None,
            error: None,
        }),
    );

    // Section: automerge [feature-gated] (255-265) — LAST gated block
    assert_response_disc(
        &map,
        &ClientRpcResponse::AutomergeCreateResult(automerge::AutomergeCreateResultResponse {
            is_success: false,
            document_id: None,
            error: None,
        }),
    );
    assert_response_disc(
        &map,
        &ClientRpcResponse::AutomergeReceiveSyncMessageResult(automerge::AutomergeReceiveSyncMessageResultResponse {
            is_success: false,
            changes_applied: false,
            sync_state: None,
            error: None,
        }),
    );

    // Section: hash check (266) — LAST variant
    assert_response_disc(
        &map,
        &ClientRpcResponse::HashCheckResult(HashCheckResultResponse {
            is_changed: false,
            new_hash: None,
            error: None,
        }),
    );
}

// ---------------------------------------------------------------------------
// Structural invariant tests
// ---------------------------------------------------------------------------

/// Feature-gated request variants must appear after all non-gated variants
/// in their respective feature group. This prevents discriminant shift when
/// a feature is toggled (which shouldn't happen since features are default-on,
/// but defense in depth).
#[test]
fn test_request_feature_gated_after_non_gated_sections() {
    let map = golden_map("request_discriminants.txt");

    // Last non-gated variant before automerge block
    let last_before_automerge = map["SecretsNixCacheListKeys"];
    let first_automerge = map["AutomergeCreate"];
    assert!(
        last_before_automerge < first_automerge,
        "SecretsNixCacheListKeys ({last_before_automerge}) must be before \
         AutomergeCreate ({first_automerge})"
    );

    // Last non-gated variant before CI migration block
    let last_before_ci_migration = map["NixCacheGetPublicKey"];
    let first_ci_migration = map["CacheMigrationStart"];
    assert!(
        last_before_ci_migration < first_ci_migration,
        "NixCacheGetPublicKey ({last_before_ci_migration}) must be before \
         CacheMigrationStart ({first_ci_migration})"
    );
}

/// Feature-gated response variants follow the same ordering rule.
#[test]
fn test_response_feature_gated_ordering() {
    let map = golden_map("response_discriminants.txt");

    // CI migration comes after cache results
    let cache_public_key = map["NixCachePublicKeyResult"];
    let ci_migration_start = map["CacheMigrationStartResult"];
    assert!(
        cache_public_key < ci_migration_start,
        "NixCachePublicKeyResult ({cache_public_key}) must be before \
         CacheMigrationStartResult ({ci_migration_start})"
    );

    // Automerge comes after calendar
    let calendar_export = map["CalendarExportResult"];
    let automerge_create = map["AutomergeCreateResult"];
    assert!(
        calendar_export < automerge_create,
        "CalendarExportResult ({calendar_export}) must be before \
         AutomergeCreateResult ({automerge_create})"
    );

    // HashCheckResult is the LAST variant
    let hash_check = map["HashCheckResult"];
    let automerge_last = map["AutomergeReceiveSyncMessageResult"];
    assert!(
        hash_check > automerge_last,
        "HashCheckResult ({hash_check}) must be after all automerge variants \
         (last: {automerge_last})"
    );
}

/// The Error response discriminant (14) is used by CLI retry logic.
/// It must NEVER change.
#[test]
fn test_error_response_discriminant_is_14() {
    let disc = discriminant_of(&ClientRpcResponse::error("NOT_LEADER", "x"));
    assert_eq!(
        disc, 14,
        "Error response discriminant changed from 14 to {disc}! \
         The CLI retry loop depends on this. \
         Do NOT insert variants before Error."
    );
}

/// The Pong response discriminant (12) is used by health checks.
#[test]
fn test_pong_response_discriminant_is_12() {
    let disc = discriminant_of(&ClientRpcResponse::Pong);
    assert_eq!(disc, 12, "Pong response discriminant changed from 12 to {disc}!");
}

/// The GetHealth request discriminant must be 0 (first variant).
#[test]
fn test_get_health_request_discriminant_is_0() {
    let disc = discriminant_of(&ClientRpcRequest::GetHealth);
    assert_eq!(disc, 0, "GetHealth request discriminant changed from 0 to {disc}!");
}

/// Golden files must be consistent: discriminants must be sequential
/// starting from 0 with no gaps.
#[test]
fn test_golden_files_are_sequential() {
    for filename in ["request_discriminants.txt", "response_discriminants.txt"] {
        let golden = load_golden(filename);
        for (i, (disc, name)) in golden.iter().enumerate() {
            assert_eq!(
                *disc, i as u32,
                "Golden file {filename} has gap: expected discriminant {i} \
                 but found {disc} for {name}"
            );
        }
    }
}
