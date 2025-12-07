/// Soak test module providing reusable infrastructure for long-running tests.
///
/// This module contains utilities for:
/// - Metrics collection (SoakMetrics, SoakMetricsCollector)
/// - Test configuration (SoakTestConfig)
/// - Workload generation (Workload, WorkloadOp)
/// - Progress reporting (SoakProgressReporter)
///
/// All components follow Tiger Style principles:
/// - Explicit types (u64, u32, [T; N])
/// - Bounded values and fixed limits
/// - No unbounded growth
/// - Clear error handling

pub mod infrastructure;

pub use infrastructure::*;
