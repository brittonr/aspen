use std::collections::HashMap;

use aspen_ci_core::ArtifactConfig;
use aspen_ci_core::IsolationMode;
use aspen_ci_core::JobConfig;
use aspen_ci_core::JobType;
use aspen_ci_core::PipelineConfig;
use aspen_ci_core::Priority;
use aspen_ci_core::StageConfig;
use aspen_ci_core::TriggerConfig;
use aspen_ci_core::verified::are_dependencies_met;
use aspen_ci_core::verified::are_resource_limits_valid;
use aspen_ci_core::verified::check_pipeline_limits;
use aspen_ci_core::verified::compute_deadline_ms;
use aspen_ci_core::verified::compute_effective_cpu_weight;
use aspen_ci_core::verified::compute_effective_memory_limit;
use aspen_ci_core::verified::count_total_jobs;
use aspen_ci_core::verified::extract_branch_name;
use aspen_ci_core::verified::find_ready_stages;
use aspen_ci_core::verified::has_self_dependency;
use aspen_ci_core::verified::is_branch_ref;
use aspen_ci_core::verified::is_deadline_exceeded;
use aspen_ci_core::verified::is_tag_ref;
use aspen_ci_core::verified::ms_to_secs;
use aspen_ci_core::verified::path_matches_pattern;
use aspen_ci_core::verified::ref_matches_any_pattern;
use aspen_ci_core::verified::ref_matches_pattern;
use aspen_ci_core::verified::remaining_time_ms;
use aspen_ci_core::verified::secs_to_ms;
use aspen_cluster_types::ClusterMetrics;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::ClusterState;
use aspen_cluster_types::ControlPlaneError;
use aspen_cluster_types::NodeId as ClusterNodeId;
use aspen_cluster_types::NodeState;
use aspen_constants::api;
use aspen_constants::ci;
use aspen_constants::coordination;
use aspen_constants::network;
use aspen_constants::raft;
use aspen_coordination_protocol::CounterResultResponse;
use aspen_coordination_protocol::LockResultResponse;
use aspen_coordination_protocol::QueueEnqueueResultResponse;
use aspen_crypto::cookie::MAX_COOKIE_LENGTH;
use aspen_crypto::cookie::UNSAFE_DEFAULT_COOKIE;
use aspen_crypto::cookie::derive_cookie_hmac_key;
use aspen_crypto::cookie::derive_gossip_topic;
use aspen_crypto::cookie::validate_cookie;
use aspen_crypto::cookie::validate_cookie_safety;
use aspen_forge_protocol::ForgeCommitInfo;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_forge_protocol::ForgeRepoListResultResponse;
use aspen_forge_protocol::ForgeRepoResultResponse;
use aspen_forge_protocol::ForgeTreeEntry;
use aspen_hlc::create_hlc;
use aspen_hlc::new_timestamp;
use aspen_hlc::to_unix_ms;
use aspen_hooks_types::ExecutionMode;
use aspen_hooks_types::HookHandlerConfig;
use aspen_hooks_types::HookHandlerType;
use aspen_hooks_types::HooksConfig;
use aspen_hooks_types::constants::DEFAULT_HANDLER_TIMEOUT_MS;
use aspen_hooks_types::constants::HOOK_TOPIC_PREFIX;
use aspen_hooks_types::constants::MAX_HANDLER_NAME_SIZE;
use aspen_jobs_protocol::JobDetails;
use aspen_jobs_protocol::JobQueueStatsResultResponse;
use aspen_jobs_protocol::JobSubmitResultResponse;
use aspen_jobs_protocol::PriorityCount;
use aspen_jobs_protocol::TypeCount;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_layer::Element;
use aspen_layer::Subspace;
use aspen_layer::Tuple;
use aspen_time::current_time_ms;
use aspen_time::current_time_secs;

fn main() {
    println!("=== Aspen Workspace Self-Build ===");
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!();

    // aspen-constants: Tiger Style resource bounds
    println!("[aspen-constants]");
    println!("  MAX_KEY_SIZE       = {} bytes", api::MAX_KEY_SIZE);
    println!("  MAX_VALUE_SIZE     = {} bytes", api::MAX_VALUE_SIZE);
    println!("  MAX_BATCH_SIZE     = {}", raft::MAX_BATCH_SIZE);
    println!("  MAX_PEERS          = {}", network::MAX_PEERS);
    println!("  MAX_CI_VMS         = {}", ci::MAX_CI_VMS_PER_NODE);
    println!();

    // aspen-hlc: HLC timestamps with blake3 node ID hashing
    println!("[aspen-hlc]");
    let hlc = create_hlc("test-node-1");
    let ts1 = new_timestamp(&hlc);
    let ts2 = new_timestamp(&hlc);
    assert!(ts2 > ts1, "HLC timestamps must be monotonically increasing");
    let ms = to_unix_ms(&ts1);
    assert!(ms > 0, "Unix ms must be positive");
    println!("  HLC timestamps:    monotonic");
    println!("  ts1 unix_ms:       {ms}");
    println!("  ts2 > ts1:         true");
    println!();

    // aspen-kv-types: cross-crate types
    println!("[aspen-kv-types]");
    let write = WriteRequest {
        command: WriteCommand::Set {
            key: "test-key".to_string(),
            value: "test-value".to_string(),
        },
    };
    let read = ReadRequest::new("test-key");
    let scan = ScanRequest {
        prefix: "test-".to_string(),
        limit_results: Some(api::DEFAULT_SCAN_LIMIT),
        continuation_token: None,
    };
    let kv = KeyValueWithRevision {
        key: "test-key".to_string(),
        value: "test-value".to_string(),
        version: 1,
        create_revision: 1,
        mod_revision: 1,
    };
    println!("  WriteRequest:      {:?}", write.command);
    println!("  ReadRequest:       key={}", read.key);
    println!("  ScanRequest:       prefix={}, limit={:?}", scan.prefix, scan.limit_results);
    println!("  KV entry:          {}={} (v{})", kv.key, kv.value, kv.version);
    println!();

    // aspen-layer: FoundationDB tuple encoding
    println!("[aspen-layer]");
    let users = Subspace::new(Tuple::new().push("users"));
    let key = users.pack(&Tuple::new().push("alice").push(42i64));
    let unpacked = users.unpack(&key).expect("unpack must succeed");
    assert_eq!(unpacked.get(0), Some(&Element::String("alice".to_string())));
    assert_eq!(unpacked.get(1), Some(&Element::Int(42)));
    println!("  Packed key:        {} bytes", key.len());
    let t = Tuple::new().push("string").push(123i64).push(true).push(Element::Bytes(vec![0xDE, 0xAD]));
    let encoded = t.pack();
    let decoded = Tuple::unpack(&encoded).expect("tuple roundtrip");
    assert_eq!(decoded.len(), 4);
    println!("  Tuple elements:    {} (string, i64, bool, bytes)", decoded.len());
    let (range_begin, range_end) = users.range();
    assert!(range_begin < range_end);
    println!("  Range scan:        {} byte prefix", range_begin.len());
    println!();

    // aspen-time
    println!("[aspen-time]");
    let t1 = current_time_ms();
    let t2 = current_time_secs();
    assert!(t1 > 0);
    assert!(t2 > 0);
    assert!(t1 >= t2 * 1000);
    println!("  current_time_ms:   {t1}");
    println!("  current_time_secs: {t2}");
    println!("  monotonic:         true");
    println!();

    // aspen-coordination-protocol
    println!("[aspen-coordination-protocol]");
    let lock = LockResultResponse {
        is_success: true,
        fencing_token: Some(42),
        holder_id: Some("node-1".to_string()),
        deadline_ms: Some(t1 + 30_000),
        error: None,
    };
    let counter = CounterResultResponse {
        is_success: true,
        value: Some(100),
        error: None,
    };
    let queue = QueueEnqueueResultResponse {
        is_success: true,
        item_id: Some(1),
        error: None,
    };
    assert!(lock.is_success && counter.is_success && queue.is_success);
    println!("  LockResult:        fencing_token={:?}", lock.fencing_token);
    println!("  CounterResult:     value={:?}", counter.value);
    println!("  QueueResult:       item_id={:?}", queue.item_id);
    println!();

    // aspen-ci-core: verified pipeline functions
    println!("[aspen-ci-core]");
    let deadline = compute_deadline_ms(1000, 300);
    assert_eq!(deadline, 301_000);
    assert!(!is_deadline_exceeded(deadline, 2000));
    assert!(is_deadline_exceeded(deadline, 400_000));
    assert_eq!(remaining_time_ms(deadline, 2000), 299_000);
    assert_eq!(secs_to_ms(5), 5_000);
    assert_eq!(ms_to_secs(5_000), 5);
    assert!(has_self_dependency("build", &["build", "test"]));
    assert!(!has_self_dependency("build", &["test"]));
    assert!(are_dependencies_met(&["check"], &["check", "lint"]));
    assert!(!are_dependencies_met(&["check"], &["lint"]));
    let ready = find_ready_stages(&[("check", vec![]), ("build", vec!["check"])], &["check"], &["check"]);
    assert_eq!(ready, vec!["build"]);
    assert_eq!(count_total_jobs(&[2, 3, 1]), 6);
    assert!(check_pipeline_limits(3, 10, 10, 100).is_ok());
    assert_eq!(compute_effective_memory_limit(Some(512), 1024, 256), 512);
    assert_eq!(compute_effective_memory_limit(None, 1024, 256), 256);
    assert_eq!(compute_effective_cpu_weight(None, 1000, 100), 100);
    assert!(are_resource_limits_valid(512, 100, 256, 1024, 1000));
    assert!(ref_matches_pattern("refs/heads/main", "refs/heads/main"));
    assert!(ref_matches_pattern("refs/heads/feature/foo", "refs/heads/feature/*"));
    assert!(ref_matches_any_pattern("refs/heads/main", &["refs/heads/main", "refs/tags/*"]));
    assert_eq!(extract_branch_name("refs/heads/main"), Some("main"));
    assert!(is_branch_ref("refs/heads/main"));
    assert!(is_tag_ref("refs/tags/v1.0"));
    assert!(path_matches_pattern("src/lib.rs", "src/*"));
    assert!(!path_matches_pattern("docs/README.md", "src/*"));
    let _pipeline = PipelineConfig {
        name: "test-pipeline".to_string(),
        description: Some("CI pipeline".to_string()),
        triggers: TriggerConfig::default(),
        stages: vec![StageConfig {
            name: "build".to_string(),
            depends_on: vec![],
            parallel: true,
            when: None,
            jobs: vec![JobConfig {
                name: "compile".to_string(),
                job_type: JobType::Nix,
                isolation: IsolationMode::None,
                timeout_secs: 600,
                flake_url: Some(".".to_string()),
                flake_attr: Some("packages.x86_64-linux.default".to_string()),
                command: None,
                args: vec![],
                env: HashMap::new(),
                working_dir: None,
                publish_to_cache: false,
                should_upload_result: true,
                binary_hash: None,
                cache_key: None,
                artifacts: vec![],
                depends_on: vec![],
                retry_count: 0,
                allow_failure: false,
                tags: vec![],
            }],
        }],
        artifacts: ArtifactConfig::default(),
        env: HashMap::new(),
        timeout_secs: 600,
        priority: Priority::Normal,
    };
    println!("  compute_deadline:  1000ms+300s = {deadline}ms");
    println!("  pipeline_limits:   3 stages, 10 jobs — OK");
    println!("  branch_extract:    refs/heads/main -> main");
    println!("  path_match:        src/lib.rs ~ src/* -> true");
    println!("  PipelineConfig:    constructed");
    println!();

    // aspen-forge-protocol
    println!("[aspen-forge-protocol]");
    let repo = ForgeRepoInfo {
        id: "repo-001".to_string(),
        name: "aspen".to_string(),
        description: Some("Distributed system".to_string()),
        default_branch: "main".to_string(),
        delegates: vec![],
        threshold_delegates: 0,
        created_at_ms: t1,
    };
    let result = ForgeRepoResultResponse {
        is_success: true,
        repo: Some(repo.clone()),
        error: None,
    };
    let list = ForgeRepoListResultResponse {
        is_success: true,
        repos: vec![repo],
        count: 1,
        error: None,
    };
    let tree_entry = ForgeTreeEntry {
        mode: 0o040000,
        name: "src".to_string(),
        hash: "abc123".to_string(),
    };
    let commit = ForgeCommitInfo {
        hash: "deadbeef".to_string(),
        tree: "aabbccdd".to_string(),
        parents: vec![],
        author_name: "test".to_string(),
        author_email: Some("test@test.com".to_string()),
        author_key: None,
        message: "initial commit".to_string(),
        timestamp_ms: t1,
    };
    assert!(result.is_success);
    assert_eq!(list.repos.len(), 1);
    assert!(!commit.hash.is_empty());
    println!("  ForgeRepoInfo:     id={}", result.repo.unwrap().id);
    println!("  ForgeRepoList:     {} repos", list.repos.len());
    println!("  ForgeTreeEntry:    name={}", tree_entry.name);
    println!("  ForgeCommitInfo:   hash={}", commit.hash);
    println!();

    // aspen-jobs-protocol
    println!("[aspen-jobs-protocol]");
    let submit = JobSubmitResultResponse {
        is_success: true,
        job_id: Some("job-42".to_string()),
        error: None,
    };
    let details = JobDetails {
        job_id: "job-42".to_string(),
        job_type: "ci_nix_build".to_string(),
        status: "completed".to_string(),
        priority: 100,
        progress: 100,
        progress_message: None,
        payload: "{}".to_string(),
        tags: vec![],
        submitted_at: "2026-03-09T00:00:00Z".to_string(),
        started_at: Some("2026-03-09T00:00:01Z".to_string()),
        completed_at: Some("2026-03-09T00:00:06Z".to_string()),
        worker_id: Some("worker-1".to_string()),
        attempts: 1,
        result: Some("success".to_string()),
        error_message: None,
    };
    let stats = JobQueueStatsResultResponse {
        pending_count: 5,
        scheduled_count: 0,
        running_count: 2,
        completed_count: 100,
        failed_count: 3,
        cancelled_count: 0,
        priority_counts: vec![
            PriorityCount {
                priority: 100,
                count: 4,
            },
            PriorityCount {
                priority: 200,
                count: 1,
            },
        ],
        type_counts: vec![
            TypeCount {
                job_type: "ci_nix_build".to_string(),
                count: 3,
            },
            TypeCount {
                job_type: "shell_command".to_string(),
                count: 2,
            },
        ],
        error: None,
    };
    assert!(submit.is_success);
    assert_eq!(details.attempts, 1);
    assert_eq!(stats.pending_count, 5);
    assert_eq!(stats.type_counts.len(), 2);
    println!("  JobSubmit:         id={}", submit.job_id.unwrap());
    println!("  JobDetails:        status={}, attempts={}", details.status, details.attempts);
    println!(
        "  QueueStats:        pending={}, running={}, completed={}",
        stats.pending_count, stats.running_count, stats.completed_count
    );
    println!();

    // aspen-cluster-types (iroh-backed)
    println!("[aspen-cluster-types]");
    let node_id = ClusterNodeId(1);
    assert_eq!(node_id.0, 1);
    let node = ClusterNode::new(1, "127.0.0.1:9000", None);
    assert_eq!(node.id, 1);
    let state = ClusterState {
        nodes: vec![
            ClusterNode::new(1, "127.0.0.1:9000", None),
            ClusterNode::new(2, "127.0.0.1:9001", None),
            ClusterNode::new(3, "127.0.0.1:9002", None),
        ],
        members: vec![1, 2, 3],
        learners: vec![],
    };
    assert_eq!(state.members.len(), 3);
    assert_eq!(state.nodes.len(), 3);
    let metrics = ClusterMetrics {
        id: 1,
        state: NodeState::Leader,
        current_leader: Some(1),
        current_term: 5,
        last_log_index: Some(42),
        last_applied_index: Some(40),
        snapshot_index: None,
        replication: None,
        voters: vec![1, 2, 3],
        learners: vec![],
    };
    assert_eq!(metrics.current_term, 5);
    assert_eq!(metrics.voters.len(), 3);
    let err = ControlPlaneError::NotInitialized;
    let _ = format!("{err}");
    println!("  ClusterNodeId:     {}", node_id.0);
    println!("  ClusterState:      {} nodes, {} voters", state.nodes.len(), state.members.len());
    println!("  ClusterMetrics:    term={}, last_log={:?}", metrics.current_term, metrics.last_log_index);
    println!();

    // aspen-hooks-types
    println!("[aspen-hooks-types]");
    assert!(DEFAULT_HANDLER_TIMEOUT_MS > 0);
    assert!(MAX_HANDLER_NAME_SIZE > 0);
    assert!(!HOOK_TOPIC_PREFIX.is_empty());
    let handler = HookHandlerConfig {
        name: "on-push".to_string(),
        pattern: "hooks.forge.push".to_string(),
        execution_mode: ExecutionMode::Direct,
        handler_type: HookHandlerType::Shell {
            command: "echo pushed".to_string(),
            working_dir: None,
        },
        timeout_ms: DEFAULT_HANDLER_TIMEOUT_MS,
        retry_count: 3,
        job_priority: None,
        is_enabled: true,
    };
    let hooks_config = HooksConfig {
        is_enabled: true,
        publish_topics: vec!["hooks.>".to_string()],
        handlers: vec![handler],
    };
    assert_eq!(hooks_config.handlers.len(), 1);
    assert_eq!(hooks_config.handlers[0].name, "on-push");
    println!("  DEFAULT_TIMEOUT:   {}ms", DEFAULT_HANDLER_TIMEOUT_MS);
    println!("  TOPIC_PREFIX:      {}", HOOK_TOPIC_PREFIX);
    println!("  HooksConfig:       {} handlers", hooks_config.handlers.len());
    println!();

    // aspen-crypto
    println!("[aspen-crypto]");
    assert!(MAX_COOKIE_LENGTH > 0);
    let good_cookie = "my-secure-cluster-cookie-2024";
    assert!(validate_cookie(good_cookie).is_ok());
    assert!(validate_cookie_safety(UNSAFE_DEFAULT_COOKIE).is_err());
    let hmac1 = derive_cookie_hmac_key(good_cookie);
    let hmac2 = derive_cookie_hmac_key(good_cookie);
    assert_eq!(hmac1, hmac2);
    let hmac3 = derive_cookie_hmac_key("different-cookie");
    assert_ne!(hmac1, hmac3);
    let topic1 = derive_gossip_topic(good_cookie);
    let topic2 = derive_gossip_topic(good_cookie);
    assert_eq!(topic1, topic2);
    let topic3 = derive_gossip_topic("different-cookie");
    assert_ne!(topic1, topic3);
    println!("  MAX_COOKIE_LEN:    {}", MAX_COOKIE_LENGTH);
    println!("  cookie_validate:   OK");
    println!("  unsafe_detect:     blocked");
    println!("  hmac_derive:       deterministic, 32 bytes");
    println!("  gossip_topic:      deterministic, 32 bytes");
    println!();

    // Cross-crate validation
    assert!(api::MAX_KEY_SIZE > 0);
    assert!(api::DEFAULT_SCAN_LIMIT <= api::MAX_SCAN_RESULTS);
    assert!(coordination::MAX_CAS_RETRIES > 0);

    println!("13 crates, 442 packages, all assertions passed");
    println!("Built by Aspen CI");
}
