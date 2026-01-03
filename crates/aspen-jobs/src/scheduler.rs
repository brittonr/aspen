//! Advanced job scheduling service with sub-second precision.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use aspen_core::KeyValueStore;

use crate::error::{JobError, Result};
use crate::job::{Job, JobId, JobSpec};
use crate::manager::JobManager;
use crate::types::Schedule;

/// Scheduling service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// How often to check for due jobs (milliseconds).
    pub tick_interval_ms: u64,
    /// Maximum jobs to process per tick.
    pub max_jobs_per_tick: usize,
    /// Enable jitter to prevent thundering herd.
    pub enable_jitter: bool,
    /// Maximum jitter in milliseconds.
    pub max_jitter_ms: u64,
    /// Enable catch-up for missed executions.
    pub enable_catch_up: bool,
    /// Time window to look ahead for scheduling (seconds).
    pub lookahead_window_sec: u64,
    /// Enable timezone-aware scheduling.
    pub timezone_aware: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 100, // 100ms for sub-second precision
            max_jobs_per_tick: 1000,
            enable_jitter: true,
            max_jitter_ms: 5000, // Up to 5 seconds jitter
            enable_catch_up: true,
            lookahead_window_sec: 300, // 5 minutes lookahead
            timezone_aware: false,
        }
    }
}

/// Policy for handling missed scheduled executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CatchUpPolicy {
    /// Execute immediately when discovered.
    RunImmediately,
    /// Skip missed execution, wait for next schedule.
    Skip,
    /// Run all missed instances.
    RunAll,
    /// Run only the most recent missed instance.
    RunLatest,
}

/// Policy for handling scheduling conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictPolicy {
    /// Skip if previous instance is still running.
    Skip,
    /// Queue behind running instance.
    Queue,
    /// Allow parallel execution.
    Parallel,
    /// Cancel previous and start new.
    Cancel,
}

/// Scheduled job entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Job ID.
    pub job_id: JobId,
    /// Job specification.
    pub spec: JobSpec,
    /// Next execution time.
    pub next_execution: DateTime<Utc>,
    /// Schedule definition.
    pub schedule: Schedule,
    /// Last execution time.
    pub last_execution: Option<DateTime<Utc>>,
    /// Number of executions.
    pub execution_count: u64,
    /// Missed execution count.
    pub missed_count: u64,
    /// Is schedule paused.
    pub paused: bool,
    /// Catch-up policy for missed executions.
    pub catch_up_policy: CatchUpPolicy,
    /// Conflict policy.
    pub conflict_policy: ConflictPolicy,
    /// Timezone for execution (if timezone-aware).
    pub timezone: Option<String>,
}

impl ScheduledJob {
    /// Calculate next execution time.
    pub fn calculate_next_execution(&self) -> Option<DateTime<Utc>> {
        self.schedule.next_execution()
    }

    /// Check if job is due for execution.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        !self.paused && self.next_execution <= now
    }

    /// Update after execution.
    pub fn mark_executed(&mut self, at: DateTime<Utc>) {
        self.last_execution = Some(at);
        self.execution_count += 1;
        if let Some(next) = self.calculate_next_execution() {
            self.next_execution = next;
        }
    }

    /// Mark as missed.
    pub fn mark_missed(&mut self) {
        self.missed_count += 1;
    }
}

/// Job scheduling service.
pub struct SchedulerService<S: KeyValueStore + ?Sized> {
    /// Job manager reference.
    manager: Arc<JobManager<S>>,
    /// Scheduler configuration.
    config: SchedulerConfig,
    /// Scheduled jobs indexed by execution time.
    /// Using BTreeMap for efficient time-based operations.
    schedule_index: Arc<RwLock<BTreeMap<DateTime<Utc>, Vec<JobId>>>>,
    /// Job details by ID.
    jobs: Arc<RwLock<HashMap<JobId, ScheduledJob>>>,
    /// Currently executing jobs (to handle conflicts).
    executing: Arc<Mutex<HashSet<JobId>>>,
    /// Shutdown signal.
    shutdown: Arc<tokio::sync::Notify>,
}

impl<S: KeyValueStore + ?Sized + 'static> SchedulerService<S> {
    /// Create a new scheduler service.
    pub fn new(manager: Arc<JobManager<S>>) -> Self {
        Self::with_config(manager, SchedulerConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(manager: Arc<JobManager<S>>, config: SchedulerConfig) -> Self {
        Self {
            manager,
            config,
            schedule_index: Arc::new(RwLock::new(BTreeMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            executing: Arc::new(Mutex::new(HashSet::new())),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Start the scheduler service.
    pub async fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        info!("Starting scheduler service");

        let service = self.clone();
        tokio::spawn(async move {
            service.run().await;
        })
    }

    /// Run the scheduler loop.
    async fn run(&self) {
        let mut ticker = interval(tokio::time::Duration::from_millis(self.config.tick_interval_ms));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.tick().await {
                        error!(error = %e, "scheduler tick failed");
                    }
                }
                _ = self.shutdown.notified() => {
                    info!("Scheduler service shutting down");
                    break;
                }
            }
        }
    }

    /// Process one scheduler tick.
    async fn tick(&self) -> Result<()> {
        let now = Utc::now();
        let mut jobs_to_execute = Vec::new();

        // Find due jobs
        {
            let schedule_index = self.schedule_index.read().await;
            let jobs = self.jobs.read().await;

            // Get all jobs due up to now + jitter window
            let cutoff = now + Duration::milliseconds(self.config.max_jitter_ms as i64);

            for (scheduled_time, job_ids) in schedule_index.range(..=cutoff) {
                for job_id in job_ids {
                    if let Some(scheduled_job) = jobs.get(job_id) {
                        if scheduled_job.is_due(now) {
                            jobs_to_execute.push((job_id.clone(), scheduled_job.clone()));
                        }
                    }
                }

                // Limit jobs per tick
                if jobs_to_execute.len() >= self.config.max_jobs_per_tick {
                    break;
                }
            }
        }

        // Execute due jobs
        for (job_id, mut scheduled_job) in jobs_to_execute {
            if let Err(e) = self.execute_scheduled_job(&job_id, &mut scheduled_job).await {
                warn!(
                    job_id = %job_id,
                    error = %e,
                    "failed to execute scheduled job"
                );
            }
        }

        // Clean up old entries
        self.cleanup_old_entries(now).await?;

        Ok(())
    }

    /// Execute a scheduled job.
    async fn execute_scheduled_job(&self, job_id: &JobId, scheduled_job: &mut ScheduledJob) -> Result<()> {
        let now = Utc::now();

        // Check conflict policy
        let is_executing = self.executing.lock().await.contains(job_id);
        if is_executing {
            match scheduled_job.conflict_policy {
                ConflictPolicy::Skip => {
                    debug!(job_id = %job_id, "skipping due to conflict");
                    scheduled_job.mark_missed();
                    self.update_scheduled_job(scheduled_job).await?;
                    return Ok(());
                }
                ConflictPolicy::Queue => {
                    // Will be queued by job manager
                }
                ConflictPolicy::Parallel => {
                    // Allow parallel execution
                }
                ConflictPolicy::Cancel => {
                    // Cancel previous (would need job manager support)
                    warn!(job_id = %job_id, "cancel policy not yet implemented");
                }
            }
        }

        // Apply jitter if enabled
        if self.config.enable_jitter {
            let jitter_ms = rand::random::<u64>() % self.config.max_jitter_ms;
            if jitter_ms > 0 {
                debug!(job_id = %job_id, jitter_ms, "applying scheduling jitter");
                tokio::time::sleep(tokio::time::Duration::from_millis(jitter_ms)).await;
            }
        }

        // Submit job for execution
        self.executing.lock().await.insert(job_id.clone());

        let new_job_id = self.manager.submit(scheduled_job.spec.clone()).await?;

        info!(
            job_id = %job_id,
            new_job_id = %new_job_id,
            "scheduled job submitted for execution"
        );

        // Update scheduled job
        scheduled_job.mark_executed(now);
        self.update_scheduled_job(scheduled_job).await?;

        // Remove from executing set (with delay to handle immediate completion)
        let executing = self.executing.clone();
        let job_id = job_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            executing.lock().await.remove(&job_id);
        });

        Ok(())
    }

    /// Add a job to the schedule.
    pub async fn schedule_job(&self, spec: JobSpec, schedule: Schedule) -> Result<JobId> {
        let job_id = JobId::new();

        let next_execution = schedule.next_execution().ok_or_else(|| JobError::InvalidJobSpec {
            reason: "Schedule has no future execution time".to_string(),
        })?;

        let scheduled_job = ScheduledJob {
            job_id: job_id.clone(),
            spec,
            next_execution,
            schedule,
            last_execution: None,
            execution_count: 0,
            missed_count: 0,
            paused: false,
            catch_up_policy: CatchUpPolicy::RunImmediately,
            conflict_policy: ConflictPolicy::Skip,
            timezone: None,
        };

        // Add to indices
        {
            let mut schedule_index = self.schedule_index.write().await;
            let mut jobs = self.jobs.write().await;

            schedule_index.entry(next_execution).or_insert_with(Vec::new).push(job_id.clone());

            jobs.insert(job_id.clone(), scheduled_job);
        }

        info!(
            job_id = %job_id,
            next_execution = %next_execution,
            "job scheduled"
        );

        Ok(job_id)
    }

    /// Cancel a scheduled job.
    pub async fn cancel_scheduled(&self, job_id: &JobId) -> Result<()> {
        let mut schedule_index = self.schedule_index.write().await;
        let mut jobs = self.jobs.write().await;

        if let Some(scheduled_job) = jobs.remove(job_id) {
            // Remove from schedule index
            if let Some(job_ids) = schedule_index.get_mut(&scheduled_job.next_execution) {
                job_ids.retain(|id| id != job_id);
                if job_ids.is_empty() {
                    schedule_index.remove(&scheduled_job.next_execution);
                }
            }

            info!(job_id = %job_id, "scheduled job cancelled");
            Ok(())
        } else {
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Pause a scheduled job.
    pub async fn pause_scheduled(&self, job_id: &JobId) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(scheduled_job) = jobs.get_mut(job_id) {
            scheduled_job.paused = true;
            info!(job_id = %job_id, "scheduled job paused");
            Ok(())
        } else {
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Resume a paused scheduled job.
    pub async fn resume_scheduled(&self, job_id: &JobId) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if let Some(scheduled_job) = jobs.get_mut(job_id) {
            scheduled_job.paused = false;

            // Recalculate next execution if needed
            if let Some(next) = scheduled_job.calculate_next_execution() {
                scheduled_job.next_execution = next;
            }

            info!(job_id = %job_id, "scheduled job resumed");
            Ok(())
        } else {
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Get upcoming scheduled jobs.
    pub async fn get_upcoming(&self, limit: usize) -> Result<Vec<ScheduledJob>> {
        let schedule_index = self.schedule_index.read().await;
        let jobs = self.jobs.read().await;
        let mut result = Vec::new();

        for (_time, job_ids) in schedule_index.iter().take(limit) {
            for job_id in job_ids {
                if let Some(job) = jobs.get(job_id) {
                    result.push(job.clone());
                }
            }
        }

        Ok(result)
    }

    /// Update scheduled job in indices.
    async fn update_scheduled_job(&self, scheduled_job: &ScheduledJob) -> Result<()> {
        let mut schedule_index = self.schedule_index.write().await;
        let mut jobs = self.jobs.write().await;

        // Remove old index entry if execution time changed
        if let Some(existing) = jobs.get(&scheduled_job.job_id) {
            if existing.next_execution != scheduled_job.next_execution {
                if let Some(job_ids) = schedule_index.get_mut(&existing.next_execution) {
                    job_ids.retain(|id| id != &scheduled_job.job_id);
                    if job_ids.is_empty() {
                        schedule_index.remove(&existing.next_execution);
                    }
                }

                // Add new index entry
                schedule_index
                    .entry(scheduled_job.next_execution)
                    .or_insert_with(Vec::new)
                    .push(scheduled_job.job_id.clone());
            }
        }

        jobs.insert(scheduled_job.job_id.clone(), scheduled_job.clone());
        Ok(())
    }

    /// Clean up old schedule entries.
    async fn cleanup_old_entries(&self, cutoff: DateTime<Utc>) -> Result<()> {
        let mut schedule_index = self.schedule_index.write().await;

        // Remove entries older than cutoff
        let old_times: Vec<_> = schedule_index.range(..cutoff).map(|(time, _)| *time).collect();

        for time in old_times {
            schedule_index.remove(&time);
        }

        Ok(())
    }

    /// Shutdown the scheduler.
    pub async fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}
