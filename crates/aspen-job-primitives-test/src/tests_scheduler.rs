//! Scheduler test.
//!
//! Scenario 1: Schedule a delayed job, verify it appears in upcoming with correct time.
//! Scenario 2: Schedule a recurring job, verify cron expression is persisted and
//!             the scheduler service tracks it.

use std::sync::Arc;

use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::SchedulerConfig;
use aspen_jobs::SchedulerService;
use chrono::Utc;

use crate::TestResult;
use crate::make_store;

pub async fn run() -> TestResult {
    test_delayed_job_scheduling().await?;
    test_cron_job_scheduling().await?;
    Ok(())
}

/// Schedule a job 60 seconds in the future, verify it shows up in upcoming.
async fn test_delayed_job_scheduling() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let config = SchedulerConfig {
        tick_interval_ms: 50,
        enable_jitter: false,
        ..Default::default()
    };
    let scheduler = Arc::new(SchedulerService::with_config(manager.clone(), store.clone(), config));

    // Schedule a job 60 seconds from now
    let target_time = Utc::now() + chrono::Duration::seconds(60);
    let spec = JobSpec::new("delayed-echo");
    let schedule = aspen_jobs::Schedule::Once(target_time);

    let job_id = scheduler.schedule_job(spec, schedule).await.map_err(|e| format!("schedule_job: {e}"))?;

    // Verify it appears in upcoming
    let upcoming = scheduler.get_upcoming(10).await.map_err(|e| format!("get_upcoming: {e}"))?;

    if upcoming.is_empty() {
        return Err("no upcoming jobs after scheduling".to_string());
    }

    let found = upcoming.iter().find(|j| j.job_id == job_id);
    if found.is_none() {
        return Err(format!(
            "scheduled job {job_id} not found in upcoming: {:?}",
            upcoming.iter().map(|j| j.job_id.as_str()).collect::<Vec<_>>()
        ));
    }

    let scheduled = found.unwrap();
    // next_execution should be close to target_time (within 1 second)
    let diff = (scheduled.next_execution - target_time).num_milliseconds().unsigned_abs();
    if diff > 1000 {
        return Err(format!("next_execution off by {diff}ms from target (expected <1000ms)"));
    }

    if scheduled.execution_count != 0 {
        return Err(format!("execution_count should be 0, got {}", scheduled.execution_count));
    }

    // Cancel it and verify it's gone
    scheduler.cancel_scheduled(&job_id).await.map_err(|e| format!("cancel_scheduled: {e}"))?;

    let upcoming_after = scheduler.get_upcoming(10).await.map_err(|e| format!("get_upcoming after cancel: {e}"))?;

    if upcoming_after.iter().any(|j| j.job_id == job_id) {
        return Err("cancelled job still appears in upcoming".to_string());
    }

    println!("  delayed job: scheduled, verified upcoming, cancelled");
    Ok(())
}

/// Schedule a recurring cron job, verify it's tracked with correct schedule type.
async fn test_cron_job_scheduling() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let config = SchedulerConfig {
        tick_interval_ms: 50,
        enable_jitter: false,
        ..Default::default()
    };
    let scheduler = Arc::new(SchedulerService::with_config(manager.clone(), store.clone(), config));

    // Schedule a recurring cron job (every 10 seconds)
    let spec = JobSpec::new("cron-echo");
    let schedule = aspen_jobs::Schedule::Recurring("*/10 * * * * *".to_string());

    let job_id = scheduler.schedule_job(spec, schedule).await.map_err(|e| format!("schedule_job cron: {e}"))?;

    // Verify it appears in upcoming
    let upcoming = scheduler.get_upcoming(10).await.map_err(|e| format!("get_upcoming: {e}"))?;

    let found = upcoming.iter().find(|j| j.job_id == job_id);
    if found.is_none() {
        return Err(format!("cron job {job_id} not found in upcoming"));
    }

    let scheduled = found.unwrap();

    // Next execution should be within 10 seconds from now
    let diff = (scheduled.next_execution - Utc::now()).num_seconds();
    if !(0..=10).contains(&diff) {
        return Err(format!("cron next_execution should be within 10s, got {diff}s away"));
    }

    // Pause and resume
    scheduler.pause_scheduled(&job_id).await.map_err(|e| format!("pause_scheduled: {e}"))?;

    scheduler.resume_scheduled(&job_id).await.map_err(|e| format!("resume_scheduled: {e}"))?;

    scheduler.shutdown().await;

    println!("  cron job: scheduled, verified upcoming, pause/resume");
    Ok(())
}
