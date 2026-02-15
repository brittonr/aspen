//! Performance profiling types and bottleneck detection.

use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;

use crate::job::JobId;

/// Performance profile for a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProfile {
    /// Job ID.
    pub job_id: JobId,
    /// Total execution time.
    pub total_time: Duration,
    /// Time breakdown by phase.
    pub phases: Vec<ProfilePhase>,
    /// Resource usage samples.
    pub resource_samples: Vec<ResourceSample>,
    /// Bottlenecks detected.
    pub bottlenecks: Vec<Bottleneck>,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

/// Phase in job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePhase {
    /// Phase name.
    pub name: String,
    /// Start time (relative to job start).
    pub start_offset: Duration,
    /// Duration.
    pub duration: Duration,
    /// CPU time.
    pub cpu_time: Duration,
    /// Wait time.
    pub wait_time: Duration,
    /// I/O time.
    pub io_time: Duration,
}

/// Resource usage sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSample {
    /// Sample time (relative to job start).
    pub time_offset: Duration,
    /// CPU usage (percentage).
    pub cpu_percent: f32,
    /// Memory usage (bytes).
    pub memory_bytes: u64,
    /// Disk I/O (bytes/sec).
    pub disk_io_rate: u64,
    /// Network I/O (bytes/sec).
    pub network_io_rate: u64,
}

/// Detected performance bottleneck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Bottleneck type.
    pub bottleneck_type: BottleneckType,
    /// Severity (0-100).
    pub severity: u8,
    /// Description.
    pub description: String,
    /// Affected phase.
    pub phase: Option<String>,
    /// Duration of bottleneck.
    pub duration: Duration,
}

/// Types of performance bottlenecks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// CPU-bound.
    CpuBound,
    /// Memory pressure.
    MemoryPressure,
    /// I/O wait.
    IoWait,
    /// Network latency.
    NetworkLatency,
    /// Lock contention.
    LockContention,
    /// Queue backup.
    QueueBackup,
}

/// Builder for job profiles.
pub(crate) struct JobProfileBuilder {
    pub(crate) job_id: JobId,
    pub(crate) start_time: Instant,
    pub(crate) phases: Vec<ProfilePhase>,
    pub(crate) samples: Vec<ResourceSample>,
}

impl JobProfileBuilder {
    pub(crate) fn new(job_id: JobId) -> Self {
        Self {
            job_id,
            start_time: Instant::now(),
            phases: Vec::new(),
            samples: Vec::new(),
        }
    }

    pub(crate) fn add_phase(&mut self, phase: ProfilePhase) {
        self.phases.push(phase);
    }

    pub(crate) fn add_sample(&mut self, sample: ResourceSample) {
        self.samples.push(sample);
    }

    pub(crate) fn build(self) -> JobProfile {
        let total_time = self.start_time.elapsed();

        // Analyze for bottlenecks
        let bottlenecks = self.detect_bottlenecks();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&bottlenecks);

        JobProfile {
            job_id: self.job_id,
            total_time,
            phases: self.phases,
            resource_samples: self.samples,
            bottlenecks,
            recommendations,
        }
    }

    fn detect_bottlenecks(&self) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // Check for CPU bottlenecks
        let high_cpu_samples = self.samples.iter().filter(|s| s.cpu_percent > 90.0).count();

        if high_cpu_samples > self.samples.len() / 2 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::CpuBound,
                severity: 80,
                description: "High CPU usage detected".to_string(),
                phase: None,
                duration: Duration::from_secs(high_cpu_samples as u64),
            });
        }

        // Check for I/O wait
        for phase in &self.phases {
            if phase.io_time > phase.duration / 2 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::IoWait,
                    severity: 70,
                    description: format!("High I/O wait in phase {}", phase.name),
                    phase: Some(phase.name.clone()),
                    duration: phase.io_time,
                });
            }
        }

        bottlenecks
    }

    fn generate_recommendations(&self, bottlenecks: &[Bottleneck]) -> Vec<String> {
        let mut recommendations = Vec::new();

        for bottleneck in bottlenecks {
            match bottleneck.bottleneck_type {
                BottleneckType::CpuBound => {
                    recommendations.push("Consider parallelizing CPU-intensive operations".to_string());
                }
                BottleneckType::IoWait => {
                    recommendations.push("Consider using async I/O or batching operations".to_string());
                }
                BottleneckType::MemoryPressure => {
                    recommendations.push("Consider streaming data instead of loading into memory".to_string());
                }
                _ => {}
            }
        }

        recommendations
    }
}
