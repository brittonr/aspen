//! Python bindings for Aspen distributed systems client.
//!
//! This module provides a Python API for the Aspen client SDK,
//! enabling Python applications to interact with Aspen clusters.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use aspen_client::AspenClient as RustAspenClient;
use aspen_client::AspenClientJobExt;
use aspen_client::AspenClientObservabilityExt;
use aspen_client::JobPriority as RustJobPriority;
use aspen_client::JobStatus as RustJobStatus;
use aspen_client::JobSubmitBuilder;
use aspen_client::SpanStatus as RustSpanStatus;
use pyo3::exceptions::PyException;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyDict;
use pyo3_asyncio_0_23 as pyo3_asyncio;
use pythonize::depythonize;
use pythonize::pythonize;
use serde_json::Value;
use tokio::sync::RwLock;

/// Python wrapper for AspenClient.
#[pyclass]
pub struct AspenClient {
    inner: Arc<RustAspenClient>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl AspenClient {
    /// Connect to an Aspen cluster.
    ///
    /// Args:
    ///     ticket (str): Cluster ticket string
    ///     timeout (float): Connection timeout in seconds (default: 10.0)
    ///     auth_token (str, optional): Authentication token
    ///
    /// Returns:
    ///     AspenClient: Connected client instance
    #[new]
    #[pyo3(signature = (ticket, timeout=10.0, auth_token=None))]
    fn new(ticket: String, timeout: f64, auth_token: Option<String>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyException::new_err(format!("Failed to create runtime: {}", e)))?;

        let timeout_duration = Duration::from_secs_f64(timeout);
        let token = auth_token.map(|t| aspen_client::AuthToken::from(t));

        let client = runtime
            .block_on(async { RustAspenClient::connect(&ticket, timeout_duration, token).await })
            .map_err(|e| PyException::new_err(format!("Connection failed: {}", e)))?;

        Ok(Self {
            inner: Arc::new(client),
            runtime: Arc::new(runtime),
        })
    }

    /// Write a key-value pair.
    ///
    /// Args:
    ///     key (str): The key to write
    ///     value (bytes): The value to write
    ///
    /// Returns:
    ///     None
    fn write<'py>(&self, py: Python<'py>, key: String, value: &PyBytes) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();
        let value_vec = value.as_bytes().to_vec();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            client
                .write(&key, value_vec)
                .await
                .map_err(|e| PyException::new_err(format!("Write failed: {}", e)))?;
            Ok(())
        })
    }

    /// Read a key-value pair.
    ///
    /// Args:
    ///     key (str): The key to read
    ///
    /// Returns:
    ///     bytes or None: The value if it exists
    fn read<'py>(&self, py: Python<'py>, key: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = client.read(&key).await.map_err(|e| PyException::new_err(format!("Read failed: {}", e)))?;

            Python::with_gil(|py| Ok(result.map(|data| PyBytes::new_bound(py, &data).into())))
        })
    }

    /// Delete a key.
    ///
    /// Args:
    ///     key (str): The key to delete
    ///
    /// Returns:
    ///     None
    fn delete<'py>(&self, py: Python<'py>, key: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.inner.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            client.delete(&key).await.map_err(|e| PyException::new_err(format!("Delete failed: {}", e)))?;
            Ok(())
        })
    }

    /// Get a job client for job management operations.
    ///
    /// Returns:
    ///     JobClient: Job management client
    fn jobs(&self) -> PyResult<JobClient> {
        Ok(JobClient {
            client: self.inner.clone(),
            runtime: self.runtime.clone(),
        })
    }

    /// Get an observability client for tracing and metrics.
    ///
    /// Returns:
    ///     ObservabilityClient: Observability client
    fn observability(&self) -> PyResult<ObservabilityClient> {
        Ok(ObservabilityClient {
            client: self.inner.clone(),
            runtime: self.runtime.clone(),
        })
    }
}

/// Python wrapper for job priority levels.
#[pyclass]
#[derive(Clone)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl From<JobPriority> for RustJobPriority {
    fn from(p: JobPriority) -> Self {
        match p {
            JobPriority::Low => RustJobPriority::Low,
            JobPriority::Normal => RustJobPriority::Normal,
            JobPriority::High => RustJobPriority::High,
            JobPriority::Critical => RustJobPriority::Critical,
        }
    }
}

/// Python wrapper for job status.
#[pyclass]
#[derive(Clone)]
pub enum JobStatus {
    Pending,
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
    Retrying,
    DeadLetter,
}

impl From<JobStatus> for RustJobStatus {
    fn from(s: JobStatus) -> Self {
        match s {
            JobStatus::Pending => RustJobStatus::Pending,
            JobStatus::Scheduled => RustJobStatus::Scheduled,
            JobStatus::Running => RustJobStatus::Running,
            JobStatus::Completed => RustJobStatus::Completed,
            JobStatus::Failed => RustJobStatus::Failed,
            JobStatus::Cancelled => RustJobStatus::Cancelled,
            JobStatus::Retrying => RustJobStatus::Retrying,
            JobStatus::DeadLetter => RustJobStatus::DeadLetter,
        }
    }
}

/// Python wrapper for job management.
#[pyclass]
pub struct JobClient {
    client: Arc<RustAspenClient>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl JobClient {
    /// Submit a job to the queue.
    ///
    /// Args:
    ///     job_type (str): Type of job
    ///     payload (dict): Job payload as JSON-compatible dict
    ///     priority (JobPriority, optional): Job priority
    ///     max_retries (int, optional): Maximum retry attempts
    ///     timeout (float, optional): Job timeout in seconds
    ///     dependencies (list[str], optional): Job IDs this job depends on
    ///     schedule (str, optional): Cron schedule expression
    ///     tags (list[str], optional): Job tags
    ///
    /// Returns:
    ///     str: Job ID
    #[pyo3(signature = (job_type, payload, priority=None, max_retries=None, timeout=None, dependencies=None, schedule=None, tags=None))]
    fn submit<'py>(
        &self,
        py: Python<'py>,
        job_type: String,
        payload: Bound<'py, PyDict>,
        priority: Option<JobPriority>,
        max_retries: Option<u32>,
        timeout: Option<f64>,
        dependencies: Option<Vec<String>>,
        schedule: Option<String>,
        tags: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Convert Python dict to serde_json::Value
        let payload_value: Value =
            depythonize(&payload).map_err(|e| PyValueError::new_err(format!("Invalid payload: {}", e)))?;

        let client = self.client.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let mut builder = JobSubmitBuilder::new(job_type, payload_value);

            if let Some(p) = priority {
                builder = builder.with_priority(p.into());
            }
            if let Some(r) = max_retries {
                builder = builder.with_max_retries(r);
            }
            if let Some(t) = timeout {
                builder = builder.with_timeout(Duration::from_secs_f64(t));
            }
            if let Some(deps) = dependencies {
                builder = builder.with_dependencies(deps);
            }
            if let Some(s) = schedule {
                builder = builder.with_cron_schedule(s);
            }
            if let Some(t) = tags {
                builder = builder.with_tags(t);
            }

            let job_id = client
                .jobs()
                .submit_job(builder)
                .await
                .map_err(|e| PyException::new_err(format!("Job submission failed: {}", e)))?;

            Ok(job_id)
        })
    }

    /// Get job details.
    ///
    /// Args:
    ///     job_id (str): Job ID
    ///
    /// Returns:
    ///     dict or None: Job details if found
    fn get<'py>(&self, py: Python<'py>, job_id: String) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let job = client
                .jobs()
                .get(&job_id)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to get job: {}", e)))?;

            Python::with_gil(|py| {
                if let Some(job_details) = job {
                    // Convert JobDetails to Python dict
                    let dict = PyDict::new_bound(py);
                    dict.set_item("job_id", job_details.job_id)?;
                    dict.set_item("job_type", job_details.job_type)?;
                    dict.set_item("status", job_details.status)?;
                    dict.set_item("priority", job_details.priority)?;
                    dict.set_item("progress", job_details.progress)?;
                    dict.set_item("tags", job_details.tags)?;
                    dict.set_item("submitted_at", job_details.submitted_at)?;
                    dict.set_item("attempts", job_details.attempts)?;

                    // Convert JSON values to Python
                    let payload_py = pythonize(py, &job_details.payload)?;
                    dict.set_item("payload", payload_py)?;

                    if let Some(result) = job_details.result {
                        let result_py = pythonize(py, &result)?;
                        dict.set_item("result", result_py)?;
                    }

                    Ok(Some(dict.into()))
                } else {
                    Ok(None)
                }
            })
        })
    }

    /// Cancel a job.
    ///
    /// Args:
    ///     job_id (str): Job ID
    ///     reason (str, optional): Cancellation reason
    ///
    /// Returns:
    ///     str: Previous job status
    fn cancel<'py>(&self, py: Python<'py>, job_id: String, reason: Option<String>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let status = client
                .jobs()
                .cancel(&job_id, reason)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to cancel job: {}", e)))?;

            Ok(status)
        })
    }

    /// Wait for a job to complete.
    ///
    /// Args:
    ///     job_id (str): Job ID
    ///     poll_interval (float): Poll interval in seconds (default: 1.0)
    ///     timeout (float, optional): Timeout in seconds
    ///
    /// Returns:
    ///     dict: Job details when completed
    #[pyo3(signature = (job_id, poll_interval=1.0, timeout=None))]
    fn wait_for_completion<'py>(
        &self,
        py: Python<'py>,
        job_id: String,
        poll_interval: f64,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let poll_duration = Duration::from_secs_f64(poll_interval);
        let timeout_duration = timeout.map(Duration::from_secs_f64);

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let job = client
                .jobs()
                .wait_for_completion(&job_id, poll_duration, timeout_duration)
                .await
                .map_err(|e| PyException::new_err(format!("Wait failed: {}", e)))?;

            Python::with_gil(|py| {
                let dict = PyDict::new_bound(py);
                dict.set_item("job_id", job.job_id)?;
                dict.set_item("status", job.status)?;

                let payload_py = pythonize(py, &job.payload)?;
                dict.set_item("payload", payload_py)?;

                if let Some(result) = job.result {
                    let result_py = pythonize(py, &result)?;
                    dict.set_item("result", result_py)?;
                }

                Ok(dict.into())
            })
        })
    }
}

/// Python wrapper for observability features.
#[pyclass]
pub struct ObservabilityClient {
    client: Arc<RustAspenClient>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl ObservabilityClient {
    /// Start a new span for tracing.
    ///
    /// Args:
    ///     operation (str): Operation name
    ///     parent_trace_id (str, optional): Parent trace ID
    ///
    /// Returns:
    ///     Span: New span object
    fn start_span(&self, operation: String, parent_trace_id: Option<String>) -> PyResult<Span> {
        Ok(Span {
            operation,
            trace_id: format!("{:032x}", rand::random::<u128>()),
            span_id: format!("{:016x}", rand::random::<u64>()),
            parent_id: parent_trace_id,
            attributes: HashMap::new(),
            start_time: std::time::Instant::now(),
        })
    }

    /// Record a metric value.
    ///
    /// Args:
    ///     name (str): Metric name
    ///     value (float): Metric value
    ///     metric_type (str): Type of metric ("counter", "gauge", "histogram")
    fn record_metric(&self, name: String, value: f64, metric_type: String) -> PyResult<()> {
        // In a real implementation, this would send metrics to the server
        println!("Recording {} metric '{}': {}", metric_type, name, value);
        Ok(())
    }
}

/// Python wrapper for a trace span.
#[pyclass]
pub struct Span {
    #[pyo3(get)]
    operation: String,
    #[pyo3(get)]
    trace_id: String,
    #[pyo3(get)]
    span_id: String,
    #[pyo3(get)]
    parent_id: Option<String>,
    attributes: HashMap<String, String>,
    start_time: std::time::Instant,
}

#[pymethods]
impl Span {
    /// Add an attribute to the span.
    fn set_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    /// End the span and return duration in seconds.
    fn end(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Get the W3C traceparent header.
    fn traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id, self.span_id)
    }
}

/// Python module initialization.
#[pymodule]
fn aspen_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AspenClient>()?;
    m.add_class::<JobClient>()?;
    m.add_class::<JobPriority>()?;
    m.add_class::<JobStatus>()?;
    m.add_class::<ObservabilityClient>()?;
    m.add_class::<Span>()?;

    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
