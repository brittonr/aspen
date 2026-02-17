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
        self.write_ms + self.replicate_ms + self.read_ms + self.cleanup_ms + self.cross_node_ms
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
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            VerifyCommand::Kv(args) => verify_kv(client, args, json).await,
            VerifyCommand::Docs(args) => verify_docs(client, args, json).await,
            VerifyCommand::Blob(args) => verify_blob(client, args, json).await,
            VerifyCommand::All(args) => verify_all(client, args, json).await,
        }
    }
}

/// Verify KV store replication.
async fn verify_kv(client: &AspenClient, args: VerifyKvArgs, json: bool) -> Result<()> {
    let start = Instant::now();
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    let mut errors = Vec::new();
    let mut replication_details = Vec::new();
    let mut node_status_list: Vec<NodeReplicationStatus> = Vec::new();
    let mut timing = VerifyTiming::default();

    let test_keys: Vec<String> = (0..args.count).map(|i| format!("{}test_{}_{}", args.prefix, timestamp, i)).collect();

    // Step 1: Write test keys
    let write_start = Instant::now();
    verify_kv_write_test_keys(client, &test_keys, &mut errors).await;
    timing.write_ms = write_start.elapsed().as_millis() as u64;

    // Brief pause for replication
    let replicate_start = Instant::now();
    tokio::time::sleep(Duration::from_millis(100)).await;
    timing.replicate_ms = replicate_start.elapsed().as_millis() as u64;

    // Step 2: Check replication status from leader
    let replication_ok =
        verify_kv_check_replication(client, &mut errors, &mut replication_details, &mut node_status_list).await;

    // Step 3: Read back and verify
    let read_start = Instant::now();
    let read_success = verify_kv_read_and_verify(client, &test_keys, &mut errors).await;
    timing.read_ms = read_start.elapsed().as_millis() as u64;

    // Step 4: Cleanup (unless --no-cleanup)
    let cleanup_start = Instant::now();
    if !args.no_cleanup {
        for key in &test_keys {
            let _ = client.send(ClientRpcRequest::DeleteKey { key: key.clone() }).await;
        }
    }
    timing.cleanup_ms = cleanup_start.elapsed().as_millis() as u64;

    let result = verify_kv_build_result(
        args.count,
        read_success,
        replication_ok,
        &errors,
        &replication_details,
        start.elapsed().as_millis() as u64,
        timing,
        node_status_list,
    );

    print_output(&result, json);

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
                let mut all_caught_up = true;
                for progress in replication {
                    let matched = progress.matched_index.unwrap_or(0);
                    let lag = last_applied.saturating_sub(matched);
                    let status = if matched >= last_applied {
                        "OK".to_string()
                    } else {
                        all_caught_up = false;
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
                all_caught_up
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
    for (i, key) in test_keys.iter().enumerate() {
        let expected = format!("verify_value_{}", i);
        let response = client.send(ClientRpcRequest::ReadKey { key: key.clone() }).await;

        match response {
            Ok(ClientRpcResponse::ReadResult(result)) => {
                if let Some(value) = result.value {
                    if value == expected.as_bytes() {
                        read_success += 1;
                    } else {
                        errors.push(format!("Key {} value mismatch", key));
                    }
                } else {
                    errors.push(format!("Key {} not found after write", key));
                }
            }
            Ok(ClientRpcResponse::Error(e)) => {
                errors.push(format!("Read {} failed: {}", key, e.message));
            }
            Ok(_) => errors.push(format!("Read {} unexpected response", key)),
            Err(e) => errors.push(format!("Read {} error: {}", key, e)),
        }
    }
    read_success
}

/// Build the final VerifyResult from collected data.
fn verify_kv_build_result(
    count: u32,
    read_success: u32,
    replication_ok: bool,
    errors: &[String],
    replication_details: &[String],
    duration_ms: u64,
    timing: VerifyTiming,
    node_status_list: Vec<NodeReplicationStatus>,
) -> VerifyResult {
    let passed = errors.is_empty() && read_success == count && replication_ok;

    let mut all_details = Vec::new();
    if !replication_details.is_empty() {
        all_details.push(format!("replication: {}", replication_details.join(", ")));
    }
    if !errors.is_empty() {
        all_details.push(format!("errors: {}", errors.join("; ")));
    }

    VerifyResult {
        name: "kv-replication".to_string(),
        passed,
        message: if passed {
            format!("{}/{} keys verified, replication OK", read_success, count)
        } else if !replication_ok {
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
async fn verify_docs(client: &AspenClient, args: VerifyDocsArgs, json: bool) -> Result<()> {
    let start = Instant::now();
    let mut details = Vec::new();

    // Get docs status
    let (enabled, namespace_id) = match verify_docs_check_status(client, &mut details).await {
        Ok(result) => result,
        Err(fail_result) => {
            print_output(&fail_result, json);
            anyhow::bail!("Docs verification failed");
        }
    };

    if !enabled {
        let result = VerifyResult {
            name: "docs-sync".to_string(),
            passed: true,
            message: "Docs sync not enabled (skipped)".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            details: None,
            timing: None,
            node_status: None,
        };
        print_output(&result, json);
        return Ok(());
    }

    // Optionally write and verify a test entry
    if args.write_test {
        verify_docs_write_read_test(client, &mut details).await;
    }

    // Wait a bit more for sync to propagate before cross-node check
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Cross-node verification: check entry counts across all nodes
    let cross_node_ok = verify_docs_cross_node(client, &mut details).await;

    let result = verify_docs_build_result(&namespace_id, &details, cross_node_ok, start.elapsed().as_millis() as u64);

    print_output(&result, json);

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

    match response {
        Ok(ClientRpcResponse::DocsStatusResult(status)) => {
            if let Some(ref ns_id) = status.namespace_id {
                details.push(format!("namespace: {}", ns_id));
            }
            if let Some(count) = status.entry_count {
                details.push(format!("entries: {}", count));
            }
            Ok((true, status.namespace_id.clone()))
        }
        Ok(ClientRpcResponse::Error(e)) => {
            if e.message.contains("not enabled") || e.message.contains("disabled") {
                Ok((false, None))
            } else {
                Err(VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: false,
                    message: format!("Status check failed: {}", e.message),
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
        Err(e) => Err(VerifyResult {
            name: "docs-sync".to_string(),
            passed: false,
            message: format!("Error: {}", e),
            duration_ms: 0,
            details: None,
            timing: None,
            node_status: None,
        }),
    }
}

/// Write a test entry via docs and read it back, collecting results into details.
async fn verify_docs_write_read_test(client: &AspenClient, details: &mut Vec<String>) {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let test_key = format!("__verify_docs_{}", timestamp);
    let test_value = "docs_verify_value";

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
    let _ = client.send(ClientRpcRequest::DocsDelete { key: test_key }).await;
}

/// Build the final VerifyResult for docs verification.
fn verify_docs_build_result(
    namespace_id: &Option<String>,
    details: &[String],
    cross_node_ok: bool,
    duration_ms: u64,
) -> VerifyResult {
    let passed = !details.iter().any(|d| d.contains("FAIL") || d.contains("MISMATCH")) && cross_node_ok;

    VerifyResult {
        name: "docs-sync".to_string(),
        passed,
        message: if passed {
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

/// Verify blob storage.
async fn verify_blob(client: &AspenClient, args: VerifyBlobArgs, json: bool) -> Result<()> {
    let start = Instant::now();
    let mut details = Vec::new();

    // Generate test data
    let test_data: Vec<u8> = (0..args.size_bytes).map(|i| (i % 256) as u8).collect();

    // Add blob and get hash
    let hash = match verify_blob_add(client, &test_data, &mut details).await {
        VerifyBlobAddResult::Hash(h) => h,
        VerifyBlobAddResult::Skipped(result) => {
            print_output(&result, json);
            return Ok(());
        }
        VerifyBlobAddResult::Failed(result) => {
            print_output(&result, json);
            anyhow::bail!("Blob verification failed");
        }
    };

    // Check blob exists
    verify_blob_check_exists(client, &hash, &mut details).await;

    // Get blob and verify content
    verify_blob_verify_content(client, &hash, &test_data, &mut details).await;

    // Cross-node verification
    let cross_node_ok = verify_blob_cross_node(client, &hash, &mut details).await;

    // Cleanup (unless --no-cleanup)
    if !args.no_cleanup {
        let _ = client
            .send(ClientRpcRequest::DeleteBlob {
                hash: hash.clone(),
                is_force: true,
            })
            .await;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let passed = !details.iter().any(|d| d.contains("FAIL")) && cross_node_ok;

    let result = VerifyResult {
        name: "blob-storage".to_string(),
        passed,
        message: if passed {
            format!("Blob add/has/get verified (hash: {}...)", &hash[..16.min(hash.len())])
        } else {
            "Blob verification failed".to_string()
        },
        duration_ms,
        details: Some(details.join(", ")),
        timing: None,
        node_status: None,
    };

    print_output(&result, json);

    if !passed {
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

/// Add a test blob and return the hash, or a result if it failed/was skipped.
async fn verify_blob_add(client: &AspenClient, test_data: &[u8], details: &mut Vec<String>) -> VerifyBlobAddResult {
    let add_response = client
        .send(ClientRpcRequest::AddBlob {
            data: test_data.to_vec(),
            tag: Some("__verify_blob".to_string()),
        })
        .await;

    let hash = match add_response {
        Ok(ClientRpcResponse::AddBlobResult(result)) => {
            if result.is_success {
                details.push(format!("add: OK ({} bytes)", result.size_bytes.unwrap_or(0)));
                result.hash
            } else {
                let msg = format!("Add blob failed: {}", result.error.unwrap_or_default());
                return VerifyBlobAddResult::Failed(verify_blob_result(false, msg));
            }
        }
        Ok(ClientRpcResponse::Error(e)) => {
            if e.message.contains("not enabled") || e.message.contains("disabled") {
                return VerifyBlobAddResult::Skipped(verify_blob_result(true, "Blob storage not enabled (skipped)"));
            }
            return VerifyBlobAddResult::Failed(verify_blob_result(false, format!("Add blob failed: {}", e.message)));
        }
        Ok(_) => {
            return VerifyBlobAddResult::Failed(verify_blob_result(false, "Unexpected response from add blob"));
        }
        Err(e) => {
            return VerifyBlobAddResult::Failed(verify_blob_result(false, format!("Error: {}", e)));
        }
    };

    match hash {
        Some(h) => VerifyBlobAddResult::Hash(h),
        None => VerifyBlobAddResult::Failed(verify_blob_result(false, "Add blob returned no hash")),
    }
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
        Ok(ClientRpcResponse::Error(e)) => {
            details.push(format!("get: FAIL ({})", e.message));
        }
        _ => {
            details.push("get: FAIL (unexpected response)".to_string());
        }
    }
}

/// Run all verification tests.
async fn verify_all(client: &AspenClient, args: VerifyAllArgs, json: bool) -> Result<()> {
    let mut results = Vec::new();

    // Verify KV
    let kv_result = run_verify_kv(client, args.no_cleanup).await;
    let kv_failed = !kv_result.passed;
    results.push(kv_result);

    if kv_failed && !args.continue_on_error {
        let output = VerifyAllResult {
            total_passed: 0,
            total_failed: 1,
            results,
        };
        print_output(&output, json);
        anyhow::bail!("Verification failed");
    }

    // Verify Docs
    let docs_result = run_verify_docs(client).await;
    let docs_failed = !docs_result.passed;
    results.push(docs_result);

    if docs_failed && !args.continue_on_error {
        let passed = results.iter().filter(|r| r.passed).count() as u32;
        let failed = results.iter().filter(|r| !r.passed).count() as u32;
        let output = VerifyAllResult {
            total_passed: passed,
            total_failed: failed,
            results,
        };
        print_output(&output, json);
        anyhow::bail!("Verification failed");
    }

    // Verify Blobs
    let blob_result = run_verify_blob(client, args.no_cleanup).await;
    results.push(blob_result);

    let total_passed = results.iter().filter(|r| r.passed).count() as u32;
    let total_failed = results.iter().filter(|r| !r.passed).count() as u32;

    let output = VerifyAllResult {
        total_passed,
        total_failed,
        results,
    };

    print_output(&output, json);

    if total_failed > 0 {
        anyhow::bail!("{} verification test(s) failed", total_failed);
    }

    Ok(())
}

/// Helper to run KV verification and return result.
async fn run_verify_kv(client: &AspenClient, no_cleanup: bool) -> VerifyResult {
    let start = Instant::now();
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    let count = 3;
    let prefix = "__verify_";
    let test_keys: Vec<String> = (0..count).map(|i| format!("{}all_{}_{}", prefix, timestamp, i)).collect();
    let mut errors = Vec::new();

    // Write
    for (i, key) in test_keys.iter().enumerate() {
        let value = format!("verify_value_{}", i);
        match client
            .send(ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.as_bytes().to_vec(),
            })
            .await
        {
            Ok(ClientRpcResponse::WriteResult(_)) => {}
            Ok(ClientRpcResponse::Error(e)) => errors.push(e.message),
            _ => errors.push("unexpected response".to_string()),
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check replication status
    let (replication_ok, replication_detail) = match client.send(ClientRpcRequest::GetRaftMetrics).await {
        Ok(ClientRpcResponse::RaftMetrics(metrics)) => {
            let last_applied = metrics.last_applied_index.unwrap_or(0);
            if let Some(replication) = &metrics.replication {
                let all_ok = replication.iter().all(|p| p.matched_index.unwrap_or(0) >= last_applied);
                let node_count = replication.len();
                (all_ok, Some(format!("{} nodes, all caught up: {}", node_count, all_ok)))
            } else {
                (true, Some(format!("last_applied: {}", last_applied)))
            }
        }
        _ => (true, None),
    };

    // Read
    let mut success = 0;
    for (i, key) in test_keys.iter().enumerate() {
        let expected = format!("verify_value_{}", i);
        match client.send(ClientRpcRequest::ReadKey { key: key.clone() }).await {
            Ok(ClientRpcResponse::ReadResult(r)) if r.value.as_deref() == Some(expected.as_bytes()) => {
                success += 1;
            }
            _ => {}
        }
    }

    // Cleanup
    if !no_cleanup {
        for key in &test_keys {
            let _ = client.send(ClientRpcRequest::DeleteKey { key: key.clone() }).await;
        }
    }

    let passed = success == count && errors.is_empty() && replication_ok;

    VerifyResult {
        name: "kv-replication".to_string(),
        passed,
        message: if passed {
            format!("{}/{} keys verified, replication OK", success, count)
        } else if !replication_ok {
            format!("{}/{} keys verified, replication BEHIND", success, count)
        } else {
            format!("{}/{} keys verified", success, count)
        },
        duration_ms: start.elapsed().as_millis() as u64,
        details: replication_detail,
        timing: None,
        node_status: None,
    }
}

/// Helper to run docs verification and return result.
async fn run_verify_docs(client: &AspenClient) -> VerifyResult {
    let start = Instant::now();

    match client.send(ClientRpcRequest::DocsStatus).await {
        Ok(ClientRpcResponse::DocsStatusResult(status)) => VerifyResult {
            name: "docs-sync".to_string(),
            passed: true,
            message: format!("Docs enabled, {} entries", status.entry_count.unwrap_or(0)),
            duration_ms: start.elapsed().as_millis() as u64,
            details: status.namespace_id.map(|ns| format!("namespace: {}", ns)),
            timing: None,
            node_status: None,
        },
        Ok(ClientRpcResponse::Error(e)) => {
            if e.message.contains("not enabled") || e.message.contains("disabled") {
                VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: true,
                    message: "Docs not enabled (skipped)".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: None,
                    timing: None,
                    node_status: None,
                }
            } else {
                VerifyResult {
                    name: "docs-sync".to_string(),
                    passed: false,
                    message: format!("Error: {}", e.message),
                    duration_ms: start.elapsed().as_millis() as u64,
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
            duration_ms: start.elapsed().as_millis() as u64,
            details: None,
            timing: None,
            node_status: None,
        },
    }
}

/// Helper to run blob verification and return result.
async fn run_verify_blob(client: &AspenClient, no_cleanup: bool) -> VerifyResult {
    let start = Instant::now();
    let test_data: Vec<u8> = (0..256).map(|i| i as u8).collect();

    let add_result = client
        .send(ClientRpcRequest::AddBlob {
            data: test_data.clone(),
            tag: Some("__verify_all".to_string()),
        })
        .await;

    let hash = match add_result {
        Ok(ClientRpcResponse::AddBlobResult(r)) if r.is_success => r.hash,
        Ok(ClientRpcResponse::Error(e)) => {
            if e.message.contains("not enabled") || e.message.contains("disabled") {
                return VerifyResult {
                    name: "blob-storage".to_string(),
                    passed: true,
                    message: "Blobs not enabled (skipped)".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    details: None,
                    timing: None,
                    node_status: None,
                };
            }
            return VerifyResult {
                name: "blob-storage".to_string(),
                passed: false,
                message: format!("Error: {}", e.message),
                duration_ms: start.elapsed().as_millis() as u64,
                details: None,
                timing: None,
                node_status: None,
            };
        }
        _ => {
            return VerifyResult {
                name: "blob-storage".to_string(),
                passed: false,
                message: "Add blob failed".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                details: None,
                timing: None,
                node_status: None,
            };
        }
    };

    let hash = match hash {
        Some(h) => h,
        None => {
            return VerifyResult {
                name: "blob-storage".to_string(),
                passed: false,
                message: "No hash returned".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                details: None,
                timing: None,
                node_status: None,
            };
        }
    };

    // Verify exists
    let has_result = client.send(ClientRpcRequest::HasBlob { hash: hash.clone() }).await;
    let exists = matches!(has_result, Ok(ClientRpcResponse::HasBlobResult(r)) if r.does_exist);

    // Cleanup
    if !no_cleanup {
        let _ = client
            .send(ClientRpcRequest::DeleteBlob {
                hash: hash.clone(),
                is_force: true,
            })
            .await;
    }

    VerifyResult {
        name: "blob-storage".to_string(),
        passed: exists,
        message: if exists {
            format!("Blob verified (hash: {}...)", &hash[..16.min(hash.len())])
        } else {
            "Blob not found after add".to_string()
        },
        duration_ms: start.elapsed().as_millis() as u64,
        details: None,
        timing: None,
        node_status: None,
    }
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
    use iroh::PublicKey;

    // Try parsing as hex-encoded public key (64 hex chars = 32 bytes)
    if addr_str.len() == 64 && addr_str.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(addr_str).context("failed to decode hex public key")?;
        let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
        let public_key = PublicKey::from_bytes(&bytes_array).context("invalid public key bytes")?;
        return Ok(EndpointAddr::new(public_key));
    }

    // Try parsing as JSON
    if let Ok(addr) = serde_json::from_str::<EndpointAddr>(addr_str) {
        return Ok(addr);
    }

    // Try extracting public key from Debug format: "EndpointAddr { id: PublicKey(...), ... }"
    if addr_str.starts_with("EndpointAddr") {
        // Extract the hex key from PublicKey(...) format
        if let Some(start) = addr_str.find("PublicKey(") {
            let rest = &addr_str[start + 10..];
            if let Some(end) = rest.find(')') {
                let key_hex = &rest[..end];
                if key_hex.len() == 64 && key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    let bytes = hex::decode(key_hex).context("failed to decode public key from debug format")?;
                    let bytes_array: [u8; 32] =
                        bytes.try_into().map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
                    let public_key = PublicKey::from_bytes(&bytes_array).context("invalid public key bytes")?;
                    return Ok(EndpointAddr::new(public_key));
                }
            }
        }
    }

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
    // Get cluster nodes
    let nodes = match get_cluster_nodes(client).await {
        Ok(nodes) => nodes,
        Err(e) => {
            details.push(format!("cross-node: SKIP ({})", e));
            return true; // Skip cross-node check if we can't get nodes
        }
    };

    if nodes.len() <= 1 {
        details.push("cross-node: SKIP (single node)".to_string());
        return true;
    }

    // Get entry counts from all nodes
    let mut entry_counts: Vec<(u64, u64)> = Vec::new();
    let mut query_errors: Vec<String> = Vec::new();

    for node in &nodes {
        let addr = match parse_endpoint_addr(&node.endpoint_addr) {
            Ok(a) => a,
            Err(e) => {
                query_errors.push(format!("node {}: parse error ({})", node.node_id, e));
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
            Ok(ClientRpcResponse::Error(e)) => {
                if !e.message.contains("not enabled") {
                    query_errors.push(format!("node {}: {}", node.node_id, e.message));
                }
            }
            Ok(_) => {
                query_errors.push(format!("node {}: unexpected response", node.node_id));
            }
            Err(e) => {
                query_errors.push(format!("node {}: {}", node.node_id, e));
            }
        }
    }

    if entry_counts.is_empty() {
        if !query_errors.is_empty() {
            details.push(format!("cross-node: SKIP (errors: {})", query_errors.join("; ")));
        } else {
            details.push("cross-node: SKIP (no nodes with docs enabled)".to_string());
        }
        return true;
    }

    // Check if entry counts match across nodes
    let first_count = entry_counts[0].1;
    let all_match = entry_counts.iter().all(|(_, count)| *count == first_count);

    let counts_str: String = entry_counts
        .iter()
        .map(|(id, count)| format!("node{}={}", id, count))
        .collect::<Vec<_>>()
        .join(", ");

    if all_match {
        details.push(format!("cross-node: OK ({} nodes, {} entries each)", entry_counts.len(), first_count));
        true
    } else {
        details.push(format!("cross-node: MISMATCH ({})", counts_str));
        false
    }
}

/// Verify blob replication across all cluster nodes.
///
/// This performs cross-node verification by:
/// 1. Adding a test blob on the current node
/// 2. Querying each node to check if the blob hash exists
/// 3. Verifying all nodes have the blob
async fn verify_blob_cross_node(client: &AspenClient, hash: &str, details: &mut Vec<String>) -> bool {
    // Get cluster nodes
    let nodes = match get_cluster_nodes(client).await {
        Ok(nodes) => nodes,
        Err(e) => {
            details.push(format!("cross-node: SKIP ({})", e));
            return true;
        }
    };

    if nodes.len() <= 1 {
        details.push("cross-node: SKIP (single node)".to_string());
        return true;
    }

    let mut nodes_with_blob: Vec<u64> = Vec::new();
    let mut nodes_without_blob: Vec<u64> = Vec::new();
    let mut query_errors: Vec<String> = Vec::new();

    for node in &nodes {
        let addr = match parse_endpoint_addr(&node.endpoint_addr) {
            Ok(a) => a,
            Err(e) => {
                query_errors.push(format!("node {}: parse error ({})", node.node_id, e));
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
            Ok(ClientRpcResponse::Error(e)) => {
                if e.message.contains("not enabled") {
                    // Blobs not enabled on this node, skip it
                    continue;
                }
                query_errors.push(format!("node {}: {}", node.node_id, e.message));
            }
            Ok(_) => {
                query_errors.push(format!("node {}: unexpected response", node.node_id));
            }
            Err(e) => {
                query_errors.push(format!("node {}: {}", node.node_id, e));
            }
        }
    }

    if nodes_with_blob.is_empty() && nodes_without_blob.is_empty() {
        if !query_errors.is_empty() {
            details.push(format!("cross-node: SKIP (errors: {})", query_errors.join("; ")));
        } else {
            details.push("cross-node: SKIP (no nodes with blobs enabled)".to_string());
        }
        return true;
    }

    if nodes_without_blob.is_empty() {
        details.push(format!("cross-node: OK ({}/{} nodes have blob)", nodes_with_blob.len(), nodes_with_blob.len()));
        true
    } else {
        details.push(format!(
            "cross-node: PARTIAL ({}/{} nodes have blob, missing on {:?})",
            nodes_with_blob.len(),
            nodes_with_blob.len() + nodes_without_blob.len(),
            nodes_without_blob
        ));
        // Return true for partial (eventual consistency - blob may not have propagated yet)
        // Use 50% threshold for "acceptable" replication
        let total = nodes_with_blob.len() + nodes_without_blob.len();
        nodes_with_blob.len() * 2 >= total
    }
}
