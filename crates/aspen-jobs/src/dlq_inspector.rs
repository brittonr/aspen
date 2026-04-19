//! Dead Letter Queue inspection and analysis tools.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_traits::KeyValueStore;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::manager::JobManager;
use crate::types::Priority;

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "DLQ inspection reports wall-clock ages and export timestamps"
)]
fn current_time_utc() -> DateTime<Utc> {
    Utc::now()
}

fn expanded_limit(limit: u32) -> u32 {
    limit.saturating_mul(2)
}

fn limit_to_usize(limit: u32) -> usize {
    usize::try_from(limit).unwrap_or(usize::MAX)
}

/// DLQ inspector for analyzing failed jobs.
pub struct DlqInspector<S: KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
}

impl<S: KeyValueStore + ?Sized + 'static> DlqInspector<S> {
    /// Create a new DLQ inspector.
    pub fn new(manager: Arc<JobManager<S>>) -> Self {
        Self { manager }
    }

    /// Get detailed analysis of DLQ contents.
    pub async fn analyze(&self, limit: u32) -> Result<DlqAnalysis> {
        let jobs = self.manager.get_dlq_jobs(None, limit).await?;
        let current_time = current_time_utc();

        let mut analysis = DlqAnalysis {
            total_jobs: u64::try_from(jobs.len()).unwrap_or(u64::MAX),
            ..Default::default()
        };

        for job in &jobs {
            *analysis.by_priority.entry(job.spec.config.priority).or_insert(0) += 1;
            *analysis.by_job_type.entry(job.spec.job_type.clone()).or_insert(0) += 1;

            if let Some(dlq_meta) = job.dlq_metadata.as_ref() {
                *analysis.by_reason.entry(format!("{:?}", dlq_meta.reason)).or_insert(0) += 1;

                let age = current_time - dlq_meta.entered_at;
                if analysis.oldest_entry.is_none_or(|oldest| dlq_meta.entered_at < oldest) {
                    analysis.oldest_entry = Some(dlq_meta.entered_at);
                }
                if analysis.newest_entry.is_none_or(|newest| dlq_meta.entered_at > newest) {
                    analysis.newest_entry = Some(dlq_meta.entered_at);
                }

                analysis.total_age_seconds =
                    analysis.total_age_seconds.saturating_add(u64::try_from(age.num_seconds()).unwrap_or(0));

                if dlq_meta.redrive_count > 0 {
                    analysis.redriven_jobs += 1;
                    analysis.total_redrive_attempts += u64::from(dlq_meta.redrive_count);
                }

                let error = dlq_meta.final_error.clone();
                *analysis.error_patterns.entry(error).or_insert(0) += 1;
            }
        }

        analysis.avg_age_seconds = analysis.total_age_seconds.checked_div(analysis.total_jobs).unwrap_or(0);
        if analysis.redriven_jobs > 0 {
            analysis.avg_redrive_attempts = analysis.total_redrive_attempts as f64 / analysis.redriven_jobs as f64;
        }

        analysis.top_errors =
            analysis.error_patterns.iter().map(|(error, count)| (error.clone(), *count)).collect::<Vec<_>>();
        analysis.top_errors.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        analysis.top_errors.truncate(10);
        debug_assert!(analysis.top_errors.len() <= 10);

        Ok(analysis)
    }

    /// Find jobs with specific error patterns.
    pub async fn find_by_error_pattern(&self, pattern: &str, limit: u32) -> Result<Vec<Job>> {
        let jobs = self.manager.get_dlq_jobs(None, expanded_limit(limit)).await?;

        let matching_jobs: Vec<Job> = jobs
            .into_iter()
            .filter(|job| job.dlq_metadata.as_ref().map(|meta| meta.final_error.contains(pattern)).unwrap_or(false))
            .take(limit_to_usize(limit))
            .collect();

        info!(pattern, count = matching_jobs.len(), "found jobs with error pattern");

        Ok(matching_jobs)
    }

    /// Find jobs that have been in DLQ longer than specified duration.
    pub async fn find_stale_jobs(&self, age_threshold: Duration, limit: u32) -> Result<Vec<Job>> {
        let jobs = self.manager.get_dlq_jobs(None, expanded_limit(limit)).await?;
        let now = current_time_utc();

        let stale_jobs: Vec<Job> = jobs
            .into_iter()
            .filter(|job| job.dlq_metadata.as_ref().map(|meta| now - meta.entered_at > age_threshold).unwrap_or(false))
            .take(limit_to_usize(limit))
            .collect();

        info!(threshold_hours = age_threshold.num_hours(), count = stale_jobs.len(), "found stale DLQ jobs");

        Ok(stale_jobs)
    }

    /// Get jobs grouped by failure reason.
    pub async fn group_by_reason(&self, limit: u32) -> Result<HashMap<String, Vec<JobId>>> {
        let jobs = self.manager.get_dlq_jobs(None, limit).await?;
        let mut grouped = HashMap::new();

        for job in jobs {
            if let Some(dlq_meta) = job.dlq_metadata.as_ref() {
                let reason = format!("{:?}", dlq_meta.reason);
                grouped.entry(reason).or_insert_with(Vec::new).push(job.id);
            }
        }

        Ok(grouped)
    }

    /// Get jobs that have been redriven multiple times.
    pub async fn find_problematic_jobs(&self, min_redrive_count: u32, limit: u32) -> Result<Vec<Job>> {
        let jobs = self.manager.get_dlq_jobs(None, expanded_limit(limit)).await?;

        let problematic_jobs: Vec<Job> = jobs
            .into_iter()
            .filter(|job| {
                job.dlq_metadata.as_ref().map(|meta| meta.redrive_count >= min_redrive_count).unwrap_or(false)
            })
            .take(limit_to_usize(limit))
            .collect();

        info!(
            min_redrive_count,
            count = problematic_jobs.len(),
            "found problematic jobs with multiple redrive attempts"
        );

        Ok(problematic_jobs)
    }

    /// Export DLQ contents to JSON.
    pub async fn export_to_json(&self, limit: u32) -> Result<String> {
        let jobs = self.manager.get_dlq_jobs(None, limit).await?;
        let total_jobs = u64::try_from(jobs.len()).unwrap_or(u64::MAX);
        debug_assert!(total_jobs >= u64::try_from(jobs.len()).unwrap_or(u64::MAX));
        debug_assert!(jobs.len() <= limit_to_usize(limit));

        let json = serde_json::to_string_pretty(&DlqExport {
            exported_at: current_time_utc(),
            total_jobs,
            jobs: jobs.into_iter().map(DlqExportEntry::from_job).collect(),
        })
        .map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        Ok(json)
    }

    /// Get recommendations for DLQ management.
    pub async fn get_recommendations(&self) -> Result<Vec<DlqRecommendation>> {
        let analysis = self.analyze(1000).await?;
        let current_time = current_time_utc();
        let mut recommendations = Vec::new();

        if let Some(oldest) = analysis.oldest_entry {
            let age = current_time - oldest;
            if age > Duration::days(7) {
                recommendations.push(DlqRecommendation {
                    severity: RecommendationSeverity::Warning,
                    category: "Stale Jobs".to_string(),
                    message: format!(
                        "Found jobs in DLQ for over {} days. Consider purging or investigating.",
                        age.num_days()
                    ),
                    action: "Run purge for jobs older than 7 days or investigate root cause".to_string(),
                });
            }
        }

        let has_redrives = analysis.redriven_jobs > 0;
        let has_high_avg_attempts = analysis.avg_redrive_attempts > 3.0;
        if has_redrives && has_high_avg_attempts {
            recommendations.push(DlqRecommendation {
                severity: RecommendationSeverity::High,
                category: "Redrive Failures".to_string(),
                message: format!(
                    "Jobs are failing after multiple redrive attempts (avg: {:.1})",
                    analysis.avg_redrive_attempts
                ),
                action: "Review job logic or consider manual intervention".to_string(),
            });
        }

        if !analysis.top_errors.is_empty() {
            let (top_error, count) = &analysis.top_errors[0];
            if *count > 10 {
                recommendations.push(DlqRecommendation {
                    severity: RecommendationSeverity::High,
                    category: "Error Pattern".to_string(),
                    message: format!(
                        "Common error affecting {} jobs: {}",
                        count,
                        top_error.chars().take(100).collect::<String>()
                    ),
                    action: "Fix the root cause of this error pattern".to_string(),
                });
            }
        }

        let max_by_type = analysis.by_job_type.values().max().copied().unwrap_or(0);
        let has_jobs = analysis.total_jobs > 0;
        let type_majority_threshold = analysis.total_jobs.checked_div(2).unwrap_or(0);
        if has_jobs && max_by_type > type_majority_threshold {
            let problem_type = analysis
                .by_job_type
                .iter()
                .find(|&(_, &count)| count == max_by_type)
                .map(|(job_type, _)| job_type.clone())
                .unwrap_or_default();
            let job_share_percent =
                max_by_type.checked_mul(100).and_then(|scaled| scaled.checked_div(analysis.total_jobs)).unwrap_or(0);

            recommendations.push(DlqRecommendation {
                severity: RecommendationSeverity::Warning,
                category: "Job Type Issue".to_string(),
                message: format!("Job type '{}' represents {}% of DLQ jobs", problem_type, job_share_percent),
                action: format!("Review implementation of '{}' job handler", problem_type),
            });
        }

        Ok(recommendations)
    }
}

/// Analysis results for DLQ contents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DlqAnalysis {
    /// Total number of jobs in DLQ.
    pub total_jobs: u64,
    /// Jobs grouped by priority.
    pub by_priority: HashMap<Priority, u64>,
    /// Jobs grouped by job type.
    pub by_job_type: HashMap<String, u64>,
    /// Jobs grouped by DLQ reason.
    pub by_reason: HashMap<String, u64>,
    /// Error message patterns and their counts.
    pub error_patterns: HashMap<String, u64>,
    /// Top 10 most common errors.
    pub top_errors: Vec<(String, u64)>,
    /// Oldest DLQ entry time.
    pub oldest_entry: Option<DateTime<Utc>>,
    /// Newest DLQ entry time.
    pub newest_entry: Option<DateTime<Utc>>,
    /// Average age of DLQ entries in seconds.
    pub avg_age_seconds: u64,
    /// Total age of all entries in seconds.
    pub total_age_seconds: u64,
    /// Number of jobs that have been redriven.
    pub redriven_jobs: u64,
    /// Total redrive attempts across all jobs.
    pub total_redrive_attempts: u64,
    /// Average redrive attempts per redriven job.
    pub avg_redrive_attempts: f64,
}

/// DLQ export format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqExport {
    /// Export timestamp.
    pub exported_at: DateTime<Utc>,
    /// Total jobs exported.
    pub total_jobs: u64,
    /// Job entries.
    pub jobs: Vec<DlqExportEntry>,
}

/// Individual DLQ export entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqExportEntry {
    /// Job ID.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Priority.
    pub priority: Priority,
    /// DLQ entry time.
    pub entered_dlq_at: Option<DateTime<Utc>>,
    /// DLQ reason.
    pub reason: Option<String>,
    /// Final error message.
    pub final_error: Option<String>,
    /// Number of attempts before DLQ.
    pub attempts: u32,
    /// Redrive count.
    pub redrive_count: u32,
    /// Job payload.
    pub payload: serde_json::Value,
}

impl DlqExportEntry {
    /// Create from a Job.
    pub fn from_job(job: Job) -> Self {
        let dlq_meta = job.dlq_metadata.clone();
        Self {
            job_id: job.id.to_string(),
            job_type: job.spec.job_type,
            priority: job.spec.config.priority,
            entered_dlq_at: dlq_meta.as_ref().map(|meta| meta.entered_at),
            reason: dlq_meta.as_ref().map(|meta| format!("{:?}", meta.reason)),
            final_error: dlq_meta.as_ref().map(|meta| meta.final_error.clone()),
            attempts: job.attempts,
            redrive_count: dlq_meta.as_ref().map(|meta| meta.redrive_count).unwrap_or(0),
            payload: job.spec.payload,
        }
    }
}

/// DLQ management recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqRecommendation {
    /// Severity of the recommendation.
    pub severity: RecommendationSeverity,
    /// Category of the issue.
    pub category: String,
    /// Description of the issue.
    pub message: String,
    /// Recommended action.
    pub action: String,
}

/// Severity level for recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    /// Low priority recommendation.
    Low,
    /// Medium priority recommendation.
    Warning,
    /// High priority recommendation.
    High,
}
