# Implementation Plan: Aspen Event Hook System

## Overview

 Add an event-driven hook system to Aspen that leverages the existing aspen-pubsub and aspen-jobs infrastructure. The system uses a hybrid approach:

 1. Fast path (Direct): In-process handlers for low-latency event processing
 2. Reliable path (Jobs): Submit to job queue for handlers needing retry, DLQ, persistence

 This design reuses existing infrastructure while providing flexibility for different use cases.

 Key Design Decisions

- Built on aspen-pubsub: Use existing pub/sub for event distribution (Raft ordering, wildcards)
- Built on aspen-jobs: Use existing Worker/Job infrastructure for reliable execution
- Hybrid execution: Direct handlers (fast) OR job-based handlers (reliable)
- Topic-based routing: Events published to hooks.kv.write, hooks.cluster.leader_elected, etc.
- NATS-style wildcards: Subscribe to hooks.> for all, hooks.kv.* for KV events
- Tiger Style compliant: Explicit bounds on concurrency, timeouts, buffer sizes

 Architecture

 Aspen Events                           Pub/Sub Layer (aspen-pubsub)
      |                                          |
      v                                          v
 KV Operations --> HookPublisher --> Topic: "hooks.kv.write_committed"
 Cluster Events -> HookPublisher --> Topic: "hooks.cluster.leader_elected"
                                           |
                                           v
                                    Raft Consensus
                                           |
                                           v
                                    HookService
                           (subscribes to hook topics)
                                           |
                +--------------------------+-------------------------+
                |                          |                         |
                v                          v                         v
         Direct Handler            Job-Based Handler           Shell Handler
       (InProcessHandler)          (HookJobWorker)           (ShellHandler)
         [fast path]              [reliable path]             [local exec]
                |                          |
                |                          v
                |                    JobManager
                |                   (retry, DLQ)
                |                          |
                v                          v
           Immediate                 Worker Pool
           Execution                (poll-based)

 Execution Paths

 Fast Path (Direct): For in-process handlers that don't need persistence
 Event --> HookService --> InProcessHandler --> Immediate async execution

 Reliable Path (Jobs): For handlers needing retry, DLQ, or persistence
 Event --> HookService --> JobManager.submit(hook_job) --> WorkerPool --> HookJobWorker

 Shell Path: For local command execution (can be direct or job-based)
 Event --> HookService --> ShellHandler --> Process group execution

 Topic Hierarchy

 hooks.
   kv.
     write_committed      - KV write operations
     delete_committed     - KV delete operations
     ttl_expired          - TTL expirations
   cluster.
     leader_elected       - New leader elected
     membership_changed   - Voters/learners changed
     node_added           - Node joined cluster
     node_removed         - Node left cluster
   system.
     snapshot_created     - Snapshot taken
     snapshot_installed   - Snapshot restored
     health_changed       - Node health status change

 Events to Support

 | Event Type         | Source         | Payload                          |
 |--------------------|----------------|----------------------------------|
 | write_committed    | log_broadcast  | key, value, operation, log_index |
 | delete_committed   | log_broadcast  | key, log_index                   |
 | leader_elected     | raft.metrics() | new_leader_id, voters, learners  |
 | membership_changed | raft.metrics() | voters, learners, affected_node  |
 | snapshot_created   | storage        | snapshot_index, term             |
 | health_changed     | health monitor | previous_state, current_state    |

 Implementation Steps

 Phase 1: Core Crate Structure

 Create crates/aspen-hooks/

 Files to create:

- Cargo.toml - Dependencies: tokio, aspen-pubsub, serde, snafu, tracing
- src/lib.rs - Public API re-exports
- src/constants.rs - Tiger Style bounds
- src/error.rs - snafu error types
- src/config.rs - HooksConfig, HookHandlerConfig
- src/publisher.rs - HookPublisher (wraps RaftPublisher)
- src/handlers.rs - HookHandler trait + InProcessHandler, ShellHandler
- src/forward.rs - ForwardHandler for cross-cluster
- src/service.rs - HookService (subscribe + route)
- src/retry.rs - Exponential backoff logic
- src/metrics.rs - Execution metrics

 Phase 2: Constants (src/constants.rs)

 use std::time::Duration;

 // Handler limits
 pub const MAX_HANDLERS: usize = 64;
 pub const MAX_HANDLER_NAME_SIZE: usize = 128;
 pub const MAX_PATTERN_SIZE: usize = 256;
 pub const MAX_SHELL_COMMAND_SIZE: usize = 4096;

 // Execution limits
 pub const MAX_CONCURRENT_HANDLER_EXECUTIONS: usize = 32;
 pub const MAX_HANDLER_TIMEOUT_MS: u64 = 30_000;
 pub const DEFAULT_HANDLER_TIMEOUT_MS: u64 = 5_000;
 pub const MAX_HANDLER_RETRY_COUNT: u32 = 5;
 pub const DEFAULT_HANDLER_RETRY_COUNT: u32 = 3;

 // Retry backoff
 pub const RETRY_INITIAL_BACKOFF_MS: u64 = 100;
 pub const RETRY_MAX_BACKOFF_MS: u64 = 10_000;

 // Shell specific
 pub const MAX_SHELL_OUTPUT_SIZE: usize = 64 * 1024;
 pub const SHELL_GRACE_PERIOD: Duration = Duration::from_secs(5);

 // Topic prefixes (reserved namespace)
 pub const HOOK_TOPIC_PREFIX: &str = "hooks";

 Phase 3: Event Types (src/event.rs)

 #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum HookEventType {
     WriteCommitted,
     DeleteCommitted,
     LeaderElected,
     MembershipChanged,
     SnapshotCreated,
     HealthChanged,
 }

 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct HookEvent {
     pub event_type: HookEventType,
     pub log_index: Option<u64>,
     pub timestamp: SerializableTimestamp,
     pub node_id: u64,
     pub payload: serde_json::Value,
 }

 Phase 4: Configuration (src/config.rs)

 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct HooksConfig {
     #[serde(default = "default_enabled")]
     pub enabled: bool,

     /// Topic patterns to publish hook events for (empty = publish all)
     #[serde(default)]
     pub publish_topics: Vec<String>,

     /// Hook handlers that process events
     #[serde(default)]
     pub handlers: Vec<HookHandlerConfig>,
 }

 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct HookHandlerConfig {
     pub name: String,

     /// Topic pattern to subscribe to (NATS-style: hooks.>, hooks.kv.*, etc.)
     pub pattern: String,

     /// Execution mode: direct (fast) or job (reliable with retry/DLQ)
     #[serde(default)]
     pub execution_mode: ExecutionMode,

     /// Handler type
     #[serde(flatten)]
     pub handler_type: HookHandlerType,

     #[serde(default = "default_timeout_ms")]
     pub timeout_ms: u64,

     #[serde(default = "default_retry_count")]
     pub retry_count: u32,

     /// Job priority (only used when execution_mode = job)
     #[serde(default)]
     pub job_priority: Option<String>,

     #[serde(default = "default_enabled")]
     pub enabled: bool,
 }

 #[derive(Debug, Clone, Default, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum ExecutionMode {
     /// Direct async execution (fast path, no persistence)
     #[default]
     Direct,
     /// Submit as job (reliable path with retry, DLQ, persistence)
     Job,
 }

 #[derive(Debug, Clone, Serialize, Deserialize)]
 #[serde(tag = "type", rename_all = "snake_case")]
 pub enum HookHandlerType {
     /// In-process Rust async callback
     InProcess { handler_id: String },

     /// Execute local shell command (event passed via ASPEN_HOOK_EVENT env var)
     Shell { command: String, working_dir: Option<String> },

     /// Forward to another Aspen node's pub/sub (useful for cross-cluster)
     Forward {
         /// Remote cluster's gossip topic or node ticket
         target_cluster: String,
         /// Topic to publish to on remote cluster
         target_topic: String,
     },
 }

 Phase 5: Hook Implementations

 Direct Handlers (src/handlers.rs)

 HookHandler trait:
 #[async_trait]
 pub trait HookHandler: Send + Sync {
     /// Handle an event from pub/sub (direct execution)
     async fn handle(&self, event: &Event) -> Result<()>;
     fn clone_box(&self) -> Box<dyn HookHandler>;
 }

 InProcessHandler: Invoke registered Rust async closure

- Fast path for reactive programming
- Zero serialization overhead
- Register handlers with unique IDs

 ShellHandler: Execute command with event as ASPEN_HOOK_EVENT env var

- Follow pattern from crates/aspen-jobs/src/workers/shell_command.rs
- Use sh -c for command execution
- Process group management for clean termination

 ForwardHandler: Forward events to another Aspen cluster's pub/sub

- Connect to remote cluster via Iroh
- Republish event to target topic on remote cluster

 Job-Based Handler (src/worker.rs)

 HookJobWorker - Implements Worker trait from aspen-jobs:
 pub struct HookJobWorker {
     handlers: HashMap<String, Arc<dyn HookHandler>>,
     shell_config: ShellHandlerConfig,
 }

 #[async_trait]
 impl Worker for HookJobWorker {
     async fn execute(&self, job: Job) -> JobResult {
         let payload: HookJobPayload = serde_json::from_value(job.spec.payload)?;

         match payload.handler_type {
             "in_process" => {
                 let handler = self.handlers.get(&payload.handler_id)?;
                 handler.handle(&payload.event).await?;
             }
             "shell" => {
                 execute_shell(&payload.command, &payload.event).await?;
             }
             "forward" => {
                 forward_event(&payload.target, &payload.event).await?;
             }
         }

         JobResult::success(json!({"handled": true}))
     }

     fn job_types(&self) -> Vec<String> {
         vec!["hook".to_string()]
     }
 }

 Benefits of job-based execution:

- Automatic retry with exponential backoff
- Dead Letter Queue for failed handlers
- Persistent job state (survives restarts)
- Visibility timeout prevents duplicate execution
- Priority levels (critical, high, normal, low)

 Phase 6: Hook Service (src/service.rs)

 HookService - Main service that routes events to handlers:

 pub struct HookService {
     config: HooksConfig,
     kv_store: Arc<dyn KeyValueStore>,
     job_manager: Arc<JobManager>,
     direct_handlers: HashMap<String, Box<dyn HookHandler>>,
     semaphore: Arc<Semaphore>,
     cancel: CancellationToken,
 }

 impl HookService {
     pub async fn run(self) {
         for handler_config in &self.config.handlers {
             let pattern = TopicPattern::new(&handler_config.pattern)?;
             let mut stream = subscribe(&self.kv_store, pattern).await?;

             match handler_config.execution_mode {
                 ExecutionMode::Direct => {
                     // Fast path: execute handler immediately
                     tokio::spawn(async move {
                         while let Some(event) = stream.next().await {
                             let permit = semaphore.acquire().await?;
                             let handler = direct_handlers.get(&handler_config.name)?;

                             tokio::spawn(async move {
                                 let _permit = permit;
                                 handler.handle(&event).await;
                             });
                         }
                     });
                 }
                 ExecutionMode::Job => {
                     // Reliable path: submit to job queue
                     tokio::spawn(async move {
                         while let Some(event) = stream.next().await {
                             let job = JobSpec::new("hook")
                                 .payload(HookJobPayload {
                                     handler_name: handler_config.name.clone(),
                                     event: event.clone(),
                                     handler_type: handler_config.handler_type.clone(),
                                 })
                                 .priority(handler_config.job_priority)
                                 .timeout(handler_config.timeout_ms)
                                 .max_retries(handler_config.retry_count);

                             job_manager.submit(job).await?;
                         }
                     });
                 }
             }
         }
     }
 }

 Key routing logic:

- execution_mode: direct → Immediate async execution with semaphore
- execution_mode: job → Submit to JobManager for reliable execution

 Phase 7: Integration with NodeConfig

 Modify crates/aspen-cluster/src/config.rs:

 pub struct NodeConfig {
     // ... existing fields ...

     /// Webhook hooks configuration.
     #[serde(default)]
     pub hooks: HooksConfig,
 }

 Phase 8: Bootstrap Integration

 Modify crates/aspen-cluster/src/bootstrap.rs:

 1. Create mpsc::channel for hook events
 2. Create HookExecutor from config
 3. Spawn executor task
 4. Spawn KV event forwarder (subscribes to log_broadcast)
 5. Spawn cluster event forwarder (watches raft.metrics())

 Phase 9: Testing

 1. Unit tests: Config validation, filter matching, retry logic
 2. Integration tests: HTTP hook with wiremock, shell hook with test scripts
 3. Madsim tests: Hook execution under network partition, concurrency limits

 Files to Modify

 | File                                  | Change                        |
 |---------------------------------------|-------------------------------|
 | Cargo.toml (workspace)                | Add aspen-hooks crate         |
 | crates/aspen-cluster/src/config.rs    | Add HooksConfig to NodeConfig |
 | crates/aspen-cluster/src/bootstrap.rs | Start hook executor           |
 | crates/aspen-cluster/Cargo.toml       | Depend on aspen-hooks         |

 Files to Create

 | File                                | Purpose                                                            |
 |-------------------------------------|--------------------------------------------------------------------|
 | crates/aspen-hooks/Cargo.toml       | Crate manifest                                                     |
 | crates/aspen-hooks/src/lib.rs       | Public API                                                         |
 | crates/aspen-hooks/src/constants.rs | Tiger Style bounds                                                 |
 | crates/aspen-hooks/src/error.rs     | Error types (snafu)                                                |
 | crates/aspen-hooks/src/config.rs    | HooksConfig, HookHandlerConfig, ExecutionMode                      |
 | crates/aspen-hooks/src/publisher.rs | HookPublisher (wraps RaftPublisher for hook topics)                |
 | crates/aspen-hooks/src/handlers.rs  | HookHandler trait + InProcessHandler, ShellHandler, ForwardHandler |
 | crates/aspen-hooks/src/worker.rs    | HookJobWorker (implements Worker trait for job-based execution)    |
 | crates/aspen-hooks/src/service.rs   | HookService (subscribes to topics, routes to handlers/jobs)        |
 | crates/aspen-hooks/src/metrics.rs   | Execution metrics                                                  |

 Example TOML Configuration

 [hooks]
 enabled = true

# Publish all hook events (default if empty)

 publish_topics = ["hooks.>"]

# Fast path: In-process handler for low-latency cache invalidation

 [[hooks.handlers]]
 name = "cache-invalidation"
 pattern = "hooks.kv.write_committed"
 execution_mode = "direct"   # Fast path (default)
 type = "in_process"
 handler_id = "cache_invalidator"

# Reliable path: Shell script with retry and DLQ

 [[hooks.handlers]]
 name = "external-notification"
 pattern = "hooks.cluster.leader_elected"
 execution_mode = "job"      # Reliable path with retry/DLQ
 type = "shell"
 command = "/usr/local/bin/notify-leader-change.sh"
 timeout_ms = 30000
 retry_count = 5
 job_priority = "high"

# Reliable path: Forward to analytics with guaranteed delivery

 [[hooks.handlers]]
 name = "analytics-forwarder"
 pattern = "hooks.kv.*"
 execution_mode = "job"
 type = "forward"
 target_cluster = "aspen://analytics-cluster-ticket..."
 target_topic = "ingested.kv_events"
 retry_count = 3
 job_priority = "normal"

# Fast path: Audit logging (fire-and-forget)

 [[hooks.handlers]]
 name = "audit-logger"
 pattern = "hooks.>"
 execution_mode = "direct"
 type = "in_process"
 handler_id = "audit_logger"

 Programmatic In-Process Hook Registration

 use aspen_hooks::{HookService, Event};

 // Register in-process handlers before starting service
 let service = HookService::new(config, kv_store)?
     .register_handler("cache_invalidator", |event: &Event| {
         Box::pin(async move {
             // Invalidate cache based on event topic and payload
             let key = event.topic.as_str();
             cache.invalidate(key).await?;
             Ok(())
         })
     })
     .register_handler("audit_logger", |event: &Event| {
         Box::pin(async move {
             audit_log.record(event).await?;
             Ok(())
         })
     });

 // Start the service (spawns subscription tasks)
 tokio::spawn(service.run());

 Subscribing to Hooks Programmatically

 Any code can subscribe to hook events via pub/sub:

 use aspen_pubsub::{subscribe, TopicPattern};
 use futures::StreamExt;

 // Subscribe to all KV write events
 let pattern = TopicPattern::new("hooks.kv.write_committed")?;
 let mut stream = subscribe(&kv_store, pattern).await?;

 while let Some(event) = stream.next().await {
     let event = event?;
     println!("Got write event: {:?}", event.payload);
 }

 Cross-Cluster Event Forwarding

 For forwarding events to remote clusters, the ForwardHandler uses Iroh to connect to the remote cluster and publishes events to their pub/sub:

 pub struct ForwardHandler {
     /// Remote cluster connection (via Iroh)
     remote_client: Arc<AspenClient>,
     /// Topic to publish to on remote cluster
     target_topic: Topic,
 }

 impl HookHandler for ForwardHandler {
     async fn handle(&self, event: &Event) -> Result<()> {
         // Republish to remote cluster's pub/sub
         self.remote_client.publish(&self.target_topic, &event.payload).await
     }
 }

 This enables:

- Cross-cluster event propagation
- Analytics pipelines
- Multi-region event synchronization

 Implementation Order

 1. Create aspen-hooks crate with Cargo.toml, constants, errors
 2. Implement HooksConfig, HookHandlerConfig, ExecutionMode with validation
 3. Implement HookPublisher (wraps RaftPublisher for hook topics)
 4. Implement HookHandler trait and direct handlers (InProcess, Shell, Forward)
 5. Implement HookJobWorker (implements Worker trait from aspen-jobs)
 6. Implement HookService (subscribe + hybrid routing: direct vs job)
 7. Add HooksConfig to NodeConfig
 8. Integrate HookService in bootstrap (with JobManager reference)
 9. Register HookJobWorker with WorkerPool
 10. Add hook publishing from KV operations and cluster events
 11. Write tests (unit, integration, pub/sub + jobs integration)
 12. Documentation

 Reference Files

 Pub/Sub:

- /home/brittonr/git/aspen/crates/aspen-pubsub/src/lib.rs - Pub/sub API
- /home/brittonr/git/aspen/crates/aspen-pubsub/src/publisher.rs - RaftPublisher
- /home/brittonr/git/aspen/crates/aspen-pubsub/src/subscriber.rs - EventStream
- /home/brittonr/git/aspen/tests/pubsub_integration_test.rs - Pub/sub tests

 Jobs/Workers:

- /home/brittonr/git/aspen/crates/aspen-jobs/src/worker.rs - Worker trait
- /home/brittonr/git/aspen/crates/aspen-jobs/src/job.rs - Job, JobSpec, JobResult
- /home/brittonr/git/aspen/crates/aspen-jobs/src/manager.rs - JobManager
- /home/brittonr/git/aspen/crates/aspen-jobs/src/workers/shell_command.rs - ShellCommandWorker

 Core:

- /home/brittonr/git/aspen/crates/aspen-raft/src/log_subscriber.rs - LogEntryPayload
- /home/brittonr/git/aspen/crates/aspen-raft/src/membership_watcher.rs - Metrics watching
- /home/brittonr/git/aspen/crates/aspen-cluster/src/config.rs - Configuration
- /home/brittonr/git/aspen/crates/aspen-constants/src/lib.rs - Constants
