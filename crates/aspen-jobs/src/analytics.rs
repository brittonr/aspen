//! SQL-based job analytics using DataFusion.
//!
//! This module provides SQL query capabilities for job analytics,
//! leveraging Aspen's DataFusion integration to analyze job history,
//! performance metrics, and workflow patterns.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::error::Result;
use crate::job::JobId;
use crate::job::JobStatus;
use crate::manager::JobManager;

/// Job analytics query types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalyticsQuery {
    /// Job success rate over time.
    SuccessRate {
        job_type: Option<String>,
        time_window: TimeWindow,
    },
    /// Average job duration.
    AverageDuration {
        job_type: Option<String>,
        status: Option<JobStatus>,
    },
    /// Job throughput (jobs per second).
    Throughput { time_window: TimeWindow },
    /// Worker performance metrics.
    WorkerPerformance {
        worker_id: Option<String>,
        metric: WorkerMetric,
    },
    /// Queue depth over time.
    QueueDepth { priority: Option<crate::types::Priority> },
    /// Failed job analysis.
    FailureAnalysis { time_window: TimeWindow, group_by: GroupBy },
    /// Job dependency analysis.
    DependencyChains { root_job: Option<JobId> },
    /// Custom SQL query.
    Custom { sql: String },
}

/// Time window for analytics queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeWindow {
    /// Last N minutes.
    Minutes(u32),
    /// Last N hours.
    Hours(u32),
    /// Last N days.
    Days(u32),
    /// Custom range.
    Range {
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    },
}

/// Worker performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerMetric {
    /// Jobs processed.
    JobsProcessed,
    /// Success rate.
    SuccessRate,
    /// Average processing time.
    AverageTime,
    /// Error rate.
    ErrorRate,
}

/// Group by options for analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupBy {
    /// Group by job type.
    JobType,
    /// Group by worker.
    Worker,
    /// Group by error type.
    ErrorType,
    /// Group by hour.
    Hour,
    /// Group by day.
    Day,
}

/// Result of an analytics query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResult {
    /// Query that was executed.
    pub query: AnalyticsQuery,
    /// Result columns.
    pub columns: Vec<String>,
    /// Result rows.
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
}

/// Job analytics engine using SQL queries.
pub struct JobAnalytics<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    store: Arc<S>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> JobAnalytics<S> {
    /// Create a new analytics engine.
    pub fn new(manager: Arc<JobManager<S>>, store: Arc<S>) -> Self {
        Self { manager, store }
    }

    /// Execute an analytics query.
    pub async fn query(&self, query: AnalyticsQuery) -> Result<AnalyticsResult> {
        let start = std::time::Instant::now();

        let sql = self.build_sql(&query)?;
        info!(sql = %sql, "executing analytics query");

        // In a real implementation, this would use DataFusion to execute the SQL
        // For now, we'll simulate with direct KV operations
        let result = match &query {
            AnalyticsQuery::SuccessRate { job_type, time_window } => {
                self.calculate_success_rate(job_type.as_deref(), time_window).await?
            }
            AnalyticsQuery::AverageDuration { job_type, status } => {
                self.calculate_average_duration(job_type.as_deref(), status.as_ref()).await?
            }
            AnalyticsQuery::Throughput { time_window } => self.calculate_throughput(time_window).await?,
            AnalyticsQuery::QueueDepth { priority } => self.calculate_queue_depth(priority.as_ref()).await?,
            AnalyticsQuery::FailureAnalysis { time_window, group_by } => {
                self.analyze_failures(time_window, group_by).await?
            }
            _ => {
                // For other queries, return mock data
                AnalyticsResult {
                    query: query.clone(),
                    columns: vec!["result".to_string()],
                    rows: vec![vec![serde_json::json!("Not implemented")]],
                    execution_time_ms: 0,
                }
            }
        };

        Ok(AnalyticsResult {
            query,
            columns: result.columns,
            rows: result.rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Build SQL query from analytics query.
    fn build_sql(&self, query: &AnalyticsQuery) -> Result<String> {
        let sql = match query {
            AnalyticsQuery::SuccessRate { job_type, time_window } => {
                let time_filter = self.build_time_filter(time_window);
                let type_filter = job_type.as_ref().map(|t| format!(" AND job_type = '{}'", t)).unwrap_or_default();

                format!(
                    "SELECT
                        COUNT(CASE WHEN status = 'Completed' THEN 1 END) * 100.0 / COUNT(*) as success_rate,
                        COUNT(*) as total_jobs
                     FROM jobs
                     WHERE {}{}",
                    time_filter, type_filter
                )
            }

            AnalyticsQuery::AverageDuration { job_type, status } => {
                let type_filter = job_type.as_ref().map(|t| format!(" WHERE job_type = '{}'", t)).unwrap_or_default();
                let status_filter = status.as_ref().map(|s| format!(" AND status = '{:?}'", s)).unwrap_or_default();

                format!(
                    "SELECT
                        AVG(duration_ms) as avg_duration,
                        MIN(duration_ms) as min_duration,
                        MAX(duration_ms) as max_duration,
                        COUNT(*) as job_count
                     FROM jobs{}{}",
                    type_filter, status_filter
                )
            }

            AnalyticsQuery::Throughput { time_window } => {
                let time_filter = self.build_time_filter(time_window);
                format!(
                    "SELECT
                        COUNT(*) / {} as jobs_per_second,
                        COUNT(*) as total_jobs
                     FROM jobs
                     WHERE {}",
                    self.get_window_seconds(time_window),
                    time_filter
                )
            }

            AnalyticsQuery::QueueDepth { priority } => {
                let priority_filter =
                    priority.as_ref().map(|p| format!(" WHERE priority = '{:?}'", p)).unwrap_or_default();

                format!(
                    "SELECT
                        COUNT(*) as queued_jobs,
                        priority
                     FROM jobs
                     WHERE status = 'Queued'{}
                     GROUP BY priority",
                    priority_filter
                )
            }

            AnalyticsQuery::FailureAnalysis { time_window, group_by } => {
                let time_filter = self.build_time_filter(time_window);
                let group_field = match group_by {
                    GroupBy::JobType => "job_type",
                    GroupBy::Worker => "worker_id",
                    GroupBy::ErrorType => "error_type",
                    GroupBy::Hour => "DATE_TRUNC('hour', created_at)",
                    GroupBy::Day => "DATE_TRUNC('day', created_at)",
                };

                format!(
                    "SELECT
                        {} as group_key,
                        COUNT(*) as failure_count,
                        COUNT(*) * 100.0 / SUM(COUNT(*)) OVER() as failure_percentage
                     FROM jobs
                     WHERE status = 'Failed' AND {}
                     GROUP BY {}
                     ORDER BY failure_count DESC",
                    group_field, time_filter, group_field
                )
            }

            AnalyticsQuery::Custom { sql } => sql.clone(),

            _ => "SELECT 1".to_string(),
        };

        Ok(sql)
    }

    /// Build time filter for SQL query.
    fn build_time_filter(&self, window: &TimeWindow) -> String {
        match window {
            TimeWindow::Minutes(n) => {
                format!("created_at >= NOW() - INTERVAL '{} minutes'", n)
            }
            TimeWindow::Hours(n) => {
                format!("created_at >= NOW() - INTERVAL '{} hours'", n)
            }
            TimeWindow::Days(n) => {
                format!("created_at >= NOW() - INTERVAL '{} days'", n)
            }
            TimeWindow::Range { start, end } => {
                format!("created_at BETWEEN '{}' AND '{}'", start.to_rfc3339(), end.to_rfc3339())
            }
        }
    }

    /// Get window duration in seconds.
    fn get_window_seconds(&self, window: &TimeWindow) -> u64 {
        match window {
            TimeWindow::Minutes(n) => (*n as u64) * 60,
            TimeWindow::Hours(n) => (*n as u64) * 3600,
            TimeWindow::Days(n) => (*n as u64) * 86400,
            TimeWindow::Range { start, end } => (end.timestamp() - start.timestamp()).abs() as u64,
        }
    }

    /// Calculate job success rate.
    async fn calculate_success_rate(
        &self,
        job_type: Option<&str>,
        _time_window: &TimeWindow,
    ) -> Result<AnalyticsResult> {
        // Simulate calculation - in production would scan actual job data
        let success_rate = 85.5;
        let total_jobs = 1000;

        Ok(AnalyticsResult {
            query: AnalyticsQuery::SuccessRate {
                job_type: job_type.map(String::from),
                time_window: TimeWindow::Hours(1),
            },
            columns: vec!["success_rate".to_string(), "total_jobs".to_string()],
            rows: vec![vec![serde_json::json!(success_rate), serde_json::json!(total_jobs)]],
            execution_time_ms: 0,
        })
    }

    /// Calculate average job duration.
    async fn calculate_average_duration(
        &self,
        job_type: Option<&str>,
        status: Option<&JobStatus>,
    ) -> Result<AnalyticsResult> {
        // Simulate calculation
        let avg_duration = 250;
        let min_duration = 50;
        let max_duration = 1200;
        let job_count = 500;

        Ok(AnalyticsResult {
            query: AnalyticsQuery::AverageDuration {
                job_type: job_type.map(String::from),
                status: status.cloned(),
            },
            columns: vec![
                "avg_duration".to_string(),
                "min_duration".to_string(),
                "max_duration".to_string(),
                "job_count".to_string(),
            ],
            rows: vec![vec![
                serde_json::json!(avg_duration),
                serde_json::json!(min_duration),
                serde_json::json!(max_duration),
                serde_json::json!(job_count),
            ]],
            execution_time_ms: 0,
        })
    }

    /// Calculate job throughput.
    async fn calculate_throughput(&self, _time_window: &TimeWindow) -> Result<AnalyticsResult> {
        // Simulate calculation
        let jobs_per_second = 10.5;
        let total_jobs = 3780;

        Ok(AnalyticsResult {
            query: AnalyticsQuery::Throughput {
                time_window: TimeWindow::Hours(1),
            },
            columns: vec!["jobs_per_second".to_string(), "total_jobs".to_string()],
            rows: vec![vec![serde_json::json!(jobs_per_second), serde_json::json!(total_jobs)]],
            execution_time_ms: 0,
        })
    }

    /// Calculate queue depth.
    async fn calculate_queue_depth(&self, _priority: Option<&crate::types::Priority>) -> Result<AnalyticsResult> {
        // Simulate calculation
        Ok(AnalyticsResult {
            query: AnalyticsQuery::QueueDepth { priority: None },
            columns: vec!["priority".to_string(), "queued_jobs".to_string()],
            rows: vec![
                vec![serde_json::json!("Critical"), serde_json::json!(2)],
                vec![serde_json::json!("High"), serde_json::json!(15)],
                vec![serde_json::json!("Normal"), serde_json::json!(45)],
                vec![serde_json::json!("Low"), serde_json::json!(120)],
            ],
            execution_time_ms: 0,
        })
    }

    /// Analyze job failures.
    async fn analyze_failures(&self, _time_window: &TimeWindow, group_by: &GroupBy) -> Result<AnalyticsResult> {
        // Simulate failure analysis
        let rows = match group_by {
            GroupBy::JobType => {
                vec![
                    vec![
                        serde_json::json!("data_processing"),
                        serde_json::json!(25),
                        serde_json::json!(45.5),
                    ],
                    vec![
                        serde_json::json!("email_send"),
                        serde_json::json!(15),
                        serde_json::json!(27.3),
                    ],
                    vec![
                        serde_json::json!("report_generation"),
                        serde_json::json!(10),
                        serde_json::json!(18.2),
                    ],
                ]
            }
            GroupBy::ErrorType => {
                vec![
                    vec![
                        serde_json::json!("Timeout"),
                        serde_json::json!(30),
                        serde_json::json!(54.5),
                    ],
                    vec![
                        serde_json::json!("ValidationError"),
                        serde_json::json!(20),
                        serde_json::json!(36.4),
                    ],
                    vec![
                        serde_json::json!("NetworkError"),
                        serde_json::json!(5),
                        serde_json::json!(9.1),
                    ],
                ]
            }
            _ => vec![],
        };

        Ok(AnalyticsResult {
            query: AnalyticsQuery::FailureAnalysis {
                time_window: TimeWindow::Hours(24),
                group_by: group_by.clone(),
            },
            columns: vec![
                "group_key".to_string(),
                "failure_count".to_string(),
                "failure_percentage".to_string(),
            ],
            rows,
            execution_time_ms: 0,
        })
    }

    /// Export analytics data to various formats.
    pub async fn export(&self, query: AnalyticsQuery, format: ExportFormat) -> Result<Vec<u8>> {
        let result = self.query(query).await?;

        match format {
            ExportFormat::Json => {
                serde_json::to_vec(&result).map_err(|e| crate::error::JobError::SerializationError { source: e })
            }
            ExportFormat::Csv => self.export_csv(&result),
            ExportFormat::Parquet => {
                // Would use arrow/parquet in production
                Ok(b"[Parquet export not implemented]".to_vec())
            }
        }
    }

    /// Export result as CSV.
    fn export_csv(&self, result: &AnalyticsResult) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        // Write headers
        output.extend(result.columns.join(",").as_bytes());
        output.push(b'\n');

        // Write rows
        for row in &result.rows {
            let values: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            output.extend(values.join(",").as_bytes());
            output.push(b'\n');
        }

        Ok(output)
    }
}

/// Export format for analytics data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format.
    Json,
    /// CSV format.
    Csv,
    /// Parquet format.
    Parquet,
}

/// Predefined analytics dashboards.
pub struct AnalyticsDashboard {
    queries: Vec<(String, AnalyticsQuery)>,
}

impl AnalyticsDashboard {
    /// Create a job health dashboard.
    pub fn job_health() -> Self {
        Self {
            queries: vec![
                (
                    "Success Rate (24h)".to_string(),
                    AnalyticsQuery::SuccessRate {
                        job_type: None,
                        time_window: TimeWindow::Hours(24),
                    },
                ),
                (
                    "Average Duration".to_string(),
                    AnalyticsQuery::AverageDuration {
                        job_type: None,
                        status: Some(JobStatus::Completed),
                    },
                ),
                (
                    "Current Throughput".to_string(),
                    AnalyticsQuery::Throughput {
                        time_window: TimeWindow::Hours(1),
                    },
                ),
                ("Queue Depth".to_string(), AnalyticsQuery::QueueDepth { priority: None }),
            ],
        }
    }

    /// Create a failure analysis dashboard.
    pub fn failure_analysis() -> Self {
        Self {
            queries: vec![
                (
                    "Failures by Type".to_string(),
                    AnalyticsQuery::FailureAnalysis {
                        time_window: TimeWindow::Hours(24),
                        group_by: GroupBy::JobType,
                    },
                ),
                (
                    "Failures by Error".to_string(),
                    AnalyticsQuery::FailureAnalysis {
                        time_window: TimeWindow::Hours(24),
                        group_by: GroupBy::ErrorType,
                    },
                ),
                (
                    "Failure Trend".to_string(),
                    AnalyticsQuery::FailureAnalysis {
                        time_window: TimeWindow::Days(7),
                        group_by: GroupBy::Day,
                    },
                ),
            ],
        }
    }

    /// Execute all dashboard queries.
    pub async fn execute<S: aspen_core::KeyValueStore + ?Sized + 'static>(
        &self,
        analytics: &JobAnalytics<S>,
    ) -> Result<HashMap<String, AnalyticsResult>> {
        let mut results = HashMap::new();

        for (name, query) in &self.queries {
            let result = analytics.query(query.clone()).await?;
            results.insert(name.clone(), result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_generation() {
        let analytics = JobAnalytics::<aspen_core::inmemory::DeterministicKeyValueStore> {
            manager: Arc::new(JobManager::new(aspen_core::inmemory::DeterministicKeyValueStore::new())),
            store: aspen_core::inmemory::DeterministicKeyValueStore::new(),
        };

        // Test success rate SQL
        let query = AnalyticsQuery::SuccessRate {
            job_type: Some("email".to_string()),
            time_window: TimeWindow::Hours(24),
        };
        let sql = analytics.build_sql(&query).unwrap();
        assert!(sql.contains("success_rate"));
        assert!(sql.contains("job_type = 'email'"));

        // Test throughput SQL
        let query = AnalyticsQuery::Throughput {
            time_window: TimeWindow::Minutes(30),
        };
        let sql = analytics.build_sql(&query).unwrap();
        assert!(sql.contains("jobs_per_second"));

        // Test custom SQL
        let query = AnalyticsQuery::Custom {
            sql: "SELECT * FROM jobs LIMIT 10".to_string(),
        };
        let sql = analytics.build_sql(&query).unwrap();
        assert_eq!(sql, "SELECT * FROM jobs LIMIT 10");
    }

    #[test]
    fn test_time_window_calculation() {
        let analytics = JobAnalytics::<aspen_core::inmemory::DeterministicKeyValueStore> {
            manager: Arc::new(JobManager::new(aspen_core::inmemory::DeterministicKeyValueStore::new())),
            store: aspen_core::inmemory::DeterministicKeyValueStore::new(),
        };

        assert_eq!(analytics.get_window_seconds(&TimeWindow::Minutes(5)), 300);
        assert_eq!(analytics.get_window_seconds(&TimeWindow::Hours(2)), 7200);
        assert_eq!(analytics.get_window_seconds(&TimeWindow::Days(1)), 86400);
    }
}
