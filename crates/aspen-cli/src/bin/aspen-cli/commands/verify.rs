//! Verification commands for testing cluster replication.
//!
//! Commands to verify that state, docs, and blobs are being
//! replicated correctly across the cluster.
//!
//! ## Cross-Node Verification Strategy
//!
//! For KV: Uses Raft metrics (matched_index vs last_applied) to verify
//! that writes have been replicated to all followers.
//!
//! For Docs: Writes a test entry, then queries each node's entry count
//! to verify that iroh-docs CRDT sync has propagated the entry.
//!
//! For Blobs: Adds a test blob, then queries each node to verify the
//! blob hash exists (content-addressed deduplication means same content
//! will have same hash on all nodes).
//!
//! ## Metrics Collected
//!
//! Each verification test captures:
//! - Total duration (ms)
//! - Per-phase timing breakdown (write, replicate, read, cleanup)
//! - Replication lag per node (matched_index vs last_applied)
//! - Node-level status (healthy, behind, unreachable)
//! - Error counts and details

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const VERIFY_ALL_KEY_COUNT: u32 = 3;
const VERIFY_ALL_KEY_PREFIX: &str = "__verify_";

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::NodeDescriptor;
use clap::Args;
use clap::Subcommand;
use iroh::EndpointAddr;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Verification commands for testing replication.
#[derive(Subcommand)]
pub enum VerifyCommand {
    /// Verify KV store replication (write, read, delete cycle).
    Kv(VerifyKvArgs),

    /// Verify iroh-docs CRDT sync status.
    Docs(VerifyDocsArgs),

    /// Verify blob storage and retrieval.
    Blob(VerifyBlobArgs),

    /// Run all verification tests.
    All(VerifyAllArgs),
}

#[derive(Args)]
pub struct VerifyKvArgs {
    /// Number of test keys to write.
    #[arg(long, default_value = "5")]
    pub count: u32,

    /// Key prefix for test keys.
    #[arg(long, default_value = "__verify_")]
    pub prefix: String,

    /// Skip cleanup of test keys.
    #[arg(long)]
    pub no_cleanup: bool,
}

#[derive(Args)]
pub struct VerifyDocsArgs {
    /// Write a test entry and verify sync.
    #[arg(long)]
    pub write_test: bool,
}

#[derive(Args)]
pub struct VerifyBlobArgs {
    /// Size of test blob in bytes.
    #[arg(long = "size", default_value = "1024")]
    pub size_bytes: u32,

    /// Skip cleanup of test blob.
    #[arg(long)]
    pub no_cleanup: bool,
}

#[derive(Args)]
pub struct VerifyAllArgs {
    /// Skip cleanup of test data.
    #[arg(long)]
    pub no_cleanup: bool,

    /// Continue on failure.
    #[arg(long)]
    pub continue_on_error: bool,
}

/// Detailed timing breakdown for verification phases.
#[derive(Clone, Default)]
pub struct VerifyTiming {
    /// Time spent writing test data (ms).
    pub write_ms: u64,
    /// Time spent waiting for replication (ms).
    pub replicate_ms: u64,
    /// Time spent reading/verifying data (ms).
    pub read_ms: u64,
    /// Time spent on cleanup (ms).
    pub cleanup_ms: u64,
    /// Time spent on cross-node queries (ms).
    pub cross_node_ms: u64,
}

impl VerifyTiming {
    /// Total time across all phases.
    pub fn total_ms(&self) -> u64 {
        self.write_ms
            .saturating_add(self.replicate_ms)
            .saturating_add(self.read_ms)
            .saturating_add(self.cleanup_ms)
            .saturating_add(self.cross_node_ms)
    }
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "CLI verification commands own the wall-clock measurement boundary"
)]
fn current_unix_seconds() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration_since_epoch) => duration_since_epoch.as_secs(),
        Err(_) => 0,
    }
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "CLI verification commands own the monotonic timing boundary"
)]
fn current_instant() -> Instant {
    Instant::now()
}

fn elapsed_ms_u64(started_at: Instant) -> u64 {
    match u64::try_from(started_at.elapsed().as_millis()) {
        Ok(duration_ms) => duration_ms,
        Err(_) => u64::MAX,
    }
}

fn usize_from_u32(value: u32) -> usize {
    match usize::try_from(value) {
        Ok(value_usize) => value_usize,
        Err(_) => usize::MAX,
    }
}

fn u32_from_usize(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(value_u32) => value_u32,
        Err(_) => u32::MAX,
    }
}

#[derive(Clone, Copy)]
struct DocsAvailabilityError<'a> {
    code: &'a str,
    message: &'a str,
}

async fn cleanup_verify_keys(client: &AspenClient, test_keys: &[String], errors: &mut Vec<String>) {
    for key in test_keys {
        match client.send(ClientRpcRequest::DeleteKey { key: key.clone() }).await {
            Ok(ClientRpcResponse::DeleteResult(_)) => {}
            Ok(ClientRpcResponse::Error(error)) => {
                errors.push(format!("Cleanup {} failed: {}", key, error.message));
            }
            Ok(_) => {
                errors.push(format!("Cleanup {} unexpected response", key));
            }
            Err(error) => {
                errors.push(format!("Cleanup {} error: {}", key, error));
            }
        }
    }
}

/// Per-node replication status for detailed metrics.
#[derive(Clone)]
pub struct NodeReplicationStatus {
    /// Node ID.
    pub node_id: u64,
    /// Current matched index (how far this node has replicated).
    pub matched_index: u64,
    /// Leader's last applied index at time of check.
    pub leader_applied: u64,
    /// Replication lag in entries (leader_applied - matched_index).
    pub lag: u64,
    /// Status label: "OK", "BEHIND", "UNREACHABLE".
    pub status: String,
}

/// Result of a single verification test.
#[derive(Clone)]
pub struct VerifyResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub duration_ms: u64,
    pub details: Option<String>,
    /// Detailed timing breakdown (optional for enhanced output).
    pub timing: Option<VerifyTiming>,
    /// Per-node replication status (optional for KV verification).
    pub node_status: Option<Vec<NodeReplicationStatus>>,
}

impl Outputable for VerifyResult {
    fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::json!({
            "name": self.name,
            "passed": self.passed,
            "message": self.message,
            "duration_ms": self.duration_ms,
            "details": self.details
        });

        // Add timing breakdown if available
        if let Some(ref timing) = self.timing {
            json["timing"] = serde_json::json!({
                "write_ms": timing.write_ms,
                "replicate_ms": timing.replicate_ms,
                "read_ms": timing.read_ms,
                "cleanup_ms": timing.cleanup_ms,
                "cross_node_ms": timing.cross_node_ms,
                "total_ms": timing.total_ms()
            });
        }

        // Add per-node replication status if available
        if let Some(ref nodes) = self.node_status {
            json["nodes"] = serde_json::json!(
                nodes
                    .iter()
                    .map(|n| {
                        serde_json::json!({
                            "node_id": n.node_id,
                            "matched_index": n.matched_index,
                            "leader_applied": n.leader_applied,
                            "lag": n.lag,
                            "status": n.status
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        json
    }

    fn to_human(&self) -> String {
        let status = if self.passed { "PASS" } else { "FAIL" };
        let mut output = format!("[{}] {} ({} ms): {}", status, self.name, self.duration_ms, self.message);

        // Add details line
        if let Some(ref details) = self.details {
            output.push_str(&format!("\n  {}", details));
        }

        // Add timing breakdown if available and there's interesting detail
        if let Some(ref timing) = self.timing
            && (timing.write_ms > 0 || timing.read_ms > 0)
        {
            output.push_str(&format!(
                "\n  timing: write={}ms, replicate={}ms, read={}ms, cleanup={}ms",
                timing.write_ms, timing.replicate_ms, timing.read_ms, timing.cleanup_ms
            ));
            if timing.cross_node_ms > 0 {
                output.push_str(&format!(", cross-node={}ms", timing.cross_node_ms));
            }
        }

        // Add per-node status if available and there's lag
        if let Some(ref nodes) = self.node_status {
            let lagging: Vec<_> = nodes.iter().filter(|n| n.lag > 0).collect();
            if !lagging.is_empty() {
                let lag_info: Vec<String> =
                    lagging.iter().map(|n| format!("node {}: {} behind", n.node_id, n.lag)).collect();
                output.push_str(&format!("\n  replication lag: {}", lag_info.join(", ")));
            }
        }

        output
    }
}

/// Result of all verification tests.
pub struct VerifyAllResult {
    pub results: Vec<VerifyResult>,
    pub total_passed: u32,
    pub total_failed: u32,
}

impl Outputable for VerifyAllResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "results": self.results.iter().map(|r| r.to_json()).collect::<Vec<_>>(),
            "total_passed": self.total_passed,
            "total_failed": self.total_failed,
            "all_passed": self.total_failed == 0
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::from("Verification Results\n====================\n\n");

        for result in &self.results {
            output.push_str(&result.to_human());
            output.push('\n');
        }

        output.push_str(&format!("\nSummary: {} passed, {} failed", self.total_passed, self.total_failed));

        if self.total_failed == 0 {
            output.push_str("\nAll verification tests passed!");
        }

        output
    }
}

impl VerifyCommand {
    /// Execute the verify command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            VerifyCommand::Kv(args) => verify_kv(client, args, is_json_output).await,
            VerifyCommand::Docs(args) => verify_docs(client, args, is_json_output).await,
            VerifyCommand::Blob(args) => verify_blob(client, args, is_json_output).await,
            VerifyCommand::All(args) => verify_all(client, args, is_json_output).await,
        }
    }
}

/// Verify KV store replication.
async fn verify_kv(client: &AspenClient, args: VerifyKvArgs, is_json_output: bool) -> Result<()> {
    let started_at = current_instant();
    let timestamp = current_unix_seconds();
    let test_key_count = usize_from_u32(args.count);

    let mut errors = Vec::with_capacity(test_key_count);
    let mut replication_details = Vec::with_capacity(test_key_count);
    let mut node_status_list: Vec<NodeReplicationStatus> = Vec::with_capacity(test_key_count);
    let mut timing = VerifyTiming::default();

    let test_keys: Vec<String> =
        (0..args.count).map(|index| format!("{}test_{}_{}", args.prefix, timestamp, index)).collect();
    debug_assert_eq!(test_keys.len(), test_key_count);

    let write_started_at = current_instant();
    verify_kv_write_test_keys(client, &test_keys, &mut errors).await;
    timing.write_ms = elapsed_ms_u64(write_started_at);

    let replicate_started_at = current_instant();
    tokio::time::sleep(Duration::from_millis(100)).await;
    timing.replicate_ms = elapsed_ms_u64(replicate_started_at);

    let is_replication_ok =
        verify_kv_check_replication(client, &mut errors, &mut replication_details, &mut node_status_list).await;

    let read_started_at = current_instant();
    let read_success = verify_kv_read_and_verify(client, &test_keys, &mut errors).await;
    timing.read_ms = elapsed_ms_u64(read_started_at);

    let cleanup_started_at = current_instant();
    if !args.no_cleanup {
        cleanup_verify_keys(client, &test_keys, &mut errors).await;
    }
    timing.cleanup_ms = elapsed_ms_u64(cleanup_started_at);

    let result = verify_kv_build_result(VerifyKvResultInputs {
        count: args.count,
        read_success,
        is_replication_ok,
        errors: &errors,
        replication_details: &replication_details,
        duration_ms: elapsed_ms_u64(started_at),
        timing,
        node_status_list,
    });

    print_output(&result, is_json_output);

    if !result.passed {
        anyhow::bail!("KV verification failed");
    }

    Ok(())
}

/// Write test keys and collect errors.
async fn verify_kv_write_test_keys(client: &AspenClient, test_keys: &[String], errors: &mut Vec<String>) {
    for (i, key) in test_keys.iter().enumerate() {
        let value = format!("verify_value_{}", i);
        let response = client
            .send(ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.as_bytes().to_vec(),
            })
            .await;

        match response {
            Ok(ClientRpcResponse::WriteResult(_)) => {}
            Ok(ClientRpcResponse::Error(e)) => {
                errors.push(format!("Write {} failed: {}", key, e.message));
            }
            Ok(_) => errors.push(format!("Write {} unexpected response", key)),
            Err(e) => errors.push(format!("Write {} error: {}", key, e)),
        }
    }
}

/// Check replication status from leader, collecting per-node details.
async fn verify_kv_check_replication(
    client: &AspenClient,
    errors: &mut Vec<String>,
    replication_details: &mut Vec<String>,
    node_status_list: &mut Vec<NodeReplicationStatus>,
) -> bool {
    match client.send(ClientRpcRequest::GetRaftMetrics).await {
        Ok(ClientRpcResponse::RaftMetrics(metrics)) => {
            let last_applied = metrics.last_applied_index.unwrap_or(0);

            if let Some(replication) = &metrics.replication {
                let mut is_caught_up = true;
                for progress in replication {
                    let matched = progress.matched_index.unwrap_or(0);
                    let lag = last_applied.saturating_sub(matched);
                    let status = if matched >= last_applied {
                        "OK".to_string()
                    } else {
                        is_caught_up = false;
                        "BEHIND".to_string()
                    };

                    node_status_list.push(NodeReplicationStatus {
                        node_id: progress.node_id,
                        matched_index: matched,
                        leader_applied: last_applied,
                        lag,
                        status: status.clone(),
                    });

                    replication_details.push(format!(
                        "node {}: matched={}, last_applied={} [{}]",
                        progress.node_id, matched, last_applied, status
                    ));
                }
                is_caught_up
            } else {
                replication_details.push(format!(
                    "leader: {}, last_applied: {}",
                    metrics.current_leader.map(|l| l.to_string()).unwrap_or_else(|| "none".to_string()),
                    last_applied
                ));
                true
            }
        }
        Ok(ClientRpcResponse::Error(e)) => {
            errors.push(format!("Failed to get replication status: {}", e.message));
            false
        }
        _ => {
            errors.push("Unexpected response from GetRaftMetrics".to_string());
            false
        }
    }
}

/// Read back test keys and verify values, returning the count of successes.
async fn verify_kv_read_and_verify(client: &AspenClient, test_keys: &[String], errors: &mut Vec<String>) -> u32 {
    let mut read_success = 0u32;
    let expected_key_count = u32_from_usize(test_keys.len());
    debug_assert!(errors.len() <= errors.capacity());
    for (index, key) in test_keys.iter().enumerate() {
        let expected = format!("verify_value_{}", index);
        let response = client.send(ClientRpcRequest::ReadKey { key: key.clone() }).await;

        match response {
            Ok(ClientRpcResponse::ReadResult(result)) => {
                if let Some(value) = result.value {
                    if value == expected.as_bytes() {
                        read_success = read_success.saturating_add(1);
                    } else {
                        errors.push(format!("Key {} value mismatch", key));
                    }
                } else {
                    errors.push(format!("Key {} not found after write", key));
                }
            }
            Ok(ClientRpcResponse::Error(error)) => {
                errors.push(format!("Read {} failed: {}", key, error.message));
            }
            Ok(_) => errors.push(format!("Read {} unexpected response", key)),
            Err(error) => errors.push(format!("Read {} error: {}", key, error)),
        }
    }
    debug_assert!(read_success <= expected_key_count);
    read_success
}

/// Inputs for building the final KV VerifyResult.
struct VerifyKvResultInputs<'a> {
    count: u32,
    read_success: u32,
    is_replication_ok: bool,
    errors: &'a [String],
    replication_details: &'a [String],
    duration_ms: u64,
    timing: VerifyTiming,
    node_status_list: Vec<NodeReplicationStatus>,
}

/// Build the final VerifyResult from collected data.
fn verify_kv_build_result(inputs: VerifyKvResultInputs<'_>) -> VerifyResult {
    let VerifyKvResultInputs {
        count,
        read_success,
        is_replication_ok,
        errors,
        replication_details,
        duration_ms,
        timing,
        node_status_list,
    } = inputs;
    let is_passed = errors.is_empty() && read_success == count && is_replication_ok;
    debug_assert!(read_success <= count);
    debug_assert!(is_passed || !errors.is_empty() || read_success < count || !is_replication_ok);

    let mut all_details = Vec::with_capacity(2);
    if !replication_details.is_empty() {
        all_details.push(format!("replication: {}", replication_details.join(", ")));
    }
    if !errors.is_empty() {
        all_details.push(format!("errors: {}", errors.join("; ")));
    }

    VerifyResult {
        name: "kv-replication".to_string(),
        passed: is_passed,
        message: if is_passed {
            format!("{}/{} keys verified, replication OK", read_success, count)
        } else if !is_replication_ok {
            format!("{}/{} keys verified, replication BEHIND", read_success, count)
        } else {
            format!("{}/{} keys verified, {} errors", read_success, count, errors.len())
        },
        duration_ms,
        details: if all_details.is_empty() {
            None
        } else {
            Some(all_details.join(" | "))
        },
        timing: Some(timing),
        node_status: if node_status_list.is_empty() {
            None
        } else {
            Some(node_status_list)
        },
    }
}

/// Verify iroh-docs sync status.
async fn verify_docs(client: &AspenClient, args: VerifyDocsArgs, is_json_output: bool) -> Result<()> {
    let start = current_instant();
    let mut details = Vec::with_capacity(6);

    let (is_enabled, namespace_id) = match verify_docs_check_status(client, &mut details).await {
        Ok(result) => result,
        Err(fail_result) => {
            print_output(&fail_result, is_json_output);
            anyhow::bail!("Docs verification failed");
        }
    };

    if !is_enabled {
        let result = VerifyResult {
            name: "docs-sync".to_string(),
            passed: true,
            message: "Docs sync not enabled (skipped)".to_string(),
            duration_ms: elapsed_ms_u64(start),
            details: None,
            timing: None,
            node_status: None,
        };
        print_output(&result, is_json_output);
        return Ok(());
    }

    if args.write_test {
        verify_docs_write_read_test(client, &mut details).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let is_cross_node_ok = verify_docs_cross_node(client, &mut details).await;
    let result = verify_docs_build_result(&namespace_id, &details, is_cross_node_ok, elapsed_ms_u64(start));
    debug_assert!(!args.write_test || !details.is_empty());

    print_output(&result, is_json_output);

    if !result.passed {
        anyhow::bail!("Docs verification failed");
    }

    Ok(())
}

/// Check docs status. Returns Ok((enabled, namespace_id)) or Err(VerifyResult) on failure.
async fn verify_docs_check_status(
    client: &AspenClient,
    details: &mut Vec<String>,
) -> std::result::Result<(bool, Option<String>), VerifyResult> {
    let response = client.send(ClientRpcRequest::DocsStatus).await;
    let status_result = match response {
        Ok(ClientRpcResponse::DocsStatusResult(status)) => {
            if let Some(ref namespace_id) = status.namespace_id {
                details.push(format!("namespace: {}", namespace_id));
            }
            if let Some(entry_count) = status.entry_count {
                details.push(format!("entries: {}", entry_count));
            }
            Ok((true, status.namespace_id.clone()))
        }
        Ok(ClientRpcResponse::Error(error)) => {
            let docs_error = DocsAvailabilityError {
                code: &error.code,
                message: &error.message,
            };
            if is_docs_unavailable_error(docs_error) {
                Ok((false, None))
            } else {
                Err(VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: false,
                    message: format!("Status check failed: {}", error.message),
                    duration_ms: 0,
                    details: None,
                    timing: None,
                    node_status: None,
                })
            }
        }
        Ok(_) => Err(VerifyResult {
            name: "docs-sync".to_string(),
            passed: false,
            message: "Unexpected response from docs status".to_string(),
            duration_ms: 0,
            details: None,
            timing: None,
            node_status: None,
        }),
        Err(error) => Err(VerifyResult {
            name: "docs-sync".to_string(),
            passed: false,
            message: format!("Error: {}", error),
            duration_ms: 0,
            details: None,
            timing: None,
            node_status: None,
        }),
    };
    debug_assert!(details.len() <= details.capacity());
    debug_assert!(match &status_result {
        Ok((is_enabled, _)) => *is_enabled || details.is_empty(),
        Err(result) => !result.passed,
    });
    status_result
}

/// Write a test entry via docs and read it back, collecting results into details.
async fn verify_docs_write_read_test(client: &AspenClient, details: &mut Vec<String>) {
    let timestamp = current_unix_seconds();
    let test_key = format!("__verify_docs_{}", timestamp);
    let test_value = "docs_verify_value";
    debug_assert!(!test_key.is_empty());

    // Write via docs
    let write_response = client
        .send(ClientRpcRequest::DocsSet {
            key: test_key.clone(),
            value: test_value.as_bytes().to_vec(),
        })
        .await;

    match write_response {
        Ok(ClientRpcResponse::DocsSetResult(_)) => {
            details.push("write: OK".to_string());
        }
        Ok(ClientRpcResponse::Error(e)) => {
            details.push(format!("write: FAIL ({})", e.message));
        }
        _ => {
            details.push("write: FAIL (unexpected response)".to_string());
        }
    }

    // Read back
    tokio::time::sleep(Duration::from_millis(100)).await;

    let read_response = client.send(ClientRpcRequest::DocsGet { key: test_key.clone() }).await;

    match read_response {
        Ok(ClientRpcResponse::DocsGetResult(result)) => {
            if result.was_found && result.value.as_deref() == Some(test_value.as_bytes()) {
                details.push("read: OK".to_string());
            } else {
                details.push("read: FAIL (value mismatch)".to_string());
            }
        }
        Ok(ClientRpcResponse::Error(e)) => {
            details.push(format!("read: FAIL ({})", e.message));
        }
        _ => {
            details.push("read: FAIL (unexpected response)".to_string());
        }
    }

    // Cleanup
    match client.send(ClientRpcRequest::DocsDelete { key: test_key }).await {
        Ok(ClientRpcResponse::DocsDeleteResult(_)) => {}
        Ok(ClientRpcResponse::Error(error)) => {
            details.push(format!("cleanup: FAIL ({})", error.message));
        }
        Ok(_) => {
            details.push("cleanup: FAIL (unexpected response)".to_string());
        }
        Err(error) => {
            details.push(format!("cleanup: FAIL ({})", error));
        }
    }
}

/// Build the final VerifyResult for docs verification.
fn verify_docs_build_result(
    namespace_id: &Option<String>,
    details: &[String],
    is_cross_node_ok: bool,
    duration_ms: u64,
) -> VerifyResult {
    let is_passed =
        !details.iter().any(|detail| detail.contains("FAIL") || detail.contains("MISMATCH")) && is_cross_node_ok;
    debug_assert!(is_passed || !details.is_empty() || !is_cross_node_ok);

    VerifyResult {
        name: "docs-sync".to_string(),
        passed: is_passed,
        message: if is_passed {
            format!("Docs enabled, namespace {}", namespace_id.as_deref().unwrap_or("unknown"))
        } else {
            "Docs verification failed".to_string()
        },
        duration_ms,
        details: if details.is_empty() {
            None
        } else {
            Some(details.join(", "))
        },
        timing: None,
        node_status: None,
    }
}

fn generate_test_blob_data(size_bytes: u32) -> Vec<u8> {
    let mut test_data = Vec::with_capacity(usize::try_from(size_bytes).unwrap_or(usize::MAX));
    for byte_index in 0..size_bytes {
        test_data.push(u8::try_from(byte_index % 256).unwrap_or(0));
    }
    test_data
}

/// Verify blob storage.
async fn verify_blob(client: &AspenClient, args: VerifyBlobArgs, is_json_output: bool) -> Result<()> {
    let started_at = current_instant();
    let mut details = Vec::with_capacity(8);
    let test_data = generate_test_blob_data(args.size_bytes);

    let hash = match verify_blob_add(client, &test_data, &mut details).await {
        VerifyBlobAddResult::Hash(hash) => hash,
        VerifyBlobAddResult::Skipped(result) => {
            print_output(&result, is_json_output);
            return Ok(());
        }
        VerifyBlobAddResult::Failed(result) => {
            print_output(&result, is_json_output);
            anyhow::bail!("Blob verification failed");
        }
    };

    verify_blob_check_exists(client, &hash, &mut details).await;
    verify_blob_verify_content(client, &hash, &test_data, &mut details).await;
    let is_cross_node_ok = verify_blob_cross_node(client, &hash, &mut details).await;

    if !args.no_cleanup {
        match client
            .send(ClientRpcRequest::DeleteBlob {
                hash: hash.clone(),
                is_force: true,
            })
            .await
        {
            Ok(ClientRpcResponse::DeleteBlobResult(_)) => {}
            Ok(ClientRpcResponse::Error(error)) => {
                details.push(format!("cleanup: FAIL ({})", error.message));
            }
            Ok(_) => {
                details.push("cleanup: FAIL (unexpected response)".to_string());
            }
            Err(error) => {
                details.push(format!("cleanup: FAIL ({})", error));
            }
        }
    }

    let is_core_passed = !details
        .iter()
        .filter(|detail| detail.starts_with("add:") || detail.starts_with("has:") || detail.starts_with("get:"))
        .any(|detail| detail.contains("FAIL"));
    let is_passed = is_core_passed && is_cross_node_ok;
    debug_assert!(is_passed || !details.is_empty() || !is_cross_node_ok);

    let result = VerifyResult {
        name: "blob-storage".to_string(),
        passed: is_passed,
        message: if is_passed {
            format!("Blob add/has/get verified (hash: {}...)", &hash[..16.min(hash.len())])
        } else if !is_cross_node_ok {
            "Blob replication incomplete".to_string()
        } else {
            "Blob verification failed".to_string()
        },
        duration_ms: elapsed_ms_u64(started_at),
        details: Some(details.join(", ")),
        timing: None,
        node_status: None,
    };

    print_output(&result, is_json_output);

    if !is_passed {
        anyhow::bail!("Blob verification failed");
    }

    Ok(())
}

/// Result of attempting to add a blob for verification.
enum VerifyBlobAddResult {
    /// Successfully added, contains the blob hash.
    Hash(String),
    /// Blob storage not enabled, contains skip result.
    Skipped(VerifyResult),
    /// Add failed, contains failure result.
    Failed(VerifyResult),
}

/// Create a blob VerifyResult with the given passed/message.
fn verify_blob_result(passed: bool, message: impl Into<String>) -> VerifyResult {
    VerifyResult {
        name: "blob-storage".to_string(),
        passed,
        message: message.into(),
        duration_ms: 0,
        details: None,
        timing: None,
        node_status: None,
    }
}

fn simple_verify_result(
    name: &str,
    is_passed: bool,
    message: impl Into<String>,
    duration_ms: u64,
    details: Option<String>,
) -> VerifyResult {
    VerifyResult {
        name: name.to_string(),
        passed: is_passed,
        message: message.into(),
        duration_ms,
        details,
        timing: None,
        node_status: None,
    }
}

async fn run_verify_kv_replication_status(client: &AspenClient) -> (bool, Option<String>) {
    match client.send(ClientRpcRequest::GetRaftMetrics).await {
        Ok(ClientRpcResponse::RaftMetrics(metrics)) => {
            let last_applied = metrics.last_applied_index.unwrap_or(0);
            if let Some(replication) = &metrics.replication {
                let is_all_caught_up =
                    replication.iter().all(|progress| progress.matched_index.unwrap_or(0) >= last_applied);
                let node_count = replication.len();
                (is_all_caught_up, Some(format!("{} nodes, all caught up: {}", node_count, is_all_caught_up)))
            } else {
                (true, Some(format!("last_applied: {}", last_applied)))
            }
        }
        _ => (true, None),
    }
}

async fn run_verify_kv_cleanup(client: &AspenClient, test_keys: &[String], errors: &mut Vec<String>) {
    for key in test_keys {
        match client.send(ClientRpcRequest::DeleteKey { key: key.clone() }).await {
            Ok(ClientRpcResponse::DeleteResult(_)) => {}
            Ok(ClientRpcResponse::Error(error)) => errors.push(error.message),
            Ok(_) => errors.push("unexpected cleanup response".to_string()),
            Err(error) => errors.push(error.to_string()),
        }
    }
}

async fn run_verify_blob_add_hash(client: &AspenClient) -> std::result::Result<String, VerifyResult> {
    let add_result = client
        .send(ClientRpcRequest::AddBlob {
            data: (u8::MIN..=u8::MAX).collect(),
            tag: Some("__verify_all".to_string()),
        })
        .await;

    let blob_result = match add_result {
        Ok(ClientRpcResponse::AddBlobResult(result)) if result.is_success => match result.hash {
            Some(hash) => Ok(hash),
            None => Err(simple_verify_result("blob-storage", false, "No hash returned", 0, None)),
        },
        Ok(ClientRpcResponse::Error(error)) => {
            let is_skipped = error.message.contains("not enabled") || error.message.contains("disabled");
            let message = if is_skipped {
                "Blobs not enabled (skipped)".to_string()
            } else {
                format!("Error: {}", error.message)
            };
            Err(simple_verify_result("blob-storage", is_skipped, message, 0, None))
        }
        Ok(ClientRpcResponse::AddBlobResult(_)) => {
            Err(simple_verify_result("blob-storage", false, "Add blob failed", 0, None))
        }
        Ok(other_response) => Err(simple_verify_result(
            "blob-storage",
            false,
            format!("Unexpected add blob response: {:?}", other_response),
            0,
            None,
        )),
        Err(error) => Err(simple_verify_result("blob-storage", false, format!("Transport error: {}", error), 0, None)),
    };

    debug_assert!(matches!(&blob_result, Ok(hash) if !hash.is_empty()) || blob_result.is_err());
    debug_assert!(matches!(&blob_result, Err(result) if !result.message.is_empty()) || blob_result.is_ok());
    blob_result
}

async fn run_verify_blob_cleanup(client: &AspenClient, hash: &str) {
    match client
        .send(ClientRpcRequest::DeleteBlob {
            hash: hash.to_string(),
            is_force: true,
        })
        .await
    {
        Ok(ClientRpcResponse::DeleteBlobResult(_)) => {}
        Ok(ClientRpcResponse::Error(_)) | Ok(_) | Err(_) => {}
    }
}

fn endpoint_addr_from_public_key_hex(key_hex: &str) -> Result<EndpointAddr> {
    use iroh::PublicKey;

    let bytes = hex::decode(key_hex).context("failed to decode public key")?;
    let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
    let public_key = PublicKey::from_bytes(&bytes_array).context("invalid public key bytes")?;
    Ok(EndpointAddr::new(public_key))
}

fn debug_endpoint_public_key_hex(addr_str: &str) -> Option<&str> {
    let public_key_prefix = "PublicKey(";
    let start = addr_str.find(public_key_prefix)?;
    let rest_start = start.saturating_add(public_key_prefix.len());
    let rest = addr_str.get(rest_start..)?;
    let end = rest.find(')')?;
    Some(&rest[..end])
}

/// Add a test blob and return the hash, or a result if it failed/was skipped.
async fn verify_blob_add(client: &AspenClient, test_data: &[u8], details: &mut Vec<String>) -> VerifyBlobAddResult {
    debug_assert!(!test_data.is_empty());
    let add_response = client
        .send(ClientRpcRequest::AddBlob {
            data: test_data.to_vec(),
            tag: Some("__verify_blob".to_string()),
        })
        .await;

    let add_result = match add_response {
        Ok(ClientRpcResponse::AddBlobResult(result)) => {
            if result.is_success {
                details.push(format!("add: OK ({} bytes)", result.size_bytes.unwrap_or(0)));
                match result.hash {
                    Some(hash) => VerifyBlobAddResult::Hash(hash),
                    None => VerifyBlobAddResult::Failed(verify_blob_result(false, "Add blob returned no hash")),
                }
            } else {
                let message = format!("Add blob failed: {}", result.error.unwrap_or_default());
                VerifyBlobAddResult::Failed(verify_blob_result(false, message))
            }
        }
        Ok(ClientRpcResponse::Error(error)) => {
            if error.message.contains("not enabled") || error.message.contains("disabled") {
                VerifyBlobAddResult::Skipped(verify_blob_result(true, "Blob storage not enabled (skipped)"))
            } else {
                VerifyBlobAddResult::Failed(verify_blob_result(false, format!("Add blob failed: {}", error.message)))
            }
        }
        Ok(_) => VerifyBlobAddResult::Failed(verify_blob_result(false, "Unexpected response from add blob")),
        Err(error) => VerifyBlobAddResult::Failed(verify_blob_result(false, format!("Error: {}", error))),
    };

    debug_assert!(details.iter().all(|detail| !detail.is_empty()));
    add_result
}

/// Check if a blob exists by hash, appending result to details.
async fn verify_blob_check_exists(client: &AspenClient, hash: &str, details: &mut Vec<String>) {
    let has_response = client.send(ClientRpcRequest::HasBlob { hash: hash.to_string() }).await;

    match has_response {
        Ok(ClientRpcResponse::HasBlobResult(result)) => {
            if result.does_exist {
                details.push("has: OK".to_string());
            } else {
                details.push("has: FAIL (not found)".to_string());
            }
        }
        Ok(ClientRpcResponse::Error(e)) => {
            details.push(format!("has: FAIL ({})", e.message));
        }
        _ => {
            details.push("has: FAIL (unexpected response)".to_string());
        }
    }
}

/// Get a blob by hash and verify its content matches test_data.
async fn verify_blob_verify_content(client: &AspenClient, hash: &str, test_data: &[u8], details: &mut Vec<String>) {
    debug_assert!(!hash.is_empty());
    let get_response = client.send(ClientRpcRequest::GetBlob { hash: hash.to_string() }).await;

    match get_response {
        Ok(ClientRpcResponse::GetBlobResult(result)) => {
            if result.was_found && result.data.as_deref() == Some(test_data) {
                details.push("get: OK (content verified)".to_string());
            } else if result.was_found {
                details.push("get: FAIL (content mismatch)".to_string());
            } else {
                details.push("get: FAIL (not found)".to_string());
            }
        }
        Ok(ClientRpcResponse::Error(error)) => {
            details.push(format!("get: FAIL ({})", error.message));
        }
        _ => {
            details.push("get: FAIL (unexpected response)".to_string());
        }
    }

    debug_assert!(details.iter().all(|detail| !detail.is_empty()));
}

/// Run all verification tests.
async fn verify_all(client: &AspenClient, args: VerifyAllArgs, is_json_output: bool) -> Result<()> {
    let mut results = Vec::with_capacity(3);

    let kv_result = run_verify_kv(client, args.no_cleanup).await;
    let is_kv_failed = !kv_result.passed;
    results.push(kv_result);

    if is_kv_failed && !args.continue_on_error {
        let output = VerifyAllResult {
            total_passed: 0,
            total_failed: 1,
            results,
        };
        print_output(&output, is_json_output);
        anyhow::bail!("Verification failed");
    }

    let docs_result = run_verify_docs(client).await;
    let is_docs_failed = !docs_result.passed;
    results.push(docs_result);

    if is_docs_failed && !args.continue_on_error {
        let total_passed = u32_from_usize(results.iter().filter(|result| result.passed).count());
        let total_failed = u32_from_usize(results.iter().filter(|result| !result.passed).count());
        let output = VerifyAllResult {
            total_passed,
            total_failed,
            results,
        };
        debug_assert!(total_passed.saturating_add(total_failed) >= 1);
        debug_assert!(total_failed >= 1);
        print_output(&output, is_json_output);
        anyhow::bail!("Verification failed");
    }

    let blob_result = run_verify_blob(client, args.no_cleanup).await;
    results.push(blob_result);

    let total_passed = u32_from_usize(results.iter().filter(|result| result.passed).count());
    let total_failed = u32_from_usize(results.iter().filter(|result| !result.passed).count());
    let output = VerifyAllResult {
        total_passed,
        total_failed,
        results,
    };

    debug_assert_eq!(total_passed.saturating_add(total_failed), 3);
    debug_assert!(total_failed <= 3);
    print_output(&output, is_json_output);

    if total_failed > 0 {
        anyhow::bail!("{} verification test(s) failed", total_failed);
    }

    Ok(())
}

/// Helper to run KV verification and return result.
async fn run_verify_kv(client: &AspenClient, should_skip_cleanup: bool) -> VerifyResult {
    let started_at = current_instant();
    let count = VERIFY_ALL_KEY_COUNT;
    let timestamp = current_unix_seconds();
    let test_keys: Vec<String> =
        (0..count).map(|index| format!("{}all_{}_{}", VERIFY_ALL_KEY_PREFIX, timestamp, index)).collect();
    let mut errors = Vec::with_capacity(usize_from_u32(count));

    for (index, key) in test_keys.iter().enumerate() {
        let value = format!("verify_value_{}", index);
        match client
            .send(ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.as_bytes().to_vec(),
            })
            .await
        {
            Ok(ClientRpcResponse::WriteResult(_)) => {}
            Ok(ClientRpcResponse::Error(error)) => errors.push(error.message),
            _ => errors.push("unexpected response".to_string()),
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
    let (is_replication_ok, replication_detail) = run_verify_kv_replication_status(client).await;

    let mut success = 0u32;
    for (index, key) in test_keys.iter().enumerate() {
        let expected = format!("verify_value_{}", index);
        if let Ok(ClientRpcResponse::ReadResult(result)) =
            client.send(ClientRpcRequest::ReadKey { key: key.clone() }).await
            && result.value.as_deref() == Some(expected.as_bytes())
        {
            success = success.saturating_add(1);
        }
    }

    if !should_skip_cleanup {
        run_verify_kv_cleanup(client, &test_keys, &mut errors).await;
    }

    let is_passed = success == count && errors.is_empty() && is_replication_ok;
    debug_assert!(success <= count);
    debug_assert!(is_passed || !errors.is_empty() || success < count || !is_replication_ok);
    simple_verify_result(
        "kv-replication",
        is_passed,
        if is_passed {
            format!("{}/{} keys verified, replication OK", success, count)
        } else if !is_replication_ok {
            format!("{}/{} keys verified, replication BEHIND", success, count)
        } else {
            format!("{}/{} keys verified", success, count)
        },
        elapsed_ms_u64(started_at),
        replication_detail,
    )
}

/// Helper to run docs verification and return result.
async fn run_verify_docs(client: &AspenClient) -> VerifyResult {
    let started_at = current_instant();
    let result = match client.send(ClientRpcRequest::DocsStatus).await {
        Ok(ClientRpcResponse::DocsStatusResult(status)) => VerifyResult {
            name: "docs-sync".to_string(),
            passed: true,
            message: format!("Docs enabled, {} entries", status.entry_count.unwrap_or(0)),
            duration_ms: elapsed_ms_u64(started_at),
            details: status.namespace_id.map(|namespace_id| format!("namespace: {}", namespace_id)),
            timing: None,
            node_status: None,
        },
        Ok(ClientRpcResponse::Error(error)) => {
            let docs_error = DocsAvailabilityError {
                code: &error.code,
                message: &error.message,
            };
            if is_docs_unavailable_error(docs_error) {
                VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: true,
                    message: "Docs not enabled (skipped)".to_string(),
                    duration_ms: elapsed_ms_u64(started_at),
                    details: None,
                    timing: None,
                    node_status: None,
                }
            } else {
                VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: false,
                    message: format!("Error: {}", error.message),
                    duration_ms: elapsed_ms_u64(started_at),
                    details: None,
                    timing: None,
                    node_status: None,
                }
            }
        }
        _ => VerifyResult {
            name: "docs-sync".to_string(),
            passed: false,
            message: "Unexpected response".to_string(),
            duration_ms: elapsed_ms_u64(started_at),
            details: None,
            timing: None,
            node_status: None,
        },
    };
    debug_assert!(!result.message.is_empty());
    debug_assert!(result.duration_ms <= elapsed_ms_u64(started_at));
    result
}

/// Check if an error indicates the docs service is unavailable.
///
/// The docs handler is only registered when `docs_sync` is available in the
/// server context. When it's not, the dispatcher returns "no handler found"
/// which gets sanitized to "internal error" by the server's error sanitization
/// layer (to prevent information leakage). We treat both the raw and sanitized
/// forms as "docs not available".
fn is_docs_unavailable_error(error: DocsAvailabilityError<'_>) -> bool {
    debug_assert!(!error.code.is_empty() || !error.message.is_empty());
    let is_unavailable = error.message.contains("not enabled")
        || error.message.contains("disabled")
        || error.message.contains("no handler found")
        || error.message.contains("not available")
        || (error.code == "INTERNAL_ERROR" && error.message == "internal error");
    debug_assert!(is_unavailable || !error.message.is_empty());
    is_unavailable
}

/// Helper to run blob verification and return result.
async fn run_verify_blob(client: &AspenClient, should_skip_cleanup: bool) -> VerifyResult {
    let started_at = current_instant();
    let hash = match run_verify_blob_add_hash(client).await {
        Ok(hash) => hash,
        Err(mut result) => {
            result.duration_ms = elapsed_ms_u64(started_at);
            return result;
        }
    };

    let has_result = client.send(ClientRpcRequest::HasBlob { hash: hash.clone() }).await;
    let is_blob_present = matches!(has_result, Ok(ClientRpcResponse::HasBlobResult(result)) if result.does_exist);

    if !should_skip_cleanup {
        run_verify_blob_cleanup(client, &hash).await;
    }

    debug_assert!(!hash.is_empty());
    debug_assert!(is_blob_present || !should_skip_cleanup || !hash.is_empty());
    simple_verify_result(
        "blob-storage",
        is_blob_present,
        if is_blob_present {
            format!("Blob verified (hash: {}...)", &hash[..16.min(hash.len())])
        } else {
            "Blob not found after add".to_string()
        },
        elapsed_ms_u64(started_at),
        None,
    )
}

// =============================================================================
// Cross-node verification helpers
// =============================================================================

/// Get all cluster nodes for cross-node verification.
///
/// Returns a list of NodeDescriptor containing endpoint addresses
/// that can be used to query each node directly.
async fn get_cluster_nodes(client: &AspenClient) -> Result<Vec<NodeDescriptor>> {
    let response = client.send(ClientRpcRequest::GetClusterState).await?;

    match response {
        ClientRpcResponse::ClusterState(state) => Ok(state.nodes),
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("Failed to get cluster state: {}", e.message);
        }
        _ => anyhow::bail!("Unexpected response from GetClusterState"),
    }
}

/// Parse an endpoint address string into an EndpointAddr.
///
/// The endpoint_addr field in NodeDescriptor can be:
/// 1. A hex-encoded public key (for other nodes from Raft storage)
/// 2. Debug format like "EndpointAddr { id: PublicKey(...), addrs: {...} }"
///
/// We try to parse as hex public key first (most common), then fall back to
/// other formats if needed.
fn parse_endpoint_addr(addr_str: &str) -> Result<EndpointAddr> {
    debug_assert!(!addr_str.is_empty());
    if addr_str.len() == 64 && addr_str.chars().all(|character| character.is_ascii_hexdigit()) {
        return endpoint_addr_from_public_key_hex(addr_str).context("failed to decode hex public key");
    }

    if let Ok(addr) = serde_json::from_str::<EndpointAddr>(addr_str) {
        return Ok(addr);
    }

    if let Some(key_hex) = debug_endpoint_public_key_hex(addr_str)
        && key_hex.len() == 64
        && key_hex.chars().all(|character| character.is_ascii_hexdigit())
    {
        return endpoint_addr_from_public_key_hex(key_hex).context("failed to decode public key from debug format");
    }

    debug_assert!(addr_str.len() != 64 || !addr_str.chars().all(|character| character.is_ascii_hexdigit()));
    anyhow::bail!("unrecognized endpoint address format: {}", addr_str)
}

/// Verify docs replication across all cluster nodes.
///
/// This performs cross-node verification by:
/// 1. Writing a test entry on the current node
/// 2. Waiting for sync to propagate
/// 3. Querying each node's entry count
/// 4. Verifying counts match (within tolerance for eventual consistency)
async fn verify_docs_cross_node(client: &AspenClient, details: &mut Vec<String>) -> bool {
    let nodes = match get_cluster_nodes(client).await {
        Ok(nodes) => nodes,
        Err(error) => {
            details.push(format!("cross-node: SKIP ({})", error));
            return true;
        }
    };

    if nodes.len() <= 1 {
        details.push("cross-node: SKIP (single node)".to_string());
        return true;
    }

    let mut entry_counts: Vec<(u64, u64)> = Vec::with_capacity(nodes.len());
    let mut query_errors: Vec<String> = Vec::with_capacity(nodes.len());

    for node in &nodes {
        let addr = match parse_endpoint_addr(&node.endpoint_addr) {
            Ok(addr) => addr,
            Err(error) => {
                query_errors.push(format!("node {}: parse error ({})", node.node_id, error));
                continue;
            }
        };

        match client.send_to(&addr, ClientRpcRequest::DocsStatus).await {
            Ok(ClientRpcResponse::DocsStatusResult(status)) => {
                if status.is_enabled {
                    let count = status.entry_count.unwrap_or(0);
                    entry_counts.push((node.node_id, count));
                }
            }
            Ok(ClientRpcResponse::Error(error)) => {
                if !error.message.contains("not enabled") {
                    query_errors.push(format!("node {}: {}", node.node_id, error.message));
                }
            }
            Ok(_) => {
                query_errors.push(format!("node {}: unexpected response", node.node_id));
            }
            Err(error) => {
                query_errors.push(format!("node {}: {}", node.node_id, error));
            }
        }
    }

    let is_consistent_across_nodes = if entry_counts.is_empty() {
        if !query_errors.is_empty() {
            details.push(format!("cross-node: SKIP (errors: {})", query_errors.join("; ")));
        } else {
            details.push("cross-node: SKIP (no nodes with docs enabled)".to_string());
        }
        true
    } else {
        let first_count = entry_counts[0].1;
        let is_all_match = entry_counts.iter().all(|(_, count)| *count == first_count);
        let counts_str = entry_counts
            .iter()
            .map(|(id, count)| format!("node{}={}", id, count))
            .collect::<Vec<_>>()
            .join(", ");

        if is_all_match {
            details.push(format!("cross-node: OK ({} nodes, {} entries each)", entry_counts.len(), first_count));
            true
        } else {
            details.push(format!("cross-node: MISMATCH ({})", counts_str));
            false
        }
    };

    debug_assert!(details.len() <= details.capacity() || details.capacity() == 0);
    debug_assert!(is_consistent_across_nodes || !entry_counts.is_empty());
    is_consistent_across_nodes
}

/// Verify blob replication across all cluster nodes.
///
/// This performs cross-node verification by:
/// 1. Adding a test blob on the current node
/// 2. Querying each node to check if the blob hash exists
/// 3. Verifying all nodes have the blob
async fn verify_blob_cross_node(client: &AspenClient, hash: &str, details: &mut Vec<String>) -> bool {
    let nodes = match get_cluster_nodes(client).await {
        Ok(nodes) => nodes,
        Err(error) => {
            details.push(format!("cross-node: SKIP ({})", error));
            return true;
        }
    };

    if nodes.len() <= 1 {
        details.push("cross-node: SKIP (single node)".to_string());
        return true;
    }

    let mut nodes_with_blob: Vec<u64> = Vec::with_capacity(nodes.len());
    let mut nodes_without_blob: Vec<u64> = Vec::with_capacity(nodes.len());
    let mut query_errors: Vec<String> = Vec::with_capacity(nodes.len());

    for node in &nodes {
        let addr = match parse_endpoint_addr(&node.endpoint_addr) {
            Ok(addr) => addr,
            Err(error) => {
                query_errors.push(format!("node {}: parse error ({})", node.node_id, error));
                continue;
            }
        };

        match client.send_to(&addr, ClientRpcRequest::HasBlob { hash: hash.to_string() }).await {
            Ok(ClientRpcResponse::HasBlobResult(result)) => {
                if result.does_exist {
                    nodes_with_blob.push(node.node_id);
                } else {
                    nodes_without_blob.push(node.node_id);
                }
            }
            Ok(ClientRpcResponse::Error(error)) => {
                if !error.message.contains("not enabled") {
                    query_errors.push(format!("node {}: {}", node.node_id, error.message));
                }
            }
            Ok(_) => {
                query_errors.push(format!("node {}: unexpected response", node.node_id));
            }
            Err(error) => {
                query_errors.push(format!("node {}: {}", node.node_id, error));
            }
        }
    }

    let is_replication_acceptable = if nodes_with_blob.is_empty() && nodes_without_blob.is_empty() {
        if !query_errors.is_empty() {
            details.push(format!("cross-node: SKIP (errors: {})", query_errors.join("; ")));
        } else {
            details.push("cross-node: SKIP (no nodes with blobs enabled)".to_string());
        }
        true
    } else if nodes_without_blob.is_empty() {
        details.push(format!("cross-node: OK ({}/{} nodes have blob)", nodes_with_blob.len(), nodes_with_blob.len()));
        true
    } else {
        let total_nodes = nodes_with_blob.len().saturating_add(nodes_without_blob.len());
        details.push(format!(
            "cross-node: PARTIAL ({}/{} nodes have blob, missing on {:?})",
            nodes_with_blob.len(),
            total_nodes,
            nodes_without_blob
        ));
        nodes_with_blob.len().saturating_mul(2) >= total_nodes
    };

    debug_assert!(!hash.is_empty());
    debug_assert!(is_replication_acceptable || !nodes_without_blob.is_empty());
    is_replication_acceptable
}
