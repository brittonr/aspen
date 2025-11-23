//! Job list view model

use askama::Template;
use crate::domain::job_lifecycle::EnrichedJob;
use crate::domain::{format_duration, format_time_ago, JobStatus};

/// Individual job row for display
#[derive(Debug, Clone)]
pub struct JobRow {
    pub job_id: String,
    pub status_class: String,
    pub status_text: String,
    pub url: String,
    pub duration_text: String,
    pub duration_class: String,
    pub time_ago: String,
    pub node_id: String,
}

impl From<EnrichedJob> for JobRow {
    fn from(job: EnrichedJob) -> Self {
        let status_class = match job.status {
            JobStatus::Pending => "status-pending",
            JobStatus::Claimed => "status-claimed",
            JobStatus::InProgress => "status-in-progress",
            JobStatus::Completed => "status-completed",
            JobStatus::Failed => "status-failed",
        };

        let status_text = format!("{:?}", job.status);

        let node_id = job
            .claimed_by
            .map(|id| format!("<span class=\"node-id\">{}</span>", &id[..std::cmp::min(16, id.len())]))
            .unwrap_or_else(|| "-".to_string());

        let (duration_text, duration_class) = format_duration(job.duration_seconds);
        let time_ago = format_time_ago(job.time_ago_seconds);

        Self {
            job_id: job.job_id,
            status_class: status_class.to_string(),
            status_text,
            url: job.url,
            duration_text,
            duration_class: duration_class.to_string(),
            time_ago,
            node_id,
        }
    }
}

/// View model for job list table
#[derive(Template)]
#[template(path = "partials/job_list.html")]
pub struct JobListView {
    pub jobs: Vec<JobRow>,
}

impl JobListView {
    pub fn new(enriched_jobs: Vec<EnrichedJob>) -> Self {
        Self {
            jobs: enriched_jobs.into_iter().map(JobRow::from).collect(),
        }
    }

    pub fn empty() -> Self {
        Self { jobs: Vec::new() }
    }
}
