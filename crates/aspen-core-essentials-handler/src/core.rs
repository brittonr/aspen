//! Core request handler for basic operations.
//!
//! Handles: Ping, GetHealth, GetRaftMetrics, GetNodeInfo, GetLeader, GetMetrics,
//! CheckpointWal, ListVaults, GetVaultKeys, Trace*, Metric*, Alert*.

use std::collections::HashMap;

use aspen_client_api::AlertComparison;
use aspen_client_api::AlertEvaluateResultResponse;
use aspen_client_api::AlertGetResultResponse;
use aspen_client_api::AlertHistoryEntry;
use aspen_client_api::AlertListResultResponse;
use aspen_client_api::AlertRuleResultResponse;
use aspen_client_api::AlertRuleWire;
use aspen_client_api::AlertRuleWithState;
use aspen_client_api::AlertStateWire;
use aspen_client_api::AlertStatus;
use aspen_client_api::CheckpointWalResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HealthResponse;
use aspen_client_api::IngestSpan;
use aspen_client_api::MetricDataPoint;
use aspen_client_api::MetricIngestResultResponse;
use aspen_client_api::MetricListResultResponse;
use aspen_client_api::MetricMetadata;
use aspen_client_api::MetricQueryResultResponse;
use aspen_client_api::MetricTypeWire;
use aspen_client_api::MetricsResponse;
use aspen_client_api::NodeInfoResponse;
use aspen_client_api::RaftMetricsResponse;
use aspen_client_api::ReplicationProgress;
use aspen_client_api::SpanStatusWire;
use aspen_client_api::TraceGetResultResponse;
use aspen_client_api::TraceIngestResultResponse;
use aspen_client_api::TraceListResultResponse;
use aspen_client_api::TraceSearchResultResponse;
use aspen_client_api::TraceSummary;
use aspen_client_api::VaultKeysResponse;
use aspen_client_api::VaultListResponse;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_core::CLIENT_RPC_REQUEST_COUNTER;
use aspen_core::kv::DeleteRequest;
use aspen_core::kv::ReadConsistency;
use aspen_core::kv::ReadRequest;
use aspen_core::kv::ScanRequest;
use aspen_core::kv::WriteRequest;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Handler for core operations (health, metrics, node info).
pub struct CoreHandler;

#[async_trait::async_trait]
impl RequestHandler for CoreHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::Ping
                | ClientRpcRequest::GetHealth
                | ClientRpcRequest::GetRaftMetrics
                | ClientRpcRequest::GetNodeInfo
                | ClientRpcRequest::GetLeader
                | ClientRpcRequest::GetMetrics
                | ClientRpcRequest::CheckpointWal
                | ClientRpcRequest::ListVaults
                | ClientRpcRequest::GetVaultKeys { .. }
                | ClientRpcRequest::TraceIngest { .. }
                | ClientRpcRequest::TraceList { .. }
                | ClientRpcRequest::TraceGet { .. }
                | ClientRpcRequest::TraceSearch { .. }
                | ClientRpcRequest::MetricIngest { .. }
                | ClientRpcRequest::MetricList { .. }
                | ClientRpcRequest::MetricQuery { .. }
                | ClientRpcRequest::AlertCreate { .. }
                | ClientRpcRequest::AlertDelete { .. }
                | ClientRpcRequest::AlertList
                | ClientRpcRequest::AlertGet { .. }
                | ClientRpcRequest::AlertEvaluate { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::Ping => Ok(ClientRpcResponse::Pong),
            ClientRpcRequest::GetHealth => handle_get_health(ctx).await,
            ClientRpcRequest::GetRaftMetrics => handle_get_raft_metrics(ctx).await,
            ClientRpcRequest::GetLeader => handle_get_leader(ctx).await,
            ClientRpcRequest::GetNodeInfo => handle_get_node_info(ctx).await,
            ClientRpcRequest::GetMetrics => handle_get_metrics(ctx).await,
            ClientRpcRequest::CheckpointWal => handle_checkpoint_wal(),
            ClientRpcRequest::ListVaults => handle_list_vaults(),
            ClientRpcRequest::GetVaultKeys { vault_name } => handle_get_vault_keys(vault_name),
            ClientRpcRequest::TraceIngest { spans } => handle_trace_ingest(ctx, spans).await,
            ClientRpcRequest::TraceList {
                start_time_us,
                end_time_us,
                limit,
                continuation_token,
            } => handle_trace_list(ctx, start_time_us, end_time_us, limit, continuation_token).await,
            ClientRpcRequest::TraceGet { trace_id } => handle_trace_get(ctx, trace_id).await,
            ClientRpcRequest::TraceSearch {
                operation,
                min_duration_us,
                max_duration_us,
                status,
                limit,
            } => handle_trace_search(ctx, operation, min_duration_us, max_duration_us, status, limit).await,
            ClientRpcRequest::MetricIngest {
                data_points,
                ttl_seconds,
            } => handle_metric_ingest(ctx, data_points, ttl_seconds).await,
            ClientRpcRequest::MetricList { prefix, limit } => handle_metric_list(ctx, prefix, limit).await,
            ClientRpcRequest::MetricQuery {
                name,
                start_time_us,
                end_time_us,
                label_filters,
                aggregation,
                step_us,
                limit,
            } => {
                handle_metric_query(ctx, name, start_time_us, end_time_us, label_filters, aggregation, step_us, limit)
                    .await
            }
            ClientRpcRequest::AlertCreate { rule } => handle_alert_create(ctx, rule).await,
            ClientRpcRequest::AlertDelete { name } => handle_alert_delete(ctx, name).await,
            ClientRpcRequest::AlertList => handle_alert_list(ctx).await,
            ClientRpcRequest::AlertGet { name } => handle_alert_get(ctx, name).await,
            ClientRpcRequest::AlertEvaluate { name, now_us } => handle_alert_evaluate(ctx, name, now_us).await,
            _ => Err(anyhow::anyhow!("request not handled by CoreHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CoreHandler"
    }
}

// =============================================================================
// Individual handler functions (Tiger Style: functions under 70 lines)
// =============================================================================

async fn handle_get_health(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let uptime_seconds = ctx.start_time.elapsed().as_secs();

    // Get metrics to determine health status and membership count
    let (status, is_initialized, membership_node_count) = match ctx.controller.get_metrics().await {
        Ok(metrics) => {
            let status = if metrics.current_leader.is_some() {
                "healthy"
            } else {
                "degraded"
            };
            // Node is initialized if it has non-empty membership (voters + learners)
            let node_count = metrics.voters.len() + metrics.learners.len();
            let is_init = node_count > 0;
            (status, is_init, Some(node_count as u32))
        }
        Err(_) => ("unhealthy", false, None),
    };

    Ok(ClientRpcResponse::Health(HealthResponse {
        status: status.to_string(),
        node_id: ctx.node_id,
        raft_node_id: Some(ctx.node_id),
        uptime_seconds,
        is_initialized,
        membership_node_count,
    }))
}

async fn handle_get_raft_metrics(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let metrics = ctx
        .controller
        .get_metrics()
        .await
        .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

    // Extract replication progress if this node is leader
    // ClusterMetrics already has unwrapped types (u64 instead of NodeId/LogId)
    let replication = metrics.replication.as_ref().map(|repl_map| {
        repl_map
            .iter()
            .map(|(node_id, matched_index)| ReplicationProgress {
                node_id: *node_id,
                matched_index: *matched_index,
            })
            .collect()
    });

    Ok(ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
        node_id: ctx.node_id,
        state: format!("{:?}", metrics.state),
        current_leader: metrics.current_leader,
        current_term: metrics.current_term,
        last_log_index: metrics.last_log_index,
        last_applied_index: metrics.last_applied_index,
        snapshot_index: metrics.snapshot_index,
        replication,
    }))
}

async fn handle_get_leader(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let leader = ctx.controller.get_leader().await.map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
    Ok(ClientRpcResponse::Leader(leader))
}

async fn handle_get_node_info(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Get peer ID and addresses from endpoint provider
    let peer_id = ctx.endpoint_manager.peer_id().await;
    let addresses = ctx.endpoint_manager.addresses().await;
    Ok(ClientRpcResponse::NodeInfo(NodeInfoResponse {
        node_id: ctx.node_id,
        endpoint_addr: format!("{}:{:?}", peer_id, addresses),
    }))
}

async fn handle_get_metrics(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Return Prometheus-format metrics from Raft
    let metrics_text = match ctx.controller.get_metrics().await {
        Ok(metrics) => {
            let state_value = metrics.state.as_u8();
            let is_leader: u8 = u8::from(metrics.state.is_leader());
            let last_applied = metrics.last_applied_index.unwrap_or(0);
            let snapshot_index = metrics.snapshot_index.unwrap_or(0);

            // Get cluster-wide request counter
            let request_counter = {
                let counter =
                    AtomicCounter::new(ctx.kv_store.clone(), CLIENT_RPC_REQUEST_COUNTER, CounterConfig::default());
                counter.get().await.unwrap_or(0)
            };

            format!(
                "# HELP aspen_raft_term Current Raft term\n\
                 # TYPE aspen_raft_term gauge\n\
                 aspen_raft_term{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_raft_state Raft state (0=Learner, 1=Follower, 2=Candidate, 3=Leader, 4=Shutdown)\n\
                 # TYPE aspen_raft_state gauge\n\
                 aspen_raft_state{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_raft_is_leader Whether this node is the leader\n\
                 # TYPE aspen_raft_is_leader gauge\n\
                 aspen_raft_is_leader{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_raft_last_log_index Last log index\n\
                 # TYPE aspen_raft_last_log_index gauge\n\
                 aspen_raft_last_log_index{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_raft_last_applied_index Last applied log index\n\
                 # TYPE aspen_raft_last_applied_index gauge\n\
                 aspen_raft_last_applied_index{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_raft_snapshot_index Snapshot index\n\
                 # TYPE aspen_raft_snapshot_index gauge\n\
                 aspen_raft_snapshot_index{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_node_uptime_seconds Node uptime in seconds\n\
                 # TYPE aspen_node_uptime_seconds counter\n\
                 aspen_node_uptime_seconds{{node_id=\"{}\"}} {}\n\
                 # HELP aspen_client_api_requests_total Total client RPC requests processed cluster-wide\n\
                 # TYPE aspen_client_api_requests_total counter\n\
                 aspen_client_api_requests_total{{node_id=\"{}\"}} {}\n",
                ctx.node_id,
                metrics.current_term,
                ctx.node_id,
                state_value,
                ctx.node_id,
                is_leader,
                ctx.node_id,
                metrics.last_log_index.unwrap_or(0),
                ctx.node_id,
                last_applied,
                ctx.node_id,
                snapshot_index,
                ctx.node_id,
                ctx.start_time.elapsed().as_secs(),
                ctx.node_id,
                request_counter
            )
        }
        Err(_) => String::new(),
    };

    Ok(ClientRpcResponse::Metrics(MetricsResponse {
        prometheus_text: metrics_text,
    }))
}

fn handle_checkpoint_wal() -> anyhow::Result<ClientRpcResponse> {
    // WAL checkpoint requires direct access to the SQLite state machine,
    // which is not exposed through the ClusterController/KeyValueStore traits.
    Ok(ClientRpcResponse::CheckpointWalResult(CheckpointWalResultResponse {
        is_success: false,
        pages_checkpointed: None,
        wal_size_before_bytes: None,
        wal_size_after_bytes: None,
        error: Some(
            "WAL checkpoint requires state machine access - use trigger_snapshot for log compaction".to_string(),
        ),
    }))
}

fn handle_list_vaults() -> anyhow::Result<ClientRpcResponse> {
    // Deprecated: Vault-specific operations removed in favor of flat keyspace.
    Ok(ClientRpcResponse::VaultList(VaultListResponse {
        vaults: vec![],
        error: Some("ListVaults is deprecated. Use ScanKeys with a prefix instead.".to_string()),
    }))
}

fn handle_get_vault_keys(vault_name: String) -> anyhow::Result<ClientRpcResponse> {
    // Deprecated: Vault-specific operations removed in favor of flat keyspace.
    Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
        vault: vault_name,
        keys: vec![],
        error: Some("GetVaultKeys is deprecated. Use ScanKeys with a prefix instead.".to_string()),
    }))
}

async fn handle_trace_ingest(ctx: &ClientProtocolContext, spans: Vec<IngestSpan>) -> anyhow::Result<ClientRpcResponse> {
    let max_batch = aspen_constants::MAX_TRACE_BATCH_SIZE as usize;
    let total_count = spans.len();
    let accepted_count = total_count.min(max_batch) as u32;
    let dropped_count = total_count.saturating_sub(max_batch) as u32;

    // Store each span as a KV entry under _sys:traces:{trace_id}:{span_id}
    for span in spans.into_iter().take(max_batch) {
        let key = format!("_sys:traces:{}:{}", span.trace_id, span.span_id);
        let value_json = serde_json::to_string(&span).map_err(|e| anyhow::anyhow!("serialize span: {}", e))?;
        let write_req = aspen_core::kv::WriteRequest::set(key, value_json);
        if let Err(e) = ctx.kv_store.write(write_req).await {
            tracing::warn!(error = %e, "failed to store trace span");
            return Ok(ClientRpcResponse::TraceIngestResult(TraceIngestResultResponse {
                is_success: false,
                accepted_count: 0,
                dropped_count: total_count as u32,
                error: Some(format!("storage error: {}", e)),
            }));
        }
    }

    Ok(ClientRpcResponse::TraceIngestResult(TraceIngestResultResponse {
        is_success: true,
        accepted_count,
        dropped_count,
        error: None,
    }))
}

// =============================================================================
// Trace query handlers
// =============================================================================

/// Scan KV for trace spans and deserialize, skipping unparseable entries.
async fn scan_trace_spans(ctx: &ClientProtocolContext, prefix: &str) -> Vec<IngestSpan> {
    let scan_req = ScanRequest {
        prefix: prefix.to_string(),
        limit_results: Some(aspen_constants::MAX_SCAN_RESULTS),
        continuation_token: None,
    };
    let scan_result = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(error = %e, prefix, "trace scan failed");
            return Vec::new();
        }
    };
    let mut spans = Vec::with_capacity(scan_result.entries.len());
    for entry in &scan_result.entries {
        match serde_json::from_str::<IngestSpan>(&entry.value) {
            Ok(span) => spans.push(span),
            Err(e) => tracing::warn!(key = %entry.key, error = %e, "skip bad trace span"),
        }
    }
    spans
}

/// Check if a span matches a status filter string.
fn matches_status(span: &IngestSpan, filter: &str) -> bool {
    match filter {
        "ok" => span.status == SpanStatusWire::Ok,
        "error" => matches!(span.status, SpanStatusWire::Error(_)),
        "unset" => span.status == SpanStatusWire::Unset,
        _ => true,
    }
}

/// Build a TraceSummary from a group of spans sharing a trace_id.
fn build_trace_summary(trace_id: String, spans: &[IngestSpan]) -> TraceSummary {
    let span_count = spans.len() as u32;
    let root_operation = spans.iter().find(|s| s.parent_id == "0000000000000000").map(|s| s.operation.clone());
    let start_time_us = spans.iter().map(|s| s.start_time_us).min().unwrap_or(0);
    let end_time_us = spans.iter().map(|s| s.start_time_us.saturating_add(s.duration_us)).max().unwrap_or(0);
    let total_duration_us = end_time_us.saturating_sub(start_time_us);
    let has_error = spans.iter().any(|s| matches!(s.status, SpanStatusWire::Error(_)));
    TraceSummary {
        trace_id,
        span_count,
        root_operation,
        start_time_us,
        total_duration_us,
        has_error,
    }
}

async fn handle_trace_list(
    ctx: &ClientProtocolContext,
    start_time_us: Option<u64>,
    end_time_us: Option<u64>,
    limit: Option<u32>,
    _continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let effective_limit = limit
        .unwrap_or(aspen_constants::DEFAULT_TRACE_QUERY_LIMIT)
        .min(aspen_constants::MAX_TRACE_QUERY_RESULTS) as usize;

    let spans = scan_trace_spans(ctx, "_sys:traces:").await;

    // Group by trace_id
    let mut groups: HashMap<String, Vec<&IngestSpan>> = HashMap::new();
    for span in &spans {
        groups.entry(span.trace_id.clone()).or_default().push(span);
    }

    // Build summaries with optional time-range filter
    let mut summaries: Vec<TraceSummary> = groups
        .into_iter()
        .filter_map(|(trace_id, group)| {
            let owned: Vec<IngestSpan> = group.into_iter().cloned().collect();
            let summary = build_trace_summary(trace_id, &owned);
            // Time-range filter: skip if entire trace is outside the range
            if let Some(start) = start_time_us {
                let trace_end = summary.start_time_us.saturating_add(summary.total_duration_us);
                if trace_end < start {
                    return None;
                }
            }
            if let Some(end) = end_time_us
                && summary.start_time_us >= end
            {
                return None;
            }
            Some(summary)
        })
        .collect();

    // Sort newest first
    summaries.sort_by_key(|s| std::cmp::Reverse(s.start_time_us));

    let is_truncated = summaries.len() > effective_limit;
    summaries.truncate(effective_limit);
    let count = summaries.len() as u32;

    Ok(ClientRpcResponse::TraceListResult(TraceListResultResponse {
        traces: summaries,
        count,
        is_truncated,
        error: None,
    }))
}

async fn handle_trace_get(ctx: &ClientProtocolContext, trace_id: String) -> anyhow::Result<ClientRpcResponse> {
    let prefix = format!("_sys:traces:{}:", trace_id);
    let spans = scan_trace_spans(ctx, &prefix).await;
    let span_count = spans.len() as u32;

    Ok(ClientRpcResponse::TraceGetResult(TraceGetResultResponse {
        trace_id,
        spans,
        span_count,
        error: None,
    }))
}

async fn handle_trace_search(
    ctx: &ClientProtocolContext,
    operation: Option<String>,
    min_duration_us: Option<u64>,
    max_duration_us: Option<u64>,
    status: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let effective_limit = limit
        .unwrap_or(aspen_constants::DEFAULT_TRACE_QUERY_LIMIT)
        .min(aspen_constants::MAX_TRACE_QUERY_RESULTS) as usize;

    let all_spans = scan_trace_spans(ctx, "_sys:traces:").await;
    let op_lower = operation.as_deref().map(str::to_lowercase);

    let mut matched: Vec<IngestSpan> = Vec::new();
    for span in all_spans {
        if let Some(ref op) = op_lower
            && !span.operation.to_lowercase().contains(op.as_str())
        {
            continue;
        }
        if let Some(min) = min_duration_us
            && span.duration_us < min
        {
            continue;
        }
        if let Some(max) = max_duration_us
            && span.duration_us > max
        {
            continue;
        }
        if let Some(ref s) = status
            && !matches_status(&span, s)
        {
            continue;
        }
        matched.push(span);
        if matched.len() >= effective_limit {
            break;
        }
    }

    let is_truncated = matched.len() >= effective_limit;
    let count = matched.len() as u32;

    Ok(ClientRpcResponse::TraceSearchResult(TraceSearchResultResponse {
        spans: matched,
        count,
        is_truncated,
        error: None,
    }))
}

/// Helper: read a key from the KV store with linearizable consistency.
/// Returns `Some(value_string)` if found, `None` if not found or error.
async fn kv_read_value(ctx: &ClientProtocolContext, key: &str) -> Option<String> {
    let req = ReadRequest {
        key: key.to_string(),
        consistency: ReadConsistency::Linearizable,
    };
    match ctx.kv_store.read(req).await {
        Ok(result) => result.kv.map(|kv| kv.value),
        Err(_) => None,
    }
}

// =============================================================================
// Metric handlers
// =============================================================================

async fn handle_metric_ingest(
    ctx: &ClientProtocolContext,
    data_points: Vec<MetricDataPoint>,
    ttl_seconds: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let max_batch = aspen_constants::MAX_METRIC_BATCH_SIZE as usize;
    let total_count = data_points.len();
    let accepted_count = total_count.min(max_batch) as u32;
    let dropped_count = total_count.saturating_sub(max_batch) as u32;

    let ttl = ttl_seconds
        .unwrap_or(aspen_constants::METRIC_DEFAULT_TTL_SECONDS)
        .min(aspen_constants::METRIC_MAX_TTL_SECONDS);

    // Track unique metric names for metadata upsert
    let mut seen_names: HashMap<String, &MetricDataPoint> = HashMap::new();

    for dp in data_points.iter().take(max_batch) {
        let key = format!("_sys:metrics:{}:{:020}", dp.name, dp.timestamp_us);
        let value_json = serde_json::to_string(&dp).map_err(|e| anyhow::anyhow!("serialize metric: {}", e))?;
        let write_req = WriteRequest::set_with_ttl(key, value_json, ttl);
        if let Err(e) = ctx.kv_store.write(write_req).await {
            tracing::warn!(error = %e, "failed to store metric data point");
            return Ok(ClientRpcResponse::MetricIngestResult(MetricIngestResultResponse {
                is_success: false,
                accepted_count: 0,
                dropped_count: total_count as u32,
                error: Some(format!("storage error: {}", e)),
            }));
        }
        seen_names.entry(dp.name.clone()).or_insert(dp);
    }

    // Upsert metadata for each unique metric name
    for (name, sample) in &seen_names {
        let label_keys: Vec<String> = sample.labels.iter().map(|(k, _)| k.clone()).collect();
        let meta = MetricMetadata {
            name: name.clone(),
            metric_type: sample.metric_type.clone(),
            description: String::new(),
            unit: String::new(),
            label_keys,
            last_updated_us: sample.timestamp_us,
        };
        let meta_key = format!("_sys:metrics_meta:{}", name);
        let meta_json = serde_json::to_string(&meta).map_err(|e| anyhow::anyhow!("serialize metadata: {}", e))?;
        let write_req = WriteRequest::set(meta_key, meta_json);
        if let Err(e) = ctx.kv_store.write(write_req).await {
            tracing::warn!(error = %e, metric_name = %name, "failed to upsert metric metadata");
        }
    }

    Ok(ClientRpcResponse::MetricIngestResult(MetricIngestResultResponse {
        is_success: true,
        accepted_count,
        dropped_count,
        error: None,
    }))
}

async fn handle_metric_list(
    ctx: &ClientProtocolContext,
    prefix: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let scan_prefix = format!("_sys:metrics_meta:{}", prefix.as_deref().unwrap_or(""));
    let effective_limit = limit
        .unwrap_or(aspen_constants::MAX_METRIC_LIST_RESULTS)
        .min(aspen_constants::MAX_METRIC_LIST_RESULTS);

    let scan_req = ScanRequest {
        prefix: scan_prefix,
        limit_results: Some(effective_limit),
        continuation_token: None,
    };
    let scan_result = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(error = %e, "metric list scan failed");
            return Ok(ClientRpcResponse::MetricListResult(MetricListResultResponse {
                metrics: vec![],
                count: 0,
                error: Some(format!("scan error: {}", e)),
            }));
        }
    };

    let mut metrics = Vec::with_capacity(scan_result.entries.len());
    for entry in &scan_result.entries {
        match serde_json::from_str::<MetricMetadata>(&entry.value) {
            Ok(meta) => metrics.push(meta),
            Err(e) => tracing::warn!(key = %entry.key, error = %e, "skip bad metric metadata"),
        }
    }

    let count = metrics.len() as u32;
    Ok(ClientRpcResponse::MetricListResult(MetricListResultResponse {
        metrics,
        count,
        error: None,
    }))
}

#[allow(clippy::too_many_arguments)]
async fn handle_metric_query(
    ctx: &ClientProtocolContext,
    name: String,
    start_time_us: Option<u64>,
    end_time_us: Option<u64>,
    label_filters: Vec<(String, String)>,
    aggregation: Option<String>,
    step_us: Option<u64>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let scan_prefix = format!("_sys:metrics:{}:", name);
    let scan_req = ScanRequest {
        prefix: scan_prefix,
        limit_results: Some(aspen_constants::MAX_METRIC_QUERY_RESULTS),
        continuation_token: None,
    };
    let scan_result = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(error = %e, metric_name = %name, "metric query scan failed");
            return Ok(ClientRpcResponse::MetricQueryResult(MetricQueryResultResponse {
                name,
                data_points: vec![],
                count: 0,
                is_truncated: false,
                error: Some(format!("scan error: {}", e)),
            }));
        }
    };

    // Deserialize and filter
    let mut points: Vec<MetricDataPoint> = Vec::new();
    for entry in &scan_result.entries {
        let dp = match serde_json::from_str::<MetricDataPoint>(&entry.value) {
            Ok(dp) => dp,
            Err(e) => {
                tracing::warn!(key = %entry.key, error = %e, "skip bad metric data point");
                continue;
            }
        };
        if let Some(start) = start_time_us
            && dp.timestamp_us < start
        {
            continue;
        }
        if let Some(end) = end_time_us
            && dp.timestamp_us >= end
        {
            continue;
        }
        if !matches_label_filters(&dp.labels, &label_filters) {
            continue;
        }
        points.push(dp);
    }

    // Apply aggregation
    let agg = aggregation.as_deref().unwrap_or("none");
    let result_points = if agg == "none" {
        points
    } else {
        aggregate_metric_points(&points, agg, step_us, &name)
    };

    let effective_limit = limit
        .unwrap_or(aspen_constants::DEFAULT_METRIC_QUERY_LIMIT)
        .min(aspen_constants::MAX_METRIC_QUERY_RESULTS) as usize;
    let is_truncated = result_points.len() > effective_limit;
    let truncated: Vec<MetricDataPoint> = result_points.into_iter().take(effective_limit).collect();
    let count = truncated.len() as u32;

    Ok(ClientRpcResponse::MetricQueryResult(MetricQueryResultResponse {
        name,
        data_points: truncated,
        count,
        is_truncated,
        error: None,
    }))
}

/// Check if a data point's labels match all required filters.
fn matches_label_filters(dp_labels: &[(String, String)], filters: &[(String, String)]) -> bool {
    filters.iter().all(|(fk, fv)| dp_labels.iter().any(|(dk, dv)| dk == fk && dv == fv))
}

/// Aggregate metric data points by time bucket.
fn aggregate_metric_points(
    points: &[MetricDataPoint],
    aggregation: &str,
    step_us: Option<u64>,
    metric_name: &str,
) -> Vec<MetricDataPoint> {
    if points.is_empty() {
        return vec![];
    }

    match step_us {
        Some(step) if step > 0 => {
            // Bucket points by timestamp
            let mut buckets: HashMap<u64, Vec<f64>> = HashMap::new();
            for dp in points {
                let bucket_key = dp.timestamp_us / step * step;
                buckets.entry(bucket_key).or_default().push(dp.value);
            }
            let mut result: Vec<MetricDataPoint> = buckets
                .into_iter()
                .map(|(ts, values)| MetricDataPoint {
                    name: metric_name.to_string(),
                    metric_type: MetricTypeWire::Gauge,
                    timestamp_us: ts,
                    value: compute_aggregation(&values, aggregation),
                    labels: vec![],
                    histogram_buckets: None,
                    histogram_sum: None,
                    histogram_count: None,
                })
                .collect();
            result.sort_by_key(|dp| dp.timestamp_us);
            result
        }
        _ => {
            // Aggregate all points into a single result
            let values: Vec<f64> = points.iter().map(|dp| dp.value).collect();
            let ts = points.first().map(|dp| dp.timestamp_us).unwrap_or(0);
            vec![MetricDataPoint {
                name: metric_name.to_string(),
                metric_type: MetricTypeWire::Gauge,
                timestamp_us: ts,
                value: compute_aggregation(&values, aggregation),
                labels: vec![],
                histogram_buckets: None,
                histogram_sum: None,
                histogram_count: None,
            }]
        }
    }
}

/// Compute an aggregation function over a slice of values.
fn compute_aggregation(values: &[f64], aggregation: &str) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    match aggregation {
        "avg" => values.iter().sum::<f64>() / values.len() as f64,
        "sum" => values.iter().sum(),
        "min" => values.iter().cloned().fold(f64::INFINITY, f64::min),
        "max" => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "count" => values.len() as f64,
        "last" => values.last().copied().unwrap_or(0.0),
        _ => values.iter().sum::<f64>() / values.len() as f64, // default to avg
    }
}

// =============================================================================
// Alert handlers
// =============================================================================

async fn handle_alert_create(ctx: &ClientProtocolContext, rule: AlertRuleWire) -> anyhow::Result<ClientRpcResponse> {
    // Validate name length
    if rule.name.len() > aspen_constants::MAX_ALERT_RULE_NAME_SIZE as usize {
        return Ok(ClientRpcResponse::AlertCreateResult(AlertRuleResultResponse {
            is_success: false,
            rule_name: rule.name,
            error: Some(format!("rule name exceeds {} bytes", aspen_constants::MAX_ALERT_RULE_NAME_SIZE)),
        }));
    }

    // Check if this is an update (rule already exists)
    let rule_key = format!("_sys:alerts:rule:{}", rule.name);
    let is_update = kv_read_value(ctx, &rule_key).await.is_some();

    // Check rule count limit for new rules
    if !is_update {
        let scan_req = ScanRequest {
            prefix: "_sys:alerts:rule:".to_string(),
            limit_results: Some(aspen_constants::MAX_ALERT_RULES + 1),
            continuation_token: None,
        };
        if let Ok(result) = ctx.kv_store.scan(scan_req).await
            && result.entries.len() >= aspen_constants::MAX_ALERT_RULES as usize
        {
            return Ok(ClientRpcResponse::AlertCreateResult(AlertRuleResultResponse {
                is_success: false,
                rule_name: rule.name,
                error: Some(format!("alert rule limit reached ({})", aspen_constants::MAX_ALERT_RULES)),
            }));
        }
    }

    // Write rule
    let rule_json = serde_json::to_string(&rule).map_err(|e| anyhow::anyhow!("serialize rule: {}", e))?;
    let write_req = WriteRequest::set(rule_key, rule_json);
    if let Err(e) = ctx.kv_store.write(write_req).await {
        return Ok(ClientRpcResponse::AlertCreateResult(AlertRuleResultResponse {
            is_success: false,
            rule_name: rule.name,
            error: Some(format!("storage error: {}", e)),
        }));
    }

    // Initialize state if new rule
    if !is_update {
        let state = AlertStateWire {
            rule_name: rule.name.clone(),
            status: AlertStatus::Ok,
            last_value: None,
            last_evaluated_us: 0,
            condition_since_us: None,
            last_fired_us: None,
            last_resolved_us: None,
        };
        let state_key = format!("_sys:alerts:state:{}", rule.name);
        let state_json = serde_json::to_string(&state).map_err(|e| anyhow::anyhow!("serialize state: {}", e))?;
        let write_req = WriteRequest::set(state_key, state_json);
        if let Err(e) = ctx.kv_store.write(write_req).await {
            tracing::warn!(error = %e, rule = %rule.name, "failed to initialize alert state");
        }
    }

    Ok(ClientRpcResponse::AlertCreateResult(AlertRuleResultResponse {
        is_success: true,
        rule_name: rule.name,
        error: None,
    }))
}

async fn handle_alert_delete(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    // Delete rule
    let _ = ctx.kv_store.delete(DeleteRequest::new(format!("_sys:alerts:rule:{}", name))).await;

    // Delete state
    let _ = ctx.kv_store.delete(DeleteRequest::new(format!("_sys:alerts:state:{}", name))).await;

    // Delete history entries
    let history_prefix = format!("_sys:alerts:history:{}:", name);
    let scan_req = ScanRequest {
        prefix: history_prefix,
        limit_results: Some(aspen_constants::MAX_ALERT_HISTORY_PER_RULE),
        continuation_token: None,
    };
    if let Ok(result) = ctx.kv_store.scan(scan_req).await {
        for entry in &result.entries {
            let _ = ctx.kv_store.delete(DeleteRequest::new(entry.key.clone())).await;
        }
    }

    Ok(ClientRpcResponse::AlertDeleteResult(AlertRuleResultResponse {
        is_success: true,
        rule_name: name,
        error: None,
    }))
}

async fn handle_alert_list(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let scan_req = ScanRequest {
        prefix: "_sys:alerts:rule:".to_string(),
        limit_results: Some(aspen_constants::MAX_ALERT_RULES),
        continuation_token: None,
    };
    let scan_result = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result,
        Err(e) => {
            return Ok(ClientRpcResponse::AlertListResult(AlertListResultResponse {
                rules: vec![],
                count: 0,
                error: Some(format!("scan error: {}", e)),
            }));
        }
    };

    let mut rules = Vec::with_capacity(scan_result.entries.len());
    for entry in &scan_result.entries {
        let rule = match serde_json::from_str::<AlertRuleWire>(&entry.value) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(key = %entry.key, error = %e, "skip bad alert rule");
                continue;
            }
        };
        // Read state
        let state_key = format!("_sys:alerts:state:{}", rule.name);
        let state = kv_read_value(ctx, &state_key).await.and_then(|v| serde_json::from_str::<AlertStateWire>(&v).ok());
        rules.push(AlertRuleWithState { rule, state });
    }

    let count = rules.len() as u32;
    Ok(ClientRpcResponse::AlertListResult(AlertListResultResponse {
        rules,
        count,
        error: None,
    }))
}

async fn handle_alert_get(ctx: &ClientProtocolContext, name: String) -> anyhow::Result<ClientRpcResponse> {
    // Read rule
    let rule_key = format!("_sys:alerts:rule:{}", name);
    let rule = kv_read_value(ctx, &rule_key).await.and_then(|v| serde_json::from_str::<AlertRuleWire>(&v).ok());

    // Read state
    let state_key = format!("_sys:alerts:state:{}", name);
    let state = kv_read_value(ctx, &state_key).await.and_then(|v| serde_json::from_str::<AlertStateWire>(&v).ok());

    // Read history
    let history_prefix = format!("_sys:alerts:history:{}:", name);
    let scan_req = ScanRequest {
        prefix: history_prefix,
        limit_results: Some(aspen_constants::MAX_ALERT_HISTORY_PER_RULE),
        continuation_token: None,
    };
    let mut history = Vec::new();
    if let Ok(result) = ctx.kv_store.scan(scan_req).await {
        for entry in &result.entries {
            if let Ok(h) = serde_json::from_str::<AlertHistoryEntry>(&entry.value) {
                history.push(h);
            }
        }
    }

    Ok(ClientRpcResponse::AlertGetResult(AlertGetResultResponse {
        rule,
        state,
        history,
        error: None,
    }))
}

async fn handle_alert_evaluate(
    ctx: &ClientProtocolContext,
    name: String,
    now_us: u64,
) -> anyhow::Result<ClientRpcResponse> {
    // Read rule
    let rule_key = format!("_sys:alerts:rule:{}", name);
    let rule_value = kv_read_value(ctx, &rule_key).await;
    let rule = match rule_value {
        Some(v) => match serde_json::from_str::<AlertRuleWire>(&v) {
            Ok(r) => r,
            Err(e) => {
                return Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
                    rule_name: name,
                    status: AlertStatus::Ok,
                    computed_value: None,
                    threshold: 0.0,
                    did_transition: false,
                    previous_status: None,
                    error: Some(format!("bad rule data: {}", e)),
                }));
            }
        },
        None => {
            return Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
                rule_name: name,
                status: AlertStatus::Ok,
                computed_value: None,
                threshold: 0.0,
                did_transition: false,
                previous_status: None,
                error: Some("rule not found".to_string()),
            }));
        }
    };

    // If rule is disabled, return current state
    if !rule.is_enabled {
        let state_key = format!("_sys:alerts:state:{}", name);
        let status = kv_read_value(ctx, &state_key)
            .await
            .and_then(|v| serde_json::from_str::<AlertStateWire>(&v).ok())
            .map(|s| s.status)
            .unwrap_or(AlertStatus::Ok);
        return Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
            rule_name: name,
            status,
            computed_value: None,
            threshold: rule.threshold,
            did_transition: false,
            previous_status: None,
            error: None,
        }));
    }

    // Scan metric data for the evaluation window
    let start_us = now_us.saturating_sub(rule.window_duration_us);
    let scan_prefix = format!("_sys:metrics:{}:", rule.metric_name);
    let scan_req = ScanRequest {
        prefix: scan_prefix,
        limit_results: Some(aspen_constants::MAX_METRIC_QUERY_RESULTS),
        continuation_token: None,
    };
    let metric_values: Vec<f64> = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result
            .entries
            .iter()
            .filter_map(|entry| serde_json::from_str::<MetricDataPoint>(&entry.value).ok())
            .filter(|dp| dp.timestamp_us >= start_us && dp.timestamp_us <= now_us)
            .filter(|dp| matches_label_filters(&dp.labels, &rule.label_filters))
            .map(|dp| dp.value)
            .collect(),
        Err(e) => {
            return Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
                rule_name: name,
                status: AlertStatus::Ok,
                computed_value: None,
                threshold: rule.threshold,
                did_transition: false,
                previous_status: None,
                error: Some(format!("metric scan error: {}", e)),
            }));
        }
    };

    // If no data, don't transition
    if metric_values.is_empty() {
        return Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
            rule_name: name,
            status: AlertStatus::Ok,
            computed_value: None,
            threshold: rule.threshold,
            did_transition: false,
            previous_status: None,
            error: None,
        }));
    }

    // Compute aggregated value
    let computed_value = compute_aggregation(&metric_values, &rule.aggregation);
    let is_breached = evaluate_threshold(computed_value, &rule.comparison, rule.threshold);

    // Read current state
    let state_key = format!("_sys:alerts:state:{}", name);
    let default_state = || AlertStateWire {
        rule_name: name.clone(),
        status: AlertStatus::Ok,
        last_value: None,
        last_evaluated_us: 0,
        condition_since_us: None,
        last_fired_us: None,
        last_resolved_us: None,
    };
    let mut current_state = kv_read_value(ctx, &state_key)
        .await
        .and_then(|v| serde_json::from_str::<AlertStateWire>(&v).ok())
        .unwrap_or_else(default_state);

    let previous_status = current_state.status.clone();
    let new_status = compute_alert_transition(&current_state.status, is_breached, &current_state, &rule, now_us);
    let did_transition = new_status != previous_status;

    // Update state
    if did_transition {
        match &new_status {
            AlertStatus::Pending => {
                current_state.condition_since_us = Some(now_us);
            }
            AlertStatus::Firing => {
                current_state.last_fired_us = Some(now_us);
            }
            AlertStatus::Ok => {
                if previous_status == AlertStatus::Firing {
                    current_state.last_resolved_us = Some(now_us);
                }
                current_state.condition_since_us = None;
            }
        }
    }
    current_state.status = new_status.clone();
    current_state.last_value = Some(computed_value);
    current_state.last_evaluated_us = now_us;

    // Write updated state
    let state_json =
        serde_json::to_string(&current_state).map_err(|e| anyhow::anyhow!("serialize alert state: {}", e))?;
    let write_req = WriteRequest::set(state_key, state_json);
    if let Err(e) = ctx.kv_store.write(write_req).await {
        tracing::warn!(error = %e, rule = %name, "failed to write alert state");
    }

    // Record history if transition occurred
    if did_transition {
        let history_entry = AlertHistoryEntry {
            rule_name: name.clone(),
            from_status: previous_status.clone(),
            to_status: new_status.clone(),
            value: computed_value,
            threshold: rule.threshold,
            timestamp_us: now_us,
        };
        let history_key = format!("_sys:alerts:history:{}:{:020}", name, now_us);
        let history_json =
            serde_json::to_string(&history_entry).map_err(|e| anyhow::anyhow!("serialize history: {}", e))?;
        let write_req =
            WriteRequest::set_with_ttl(history_key, history_json, aspen_constants::ALERT_HISTORY_TTL_SECONDS);
        if let Err(e) = ctx.kv_store.write(write_req).await {
            tracing::warn!(error = %e, rule = %name, "failed to write alert history");
        }
    }

    Ok(ClientRpcResponse::AlertEvaluateResult(AlertEvaluateResultResponse {
        rule_name: name,
        status: new_status,
        computed_value: Some(computed_value),
        threshold: rule.threshold,
        did_transition,
        previous_status: if did_transition { Some(previous_status) } else { None },
        error: None,
    }))
}

/// Evaluate whether a value breaches a threshold given a comparison operator.
fn evaluate_threshold(value: f64, comparison: &AlertComparison, threshold: f64) -> bool {
    match comparison {
        AlertComparison::GreaterThan => value > threshold,
        AlertComparison::GreaterThanOrEqual => value >= threshold,
        AlertComparison::LessThan => value < threshold,
        AlertComparison::LessThanOrEqual => value <= threshold,
        AlertComparison::Equal => (value - threshold).abs() < f64::EPSILON,
        AlertComparison::NotEqual => (value - threshold).abs() >= f64::EPSILON,
    }
}

/// Compute alert state transition based on current status and threshold breach.
///
/// State machine:
/// - Ok + breached → Pending (or Firing if for_duration_us == 0)
/// - Pending + breached + duration elapsed → Firing
/// - Pending + breached + duration not elapsed → Pending (no change)
/// - Pending + not breached → Ok
/// - Firing + not breached → Ok
/// - Firing + breached → Firing (no change)
fn compute_alert_transition(
    current: &AlertStatus,
    is_breached: bool,
    state: &AlertStateWire,
    rule: &AlertRuleWire,
    now_us: u64,
) -> AlertStatus {
    match (current, is_breached) {
        (AlertStatus::Ok, true) => {
            if rule.for_duration_us == 0 {
                AlertStatus::Firing
            } else {
                AlertStatus::Pending
            }
        }
        (AlertStatus::Ok, false) => AlertStatus::Ok,
        (AlertStatus::Pending, true) => {
            if let Some(since) = state.condition_since_us {
                if now_us.saturating_sub(since) >= rule.for_duration_us {
                    AlertStatus::Firing
                } else {
                    AlertStatus::Pending
                }
            } else {
                AlertStatus::Pending
            }
        }
        (AlertStatus::Pending, false) => AlertStatus::Ok,
        (AlertStatus::Firing, true) => AlertStatus::Firing,
        (AlertStatus::Firing, false) => AlertStatus::Ok,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_client_api::AlertComparison;
    use aspen_client_api::AlertRuleWire;
    use aspen_client_api::AlertSeverity;
    use aspen_client_api::AlertStatus;
    use aspen_client_api::SpanEventWire;
    use aspen_rpc_core::test_support::MockEndpointProvider;
    use aspen_rpc_core::test_support::TestContextBuilder;
    use aspen_testing::DeterministicClusterController;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        TestContextBuilder::new()
            .with_node_id(1)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster")
            .build()
    }

    #[test]
    fn test_can_handle_ping() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::Ping));
    }

    #[test]
    fn test_can_handle_health() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetHealth));
    }

    #[test]
    fn test_can_handle_raft_metrics() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetRaftMetrics));
    }

    #[test]
    fn test_can_handle_node_info() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetNodeInfo));
    }

    #[test]
    fn test_can_handle_leader() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetLeader));
    }

    #[test]
    fn test_can_handle_metrics() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetMetrics));
    }

    #[test]
    fn test_can_handle_checkpoint_wal() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CheckpointWal));
    }

    #[test]
    fn test_can_handle_list_vaults() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListVaults));
    }

    #[test]
    fn test_can_handle_get_vault_keys() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetVaultKeys {
            vault_name: "test".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = CoreHandler;

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![],
        }));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));
    }

    #[test]
    fn test_handler_name() {
        let handler = CoreHandler;
        assert_eq!(handler.name(), "CoreHandler");
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::Ping, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Pong => (),
            other => panic!("expected Pong, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_health() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::GetHealth, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Health(response) => {
                assert_eq!(response.node_id, 1);
                // Status should be one of: healthy, degraded, unhealthy
                assert!(
                    response.status == "healthy" || response.status == "degraded" || response.status == "unhealthy"
                );
            }
            other => panic!("expected Health, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_node_info() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::GetNodeInfo, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::NodeInfo(response) => {
                assert_eq!(response.node_id, 1);
                assert!(!response.endpoint_addr.is_empty());
            }
            other => panic!("expected NodeInfo, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_checkpoint_wal_returns_not_supported() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::CheckpointWal, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::CheckpointWalResult(response) => {
                // WAL checkpoint is not supported via trait interface
                assert!(!response.is_success);
                assert!(response.error.is_some());
            }
            other => panic!("expected CheckpointWalResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_list_vaults_deprecated() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::ListVaults, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::VaultList(response) => {
                // ListVaults is deprecated
                assert!(response.vaults.is_empty());
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("deprecated"));
            }
            other => panic!("expected VaultList, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_vault_keys_deprecated() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let request = ClientRpcRequest::GetVaultKeys {
            vault_name: "test_vault".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::VaultKeys(response) => {
                // GetVaultKeys is deprecated
                assert_eq!(response.vault, "test_vault");
                assert!(response.keys.is_empty());
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("deprecated"));
            }
            other => panic!("expected VaultKeys, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_unhandled_request() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        // This request is not handled by CoreHandler
        let request = ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not handled"));
    }

    // =========================================================================
    // Trace query tests
    // =========================================================================

    #[test]
    fn test_can_handle_trace_list() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TraceList {
            start_time_us: None,
            end_time_us: None,
            limit: None,
            continuation_token: None,
        }));
    }

    #[test]
    fn test_can_handle_trace_get() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TraceGet {
            trace_id: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_trace_search() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TraceSearch {
            operation: None,
            min_duration_us: None,
            max_duration_us: None,
            status: None,
            limit: None,
        }));
    }

    /// Helper: ingest spans into the test KV store via the handler.
    async fn ingest_test_spans(ctx: &ClientProtocolContext, spans: Vec<IngestSpan>) {
        let handler = CoreHandler;
        let result = handler.handle(ClientRpcRequest::TraceIngest { spans }, ctx).await.expect("ingest should succeed");
        match result {
            ClientRpcResponse::TraceIngestResult(r) => assert!(r.is_success),
            other => panic!("expected TraceIngestResult, got {:?}", other),
        }
    }

    fn make_test_span(
        trace_id: &str,
        span_id: &str,
        parent_id: &str,
        op: &str,
        start_us: u64,
        dur_us: u64,
        status: SpanStatusWire,
    ) -> IngestSpan {
        IngestSpan {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_id: parent_id.to_string(),
            operation: op.to_string(),
            start_time_us: start_us,
            duration_us: dur_us,
            status,
            attributes: vec![],
            events: vec![],
        }
    }

    #[tokio::test]
    async fn test_trace_get_empty() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler
            .handle(
                ClientRpcRequest::TraceGet {
                    trace_id: "nonexistent_trace_id_00000000".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceGetResult(r) => {
                assert_eq!(r.span_count, 0);
                assert!(r.spans.is_empty());
                assert!(r.error.is_none());
            }
            other => panic!("expected TraceGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_with_ingested_spans() {
        let ctx = setup_test_context().await;

        // Ingest 3 spans across 2 traces
        let spans = vec![
            make_test_span("trace_aaa", "span_01", "0000000000000000", "http.request", 1000, 500, SpanStatusWire::Ok),
            make_test_span("trace_aaa", "span_02", "span_01", "db.query", 1100, 200, SpanStatusWire::Ok),
            make_test_span(
                "trace_bbb",
                "span_03",
                "0000000000000000",
                "grpc.call",
                2000,
                1000,
                SpanStatusWire::Error("timeout".to_string()),
            ),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: None,
                    end_time_us: None,
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 2, "should have 2 traces");
                assert!(!r.is_truncated);
                // Newest first: trace_bbb (start=2000) before trace_aaa (start=1000)
                assert_eq!(r.traces[0].trace_id, "trace_bbb");
                assert_eq!(r.traces[0].span_count, 1);
                assert!(r.traces[0].has_error);
                assert_eq!(r.traces[0].root_operation.as_deref(), Some("grpc.call"));

                assert_eq!(r.traces[1].trace_id, "trace_aaa");
                assert_eq!(r.traces[1].span_count, 2);
                assert!(!r.traces[1].has_error);
                assert_eq!(r.traces[1].root_operation.as_deref(), Some("http.request"));
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_get_returns_all_spans_for_trace() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_ccc", "span_10", "0000000000000000", "root.op", 5000, 800, SpanStatusWire::Ok),
            make_test_span("trace_ccc", "span_11", "span_10", "child.op", 5100, 300, SpanStatusWire::Ok),
            make_test_span("trace_ddd", "span_12", "0000000000000000", "other.op", 6000, 100, SpanStatusWire::Unset),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceGet {
                    trace_id: "trace_ccc".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceGetResult(r) => {
                assert_eq!(r.trace_id, "trace_ccc");
                assert_eq!(r.span_count, 2);
                let ops: Vec<&str> = r.spans.iter().map(|s| s.operation.as_str()).collect();
                assert!(ops.contains(&"root.op"));
                assert!(ops.contains(&"child.op"));
            }
            other => panic!("expected TraceGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_by_operation() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_eee", "span_20", "0000000000000000", "kv.read", 7000, 50, SpanStatusWire::Ok),
            make_test_span("trace_eee", "span_21", "span_20", "kv.write", 7100, 100, SpanStatusWire::Ok),
            make_test_span("trace_eee", "span_22", "span_20", "http.request", 7200, 200, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: Some("kv".to_string()),
                    min_duration_us: None,
                    max_duration_us: None,
                    status: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 2, "should match 2 kv spans");
                for span in &r.spans {
                    assert!(
                        span.operation.to_lowercase().contains("kv"),
                        "all results should contain 'kv' in operation"
                    );
                }
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_by_duration() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_fff", "span_30", "0000000000000000", "fast.op", 8000, 10, SpanStatusWire::Ok),
            make_test_span("trace_fff", "span_31", "span_30", "medium.op", 8100, 500, SpanStatusWire::Ok),
            make_test_span("trace_fff", "span_32", "span_30", "slow.op", 8200, 5000, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        // Search for spans >= 100µs and <= 1000µs
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: None,
                    min_duration_us: Some(100),
                    max_duration_us: Some(1000),
                    status: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1, "should match 1 span (medium.op=500µs)");
                assert_eq!(r.spans[0].operation, "medium.op");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_by_error_status() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_ggg", "span_40", "0000000000000000", "ok.op", 9000, 100, SpanStatusWire::Ok),
            make_test_span(
                "trace_ggg",
                "span_41",
                "span_40",
                "err.op",
                9100,
                200,
                SpanStatusWire::Error("fail".to_string()),
            ),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: None,
                    min_duration_us: None,
                    max_duration_us: None,
                    status: Some("error".to_string()),
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.spans[0].operation, "err.op");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_time_range_filter() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_hhh", "span_50", "0000000000000000", "early", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_iii", "span_51", "0000000000000000", "late", 5000, 100, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        // Only traces starting at 3000+ should match
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: Some(3000),
                    end_time_us: None,
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.traces[0].trace_id, "trace_iii");
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Trace ingest edge cases
    // =========================================================================

    #[tokio::test]
    async fn test_trace_ingest_empty_batch() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::TraceIngest { spans: vec![] }, &ctx).await.unwrap();

        match result {
            ClientRpcResponse::TraceIngestResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.accepted_count, 0);
                assert_eq!(r.dropped_count, 0);
                assert!(r.error.is_none());
            }
            other => panic!("expected TraceIngestResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_ingest_batch_overflow_drops_excess() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;
        let max = aspen_constants::MAX_TRACE_BATCH_SIZE as usize;

        // Create batch larger than MAX_TRACE_BATCH_SIZE
        let total = max + 5;
        let spans: Vec<IngestSpan> = (0..total)
            .map(|i| {
                make_test_span(
                    "trace_overflow",
                    &format!("span_{:04}", i),
                    "0000000000000000",
                    &format!("op_{}", i),
                    1000 + i as u64,
                    10,
                    SpanStatusWire::Ok,
                )
            })
            .collect();

        let result = handler.handle(ClientRpcRequest::TraceIngest { spans }, &ctx).await.unwrap();

        match result {
            ClientRpcResponse::TraceIngestResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.accepted_count, max as u32);
                assert_eq!(r.dropped_count, 5);
                assert!(r.error.is_none());
            }
            other => panic!("expected TraceIngestResult, got {:?}", other),
        }

        // Verify only MAX_TRACE_BATCH_SIZE spans are stored
        let get_result = handler
            .handle(
                ClientRpcRequest::TraceGet {
                    trace_id: "trace_overflow".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match get_result {
            ClientRpcResponse::TraceGetResult(r) => {
                assert_eq!(r.span_count, max as u32);
            }
            other => panic!("expected TraceGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_ingest_with_attributes_and_events() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let span = IngestSpan {
            trace_id: "trace_attrs".to_string(),
            span_id: "span_a1".to_string(),
            parent_id: "0000000000000000".to_string(),
            operation: "annotated.op".to_string(),
            start_time_us: 1000,
            duration_us: 500,
            status: SpanStatusWire::Ok,
            attributes: vec![
                ("http.method".to_string(), "GET".to_string()),
                ("http.url".to_string(), "/api/v1/health".to_string()),
            ],
            events: vec![SpanEventWire {
                name: "cache.miss".to_string(),
                timestamp_us: 1100,
                attributes: vec![("key".to_string(), "user:42".to_string())],
            }],
        };

        ingest_test_spans(&ctx, vec![span]).await;

        let result = handler
            .handle(
                ClientRpcRequest::TraceGet {
                    trace_id: "trace_attrs".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceGetResult(r) => {
                assert_eq!(r.span_count, 1);
                let span = &r.spans[0];
                assert_eq!(span.attributes.len(), 2);
                assert_eq!(span.events.len(), 1);
                assert_eq!(span.events[0].name, "cache.miss");
            }
            other => panic!("expected TraceGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_ingest_duplicate_span_id_overwrites() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        // Ingest first version
        let span_v1 =
            make_test_span("trace_dup", "span_dup", "0000000000000000", "op_v1", 1000, 100, SpanStatusWire::Ok);
        ingest_test_spans(&ctx, vec![span_v1]).await;

        // Ingest second version with same trace_id + span_id
        let span_v2 =
            make_test_span("trace_dup", "span_dup", "0000000000000000", "op_v2", 2000, 200, SpanStatusWire::Ok);
        ingest_test_spans(&ctx, vec![span_v2]).await;

        // KV set overwrites — should see v2
        let result = handler
            .handle(
                ClientRpcRequest::TraceGet {
                    trace_id: "trace_dup".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceGetResult(r) => {
                assert_eq!(r.span_count, 1, "duplicate span_id should overwrite");
                assert_eq!(r.spans[0].operation, "op_v2");
            }
            other => panic!("expected TraceGetResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Trace list edge cases
    // =========================================================================

    #[tokio::test]
    async fn test_trace_list_empty_store() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: None,
                    end_time_us: None,
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 0);
                assert!(r.traces.is_empty());
                assert!(!r.is_truncated);
                assert!(r.error.is_none());
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_limit_truncates() {
        let ctx = setup_test_context().await;

        // Ingest 5 traces
        let spans: Vec<IngestSpan> = (0..5)
            .map(|i| {
                make_test_span(
                    &format!("trace_lim_{:02}", i),
                    &format!("span_lim_{:02}", i),
                    "0000000000000000",
                    &format!("op_{}", i),
                    (i as u64 + 1) * 1000,
                    100,
                    SpanStatusWire::Ok,
                )
            })
            .collect();
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: None,
                    end_time_us: None,
                    limit: Some(3),
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 3, "should be limited to 3");
                assert!(r.is_truncated, "should indicate truncation");
                // Newest first
                assert!(r.traces[0].start_time_us > r.traces[1].start_time_us);
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_end_time_filter() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_et_a", "span_et_1", "0000000000000000", "early", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_et_b", "span_et_2", "0000000000000000", "late", 5000, 100, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        // Only traces that start before 3000 should match
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: None,
                    end_time_us: Some(3000),
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.traces[0].trace_id, "trace_et_a");
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_combined_time_range() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_tr_a", "span_tr_1", "0000000000000000", "early", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_tr_b", "span_tr_2", "0000000000000000", "middle", 3000, 100, SpanStatusWire::Ok),
            make_test_span("trace_tr_c", "span_tr_3", "0000000000000000", "late", 6000, 100, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        // Window: [2000, 5000) — only "middle" (start=3000) fits
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: Some(2000),
                    end_time_us: Some(5000),
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.traces[0].trace_id, "trace_tr_b");
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_list_rootless_trace_has_no_root_operation() {
        let ctx = setup_test_context().await;

        // All spans have non-zero parent_id (no root)
        let spans = vec![
            make_test_span("trace_noroot", "span_nr_1", "span_external", "child.a", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_noroot", "span_nr_2", "span_external", "child.b", 1100, 50, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceList {
                    start_time_us: None,
                    end_time_us: None,
                    limit: None,
                    continuation_token: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceListResult(r) => {
                assert_eq!(r.count, 1);
                assert!(r.traces[0].root_operation.is_none(), "no root span → no root_operation");
                assert_eq!(r.traces[0].span_count, 2);
            }
            other => panic!("expected TraceListResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Trace search edge cases
    // =========================================================================

    #[tokio::test]
    async fn test_trace_search_case_insensitive() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_ci", "span_ci_1", "0000000000000000", "HTTP.Request", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_ci", "span_ci_2", "span_ci_1", "grpc.call", 1100, 50, SpanStatusWire::Ok),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        // Search with lowercase "http" should match "HTTP.Request"
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: Some("http".to_string()),
                    min_duration_us: None,
                    max_duration_us: None,
                    status: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.spans[0].operation, "HTTP.Request");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_combined_filters() {
        let ctx = setup_test_context().await;

        let spans = vec![
            // kv + short + ok → matches operation + duration but not status
            make_test_span("trace_cf", "span_cf_1", "0000000000000000", "kv.read", 1000, 50, SpanStatusWire::Ok),
            // kv + long + error → matches operation + status but not duration
            make_test_span(
                "trace_cf",
                "span_cf_2",
                "span_cf_1",
                "kv.write",
                1100,
                5000,
                SpanStatusWire::Error("timeout".to_string()),
            ),
            // kv + medium + error → matches all three
            make_test_span(
                "trace_cf",
                "span_cf_3",
                "span_cf_1",
                "kv.delete",
                1200,
                500,
                SpanStatusWire::Error("conflict".to_string()),
            ),
            // http + medium + error → matches duration + status but not operation
            make_test_span(
                "trace_cf",
                "span_cf_4",
                "span_cf_1",
                "http.post",
                1300,
                400,
                SpanStatusWire::Error("500".to_string()),
            ),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: Some("kv".to_string()),
                    min_duration_us: Some(100),
                    max_duration_us: Some(1000),
                    status: Some("error".to_string()),
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1, "only kv.delete matches all filters");
                assert_eq!(r.spans[0].operation, "kv.delete");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_limit_truncates() {
        let ctx = setup_test_context().await;

        let spans: Vec<IngestSpan> = (0..10u64)
            .map(|i| {
                make_test_span(
                    "trace_sl",
                    &format!("span_sl_{:02}", i),
                    "0000000000000000",
                    "matching.op",
                    i * 1000,
                    100,
                    SpanStatusWire::Ok,
                )
            })
            .collect();
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: Some("matching".to_string()),
                    min_duration_us: None,
                    max_duration_us: None,
                    status: None,
                    limit: Some(3),
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 3);
                assert!(r.is_truncated);
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_empty_store() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: Some("anything".to_string()),
                    min_duration_us: None,
                    max_duration_us: None,
                    status: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 0);
                assert!(!r.is_truncated);
                assert!(r.spans.is_empty());
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_by_ok_status() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_os", "span_os_1", "0000000000000000", "ok.op", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_os", "span_os_2", "span_os_1", "unset.op", 1100, 50, SpanStatusWire::Unset),
            make_test_span(
                "trace_os",
                "span_os_3",
                "span_os_1",
                "err.op",
                1200,
                200,
                SpanStatusWire::Error("fail".to_string()),
            ),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: None,
                    min_duration_us: None,
                    max_duration_us: None,
                    status: Some("ok".to_string()),
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.spans[0].operation, "ok.op");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trace_search_by_unset_status() {
        let ctx = setup_test_context().await;

        let spans = vec![
            make_test_span("trace_us", "span_us_1", "0000000000000000", "ok.op", 1000, 100, SpanStatusWire::Ok),
            make_test_span("trace_us", "span_us_2", "span_us_1", "unset.op", 1100, 50, SpanStatusWire::Unset),
        ];
        ingest_test_spans(&ctx, spans).await;

        let handler = CoreHandler;
        let result = handler
            .handle(
                ClientRpcRequest::TraceSearch {
                    operation: None,
                    min_duration_us: None,
                    max_duration_us: None,
                    status: Some("unset".to_string()),
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        match result {
            ClientRpcResponse::TraceSearchResult(r) => {
                assert_eq!(r.count, 1);
                assert_eq!(r.spans[0].operation, "unset.op");
            }
            other => panic!("expected TraceSearchResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Pure function unit tests
    // =========================================================================

    #[test]
    fn test_matches_status_variants() {
        let ok_span = make_test_span("t", "s", "p", "op", 0, 0, SpanStatusWire::Ok);
        let err_span = make_test_span("t", "s", "p", "op", 0, 0, SpanStatusWire::Error("x".to_string()));
        let unset_span = make_test_span("t", "s", "p", "op", 0, 0, SpanStatusWire::Unset);

        assert!(matches_status(&ok_span, "ok"));
        assert!(!matches_status(&ok_span, "error"));
        assert!(!matches_status(&ok_span, "unset"));

        assert!(matches_status(&err_span, "error"));
        assert!(!matches_status(&err_span, "ok"));
        assert!(!matches_status(&err_span, "unset"));

        assert!(matches_status(&unset_span, "unset"));
        assert!(!matches_status(&unset_span, "ok"));
        assert!(!matches_status(&unset_span, "error"));

        // Unknown filter matches everything
        assert!(matches_status(&ok_span, "unknown"));
        assert!(matches_status(&err_span, "bogus"));
    }

    #[test]
    fn test_build_trace_summary_basic() {
        let spans = vec![
            make_test_span("t1", "s1", "0000000000000000", "root.op", 1000, 500, SpanStatusWire::Ok),
            make_test_span("t1", "s2", "s1", "child.op", 1100, 200, SpanStatusWire::Ok),
        ];
        let summary = build_trace_summary("t1".to_string(), &spans);

        assert_eq!(summary.trace_id, "t1");
        assert_eq!(summary.span_count, 2);
        assert_eq!(summary.root_operation.as_deref(), Some("root.op"));
        assert_eq!(summary.start_time_us, 1000);
        // total_duration = max(1000+500, 1100+200) - 1000 = 1500 - 1000 = 500
        assert_eq!(summary.total_duration_us, 500);
        assert!(!summary.has_error);
    }

    #[test]
    fn test_build_trace_summary_with_error() {
        let spans = vec![
            make_test_span("t2", "s1", "0000000000000000", "root", 1000, 100, SpanStatusWire::Ok),
            make_test_span("t2", "s2", "s1", "fail", 1050, 50, SpanStatusWire::Error("oops".to_string())),
        ];
        let summary = build_trace_summary("t2".to_string(), &spans);

        assert!(summary.has_error);
    }

    #[test]
    fn test_build_trace_summary_no_root_span() {
        let spans = vec![
            make_test_span("t3", "s1", "external_parent", "orphan.a", 1000, 100, SpanStatusWire::Ok),
            make_test_span("t3", "s2", "external_parent", "orphan.b", 1100, 50, SpanStatusWire::Ok),
        ];
        let summary = build_trace_summary("t3".to_string(), &spans);

        assert!(summary.root_operation.is_none());
        assert_eq!(summary.span_count, 2);
    }

    #[test]
    fn test_can_handle_trace_ingest() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TraceIngest { spans: vec![] }));
    }

    // =========================================================================
    // Metric handler tests
    // =========================================================================

    #[test]
    fn test_can_handle_metric_ingest() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::MetricIngest {
            data_points: vec![],
            ttl_seconds: None,
        }));
    }

    #[test]
    fn test_can_handle_metric_list() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::MetricList {
            prefix: None,
            limit: None,
        }));
    }

    #[test]
    fn test_can_handle_metric_query() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::MetricQuery {
            name: "test".to_string(),
            start_time_us: None,
            end_time_us: None,
            label_filters: vec![],
            aggregation: None,
            step_us: None,
            limit: None,
        }));
    }

    fn make_gauge(name: &str, value: f64, timestamp_us: u64, labels: Vec<(&str, &str)>) -> MetricDataPoint {
        MetricDataPoint {
            name: name.to_string(),
            metric_type: MetricTypeWire::Gauge,
            timestamp_us,
            value,
            labels: labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            histogram_buckets: None,
            histogram_sum: None,
            histogram_count: None,
        }
    }

    #[tokio::test]
    async fn test_metric_ingest_and_query() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        // Ingest 3 data points
        let data_points = vec![
            make_gauge("cpu.usage", 50.0, 1000, vec![("node", "1")]),
            make_gauge("cpu.usage", 75.0, 2000, vec![("node", "1")]),
            make_gauge("cpu.usage", 90.0, 3000, vec![("node", "1")]),
        ];
        let result = handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricIngestResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.accepted_count, 3);
                assert_eq!(r.dropped_count, 0);
            }
            other => panic!("expected MetricIngestResult, got {:?}", other),
        }

        // Query back
        let result = handler
            .handle(
                ClientRpcRequest::MetricQuery {
                    name: "cpu.usage".to_string(),
                    start_time_us: None,
                    end_time_us: None,
                    label_filters: vec![],
                    aggregation: None,
                    step_us: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricQueryResult(r) => {
                assert!(r.error.is_none());
                assert_eq!(r.count, 3);
                assert_eq!(r.name, "cpu.usage");
            }
            other => panic!("expected MetricQueryResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_metric_ingest_drops_excess() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let max = aspen_constants::MAX_METRIC_BATCH_SIZE as usize;
        let data_points: Vec<MetricDataPoint> =
            (0..max + 10).map(|i| make_gauge("big.batch", i as f64, i as u64, vec![])).collect();
        let result = handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricIngestResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.accepted_count, max as u32);
                assert_eq!(r.dropped_count, 10);
            }
            other => panic!("expected MetricIngestResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_metric_query_time_range_filter() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let data_points = vec![
            make_gauge("latency", 10.0, 100, vec![]),
            make_gauge("latency", 20.0, 200, vec![]),
            make_gauge("latency", 30.0, 300, vec![]),
        ];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Query with time range [150, 250) — should only get t=200
        let result = handler
            .handle(
                ClientRpcRequest::MetricQuery {
                    name: "latency".to_string(),
                    start_time_us: Some(150),
                    end_time_us: Some(250),
                    label_filters: vec![],
                    aggregation: None,
                    step_us: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricQueryResult(r) => {
                assert_eq!(r.count, 1);
                assert!((r.data_points[0].value - 20.0).abs() < f64::EPSILON);
            }
            other => panic!("expected MetricQueryResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_metric_query_label_filter() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let data_points = vec![
            make_gauge("mem", 100.0, 1000, vec![("node", "1")]),
            make_gauge("mem", 200.0, 2000, vec![("node", "2")]),
            make_gauge("mem", 300.0, 3000, vec![("node", "1")]),
        ];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Filter for node=1 only
        let result = handler
            .handle(
                ClientRpcRequest::MetricQuery {
                    name: "mem".to_string(),
                    start_time_us: None,
                    end_time_us: None,
                    label_filters: vec![("node".to_string(), "1".to_string())],
                    aggregation: None,
                    step_us: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricQueryResult(r) => {
                assert_eq!(r.count, 2);
            }
            other => panic!("expected MetricQueryResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_metric_query_aggregation_avg() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let data_points = vec![
            make_gauge("rps", 10.0, 1000, vec![]),
            make_gauge("rps", 20.0, 2000, vec![]),
            make_gauge("rps", 30.0, 3000, vec![]),
        ];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        let result = handler
            .handle(
                ClientRpcRequest::MetricQuery {
                    name: "rps".to_string(),
                    start_time_us: None,
                    end_time_us: None,
                    label_filters: vec![],
                    aggregation: Some("avg".to_string()),
                    step_us: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricQueryResult(r) => {
                assert_eq!(r.count, 1);
                assert!((r.data_points[0].value - 20.0).abs() < f64::EPSILON);
            }
            other => panic!("expected MetricQueryResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_metric_list_returns_metadata() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let data_points = vec![
            make_gauge("cpu.usage", 50.0, 1000, vec![("node", "1")]),
            make_gauge("mem.used", 80.0, 1000, vec![]),
        ];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        let result = handler
            .handle(
                ClientRpcRequest::MetricList {
                    prefix: None,
                    limit: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::MetricListResult(r) => {
                assert!(r.error.is_none());
                assert_eq!(r.count, 2);
                let names: Vec<&str> = r.metrics.iter().map(|m| m.name.as_str()).collect();
                assert!(names.contains(&"cpu.usage"));
                assert!(names.contains(&"mem.used"));
            }
            other => panic!("expected MetricListResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Alert handler tests
    // =========================================================================

    #[test]
    fn test_can_handle_alert_create() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AlertCreate {
            rule: AlertRuleWire {
                name: "test".to_string(),
                metric_name: "cpu".to_string(),
                label_filters: vec![],
                aggregation: "avg".to_string(),
                window_duration_us: 300_000_000,
                comparison: AlertComparison::GreaterThan,
                threshold: 90.0,
                for_duration_us: 0,
                severity: AlertSeverity::Warning,
                description: String::new(),
                is_enabled: true,
                created_at_us: 0,
                updated_at_us: 0,
            },
        }));
    }

    #[test]
    fn test_can_handle_alert_list() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AlertList));
    }

    #[test]
    fn test_can_handle_alert_evaluate() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AlertEvaluate {
            name: "test".to_string(),
            now_us: 0,
        }));
    }

    fn make_alert_rule(name: &str, metric: &str, threshold: f64) -> AlertRuleWire {
        AlertRuleWire {
            name: name.to_string(),
            metric_name: metric.to_string(),
            label_filters: vec![],
            aggregation: "avg".to_string(),
            window_duration_us: 300_000_000, // 5 min
            comparison: AlertComparison::GreaterThan,
            threshold,
            for_duration_us: 0,
            severity: AlertSeverity::Warning,
            description: "test rule".to_string(),
            is_enabled: true,
            created_at_us: 1000,
            updated_at_us: 1000,
        }
    }

    #[tokio::test]
    async fn test_alert_create_and_get() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        let rule = make_alert_rule("high_cpu", "cpu.usage", 90.0);
        let result = handler.handle(ClientRpcRequest::AlertCreate { rule: rule.clone() }, &ctx).await.unwrap();
        match result {
            ClientRpcResponse::AlertCreateResult(r) => {
                assert!(r.is_success);
                assert_eq!(r.rule_name, "high_cpu");
            }
            other => panic!("expected AlertCreateResult, got {:?}", other),
        }

        // Get it back
        let result = handler
            .handle(
                ClientRpcRequest::AlertGet {
                    name: "high_cpu".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertGetResult(r) => {
                assert!(r.error.is_none());
                let rule = r.rule.expect("rule should exist");
                assert_eq!(rule.name, "high_cpu");
                assert_eq!(rule.metric_name, "cpu.usage");
                let state = r.state.expect("state should exist");
                assert_eq!(state.status, AlertStatus::Ok);
            }
            other => panic!("expected AlertGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_alert_delete_removes_everything() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        // Create rule
        let rule = make_alert_rule("to_delete", "cpu", 90.0);
        handler.handle(ClientRpcRequest::AlertCreate { rule }, &ctx).await.unwrap();

        // Delete it
        let result = handler
            .handle(
                ClientRpcRequest::AlertDelete {
                    name: "to_delete".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertDeleteResult(r) => {
                assert!(r.is_success);
            }
            other => panic!("expected AlertDeleteResult, got {:?}", other),
        }

        // Verify it's gone
        let result = handler
            .handle(
                ClientRpcRequest::AlertGet {
                    name: "to_delete".to_string(),
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertGetResult(r) => {
                assert!(r.rule.is_none());
                assert!(r.state.is_none());
            }
            other => panic!("expected AlertGetResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_alert_evaluate_ok_to_firing() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        // Create rule: cpu > 80 fires immediately
        let rule = make_alert_rule("cpu_high", "cpu.usage", 80.0);
        handler.handle(ClientRpcRequest::AlertCreate { rule }, &ctx).await.unwrap();

        // Ingest metric above threshold
        let data_points = vec![make_gauge("cpu.usage", 95.0, 5_000_000, vec![])];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Evaluate
        let result = handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_high".to_string(),
                    now_us: 10_000_000,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertEvaluateResult(r) => {
                assert!(r.error.is_none());
                assert_eq!(r.status, AlertStatus::Firing);
                assert!(r.did_transition);
                assert_eq!(r.previous_status, Some(AlertStatus::Ok));
                assert!(r.computed_value.unwrap() > 80.0);
            }
            other => panic!("expected AlertEvaluateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_alert_evaluate_pending_with_for_duration() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        // Create rule with for_duration_us = 60s — needs to be breached for 60s
        let mut rule = make_alert_rule("cpu_pending", "cpu.usage", 80.0);
        rule.for_duration_us = 60_000_000; // 60 seconds
        handler.handle(ClientRpcRequest::AlertCreate { rule }, &ctx).await.unwrap();

        // Ingest metric above threshold
        let data_points = vec![make_gauge("cpu.usage", 95.0, 100_000_000, vec![])];
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points,
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        // First evaluation — should go to Pending
        let result = handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_pending".to_string(),
                    now_us: 200_000_000,
                },
                &ctx,
            )
            .await
            .unwrap();
        match &result {
            ClientRpcResponse::AlertEvaluateResult(r) => {
                assert_eq!(r.status, AlertStatus::Pending);
                assert!(r.did_transition);
            }
            other => panic!("expected AlertEvaluateResult, got {:?}", other),
        }

        // Second evaluation 30s later — still Pending (duration not met)
        let result = handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_pending".to_string(),
                    now_us: 230_000_000, // 30s later
                },
                &ctx,
            )
            .await
            .unwrap();
        match &result {
            ClientRpcResponse::AlertEvaluateResult(r) => {
                assert_eq!(r.status, AlertStatus::Pending);
                assert!(!r.did_transition);
            }
            other => panic!("expected AlertEvaluateResult, got {:?}", other),
        }

        // Third evaluation 60s+ after first — should transition to Firing
        let result = handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_pending".to_string(),
                    now_us: 260_000_001, // 60s+ after first eval
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertEvaluateResult(r) => {
                assert_eq!(r.status, AlertStatus::Firing);
                assert!(r.did_transition);
                assert_eq!(r.previous_status, Some(AlertStatus::Pending));
            }
            other => panic!("expected AlertEvaluateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_alert_evaluate_firing_to_ok() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        // Create rule and trigger firing
        let rule = make_alert_rule("cpu_recover", "cpu.usage", 80.0);
        handler.handle(ClientRpcRequest::AlertCreate { rule }, &ctx).await.unwrap();

        // Ingest above threshold and evaluate to get Firing
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points: vec![make_gauge("cpu.usage", 95.0, 5_000_000, vec![])],
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();
        handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_recover".to_string(),
                    now_us: 10_000_000,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Now ingest below threshold
        handler
            .handle(
                ClientRpcRequest::MetricIngest {
                    data_points: vec![make_gauge("cpu.usage", 50.0, 15_000_000, vec![])],
                    ttl_seconds: None,
                },
                &ctx,
            )
            .await
            .unwrap();

        // Evaluate again — should resolve to Ok
        let result = handler
            .handle(
                ClientRpcRequest::AlertEvaluate {
                    name: "cpu_recover".to_string(),
                    now_us: 20_000_000,
                },
                &ctx,
            )
            .await
            .unwrap();
        match result {
            ClientRpcResponse::AlertEvaluateResult(r) => {
                assert_eq!(r.status, AlertStatus::Ok);
                assert!(r.did_transition);
                assert_eq!(r.previous_status, Some(AlertStatus::Firing));
            }
            other => panic!("expected AlertEvaluateResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_alert_list_shows_rules() {
        let handler = CoreHandler;
        let ctx = setup_test_context().await;

        handler
            .handle(
                ClientRpcRequest::AlertCreate {
                    rule: make_alert_rule("rule_a", "cpu", 80.0),
                },
                &ctx,
            )
            .await
            .unwrap();
        handler
            .handle(
                ClientRpcRequest::AlertCreate {
                    rule: make_alert_rule("rule_b", "mem", 90.0),
                },
                &ctx,
            )
            .await
            .unwrap();

        let result = handler.handle(ClientRpcRequest::AlertList, &ctx).await.unwrap();
        match result {
            ClientRpcResponse::AlertListResult(r) => {
                assert!(r.error.is_none());
                assert_eq!(r.count, 2);
                let names: Vec<&str> = r.rules.iter().map(|rws| rws.rule.name.as_str()).collect();
                assert!(names.contains(&"rule_a"));
                assert!(names.contains(&"rule_b"));
            }
            other => panic!("expected AlertListResult, got {:?}", other),
        }
    }

    // =========================================================================
    // Pure function tests
    // =========================================================================

    #[test]
    fn test_evaluate_threshold() {
        assert!(evaluate_threshold(100.0, &AlertComparison::GreaterThan, 90.0));
        assert!(!evaluate_threshold(80.0, &AlertComparison::GreaterThan, 90.0));
        assert!(evaluate_threshold(90.0, &AlertComparison::GreaterThanOrEqual, 90.0));
        assert!(evaluate_threshold(50.0, &AlertComparison::LessThan, 90.0));
        assert!(!evaluate_threshold(100.0, &AlertComparison::LessThan, 90.0));
        assert!(evaluate_threshold(90.0, &AlertComparison::LessThanOrEqual, 90.0));
        assert!(evaluate_threshold(90.0, &AlertComparison::Equal, 90.0));
        assert!(!evaluate_threshold(91.0, &AlertComparison::Equal, 90.0));
        assert!(evaluate_threshold(91.0, &AlertComparison::NotEqual, 90.0));
        assert!(!evaluate_threshold(90.0, &AlertComparison::NotEqual, 90.0));
    }

    #[test]
    fn test_compute_aggregation() {
        let values = vec![10.0, 20.0, 30.0];
        assert!((compute_aggregation(&values, "avg") - 20.0).abs() < f64::EPSILON);
        assert!((compute_aggregation(&values, "sum") - 60.0).abs() < f64::EPSILON);
        assert!((compute_aggregation(&values, "min") - 10.0).abs() < f64::EPSILON);
        assert!((compute_aggregation(&values, "max") - 30.0).abs() < f64::EPSILON);
        assert!((compute_aggregation(&values, "count") - 3.0).abs() < f64::EPSILON);
        assert!((compute_aggregation(&values, "last") - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_aggregation_empty() {
        assert!((compute_aggregation(&[], "avg") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_matches_label_filters() {
        let labels = vec![
            ("node".to_string(), "1".to_string()),
            ("region".to_string(), "us-east".to_string()),
        ];
        // All match
        assert!(matches_label_filters(&labels, &[("node".to_string(), "1".to_string())]));
        // Multi-filter
        assert!(matches_label_filters(&labels, &[
            ("node".to_string(), "1".to_string()),
            ("region".to_string(), "us-east".to_string()),
        ]));
        // Mismatch
        assert!(!matches_label_filters(&labels, &[("node".to_string(), "2".to_string())]));
        // Empty filter matches everything
        assert!(matches_label_filters(&labels, &[]));
    }

    #[test]
    fn test_compute_alert_transition_ok_to_firing_immediate() {
        let state = AlertStateWire {
            rule_name: "test".to_string(),
            status: AlertStatus::Ok,
            last_value: None,
            last_evaluated_us: 0,
            condition_since_us: None,
            last_fired_us: None,
            last_resolved_us: None,
        };
        let rule = make_alert_rule("test", "cpu", 80.0);
        let result = compute_alert_transition(&AlertStatus::Ok, true, &state, &rule, 1000);
        assert_eq!(result, AlertStatus::Firing);
    }

    #[test]
    fn test_compute_alert_transition_ok_to_pending_with_duration() {
        let state = AlertStateWire {
            rule_name: "test".to_string(),
            status: AlertStatus::Ok,
            last_value: None,
            last_evaluated_us: 0,
            condition_since_us: None,
            last_fired_us: None,
            last_resolved_us: None,
        };
        let mut rule = make_alert_rule("test", "cpu", 80.0);
        rule.for_duration_us = 60_000_000;
        let result = compute_alert_transition(&AlertStatus::Ok, true, &state, &rule, 1000);
        assert_eq!(result, AlertStatus::Pending);
    }

    #[test]
    fn test_compute_alert_transition_firing_to_ok() {
        let state = AlertStateWire {
            rule_name: "test".to_string(),
            status: AlertStatus::Firing,
            last_value: Some(95.0),
            last_evaluated_us: 1000,
            condition_since_us: Some(500),
            last_fired_us: Some(800),
            last_resolved_us: None,
        };
        let rule = make_alert_rule("test", "cpu", 80.0);
        let result = compute_alert_transition(&AlertStatus::Firing, false, &state, &rule, 2000);
        assert_eq!(result, AlertStatus::Ok);
    }
}
