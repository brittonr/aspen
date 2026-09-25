use std::io::Read;
use std::io::Seek;

use cap_fs_ext::OpenOptionsFollowExt;

const SIGHTGLASS_BENCHMARK_COMMAND: &str = "benchmark";
const SIGHTGLASS_RAW_FLAG: &str = "--raw";
const SIGHTGLASS_OUTPUT_FORMAT_FLAG: &str = "--output-format";
const SIGHTGLASS_JSON_FORMAT: &str = "json";
const SIGHTGLASS_PROCESSES_FLAG: &str = "--processes";
const SIGHTGLASS_ITERATIONS_FLAG: &str = "--iterations-per-process";
const SIGHTGLASS_ENGINE_FLAG: &str = "--engine";
const SIGHTGLASS_MEASURE_FLAG: &str = "--measure";
const SIGHTGLASS_PIN_FLAG: &str = "--pin";
const SIGHTGLASS_ARGUMENT_SEPARATOR: &str = "--";
const MAX_DIAGNOSTIC_STDERR_BYTES: usize = 65_536;
const BOUNDED_READ_BUFFER_BYTES: usize = 8_192;
const PROCESS_POLL_INTERVAL_MILLISECONDS: u64 = 10;
const ADJACENT_PAIR_WIDTH: usize = 2;
const SIGHTGLASS_SUBRUN_PROCESSES: u32 = 1;

#[derive(Debug, Clone)]
pub struct SightglassProcessInvocation<'a> {
    pub program: &'a std::path::Path,
    pub engine: &'a std::path::Path,
    pub benchmark: &'a std::path::Path,
    pub benchmark_ref: &'a str,
    pub profile: &'a super::model::PerformanceProfile,
    pub suite: &'a super::model::BenchmarkSuite,
    pub expected_architecture: &'a str,
    pub max_output_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SightglassProcessResult {
    pub phases: Vec<super::model::PhaseSamples>,
    pub diagnostic_stderr: String,
}

pub fn run_sightglass_process(
    invocation: &SightglassProcessInvocation<'_>,
) -> super::model::PerformanceResult<SightglassProcessResult> {
    validate_invocation(invocation)?;
    let [runner_file, engine_file, benchmark_file] = open_admitted_files(invocation)?;
    let stdout_limit = usize::try_from(invocation.max_output_bytes).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass output bound is unsupported: {error}"))
    })?;
    let total_timeout = std::time::Duration::from_secs(invocation.profile.comparison.max_sightglass_run_seconds);
    let mut suite_deadline =
        crate::fabric_time::SupervisionDeadline::after(total_timeout).map_err(supervision_denial)?;
    let mut total_stdout_bytes = 0_usize;
    let mut diagnostic_stderr_bytes = Vec::new();
    let mut raw_measurements = Vec::<serde_json::Value>::new();
    for process_ordinal in 0..invocation.suite.sampling.processes {
        let remaining_timeout = Some(suite_deadline.remaining().map_err(supervision_denial)?)
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| {
                super::model::PerformanceDenial::new("Sightglass suite exceeded its admitted total runtime")
            })?;
        let remaining_stdout =
            stdout_limit.checked_sub(total_stdout_bytes).filter(|remaining| *remaining > 0).ok_or_else(|| {
                super::model::PerformanceDenial::new("Sightglass suite exceeded its admitted total output")
            })?;
        let output = run_sightglass_subprocess(SubprocessInput {
            runner: &runner_file.process_path,
            engine: &engine_file.process_path,
            benchmark: &benchmark_file.process_path,
            suite: invocation.suite,
            stdout_limit: remaining_stdout,
            timeout: remaining_timeout,
        })?;
        total_stdout_bytes = total_stdout_bytes
            .checked_add(output.stdout.len())
            .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass suite output accounting overflowed"))?;
        append_diagnostic_stderr(&mut diagnostic_stderr_bytes, output.stderr)?;
        let process_measurements = process_measurements(&output.stdout, process_ordinal)?;
        raw_measurements.reserve(process_measurements.len());
        raw_measurements.extend(process_measurements);
    }
    let stdout = serde_json::to_vec(&raw_measurements).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass raw JSON normalization failed: {error}"))
    })?;
    if stdout.len() > stdout_limit {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass normalized suite output exceeded its admitted bound",
        ));
    }
    let diagnostic_stderr = bounded_diagnostic(&diagnostic_stderr_bytes);
    let phases =
        parse_sightglass_measurements(invocation.profile, invocation.suite, invocation.expected_architecture, &stdout)?;
    Ok(SightglassProcessResult {
        phases,
        diagnostic_stderr,
    })
}

fn validate_invocation(invocation: &SightglassProcessInvocation<'_>) -> super::model::PerformanceResult<()> {
    super::profile::validate_performance_profile(invocation.profile)?;
    super::comparison::validate_suite_instance(invocation.profile, invocation.suite)?;
    if invocation.expected_architecture.trim().is_empty()
        || invocation.max_output_bytes == 0
        || invocation.max_output_bytes > invocation.profile.comparison.max_sightglass_output_bytes
    {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass invocation requires an architecture and reviewed output bound",
        ));
    }
    if !invocation.suite.workload_refs.iter().any(|value| value == invocation.benchmark_ref) {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass benchmark identity is absent from the admitted suite",
        ));
    }
    Ok(())
}

/// The runner, engine, and benchmark files, each opened against its admitted artifact ref and byte
/// bound.
fn open_admitted_files(
    invocation: &SightglassProcessInvocation<'_>,
) -> super::model::PerformanceResult<[AdmittedProcessFile; 3]> {
    let runner_file = open_admitted_process_file(
        invocation.program,
        &invocation.suite.runner_artifact_ref,
        invocation.profile.comparison.max_sightglass_runner_bytes,
        "runner",
    )?;
    let engine_file = open_admitted_process_file(
        invocation.engine,
        &invocation.suite.engine_artifact_ref,
        invocation.profile.comparison.max_sightglass_engine_bytes,
        "engine",
    )?;
    let benchmark_file = open_admitted_process_file(
        invocation.benchmark,
        invocation.benchmark_ref,
        invocation.profile.comparison.max_sightglass_benchmark_bytes,
        "benchmark",
    )?;
    Ok([runner_file, engine_file, benchmark_file])
}

fn append_diagnostic_stderr(
    diagnostic_stderr_bytes: &mut impl crate::bounded::VecSink<u8>,
    stderr: Vec<u8>,
) -> super::model::PerformanceResult<()> {
    let diagnostic_total = diagnostic_stderr_bytes
        .item_count()
        .checked_add(stderr.len())
        .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass diagnostic accounting overflowed"))?;
    if diagnostic_total > MAX_DIAGNOSTIC_STDERR_BYTES {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass suite diagnostic stderr exceeded its admitted bound",
        ));
    }
    diagnostic_stderr_bytes.reserve_items(stderr.len());
    diagnostic_stderr_bytes.extend_items(stderr);
    Ok(())
}

/// One process's raw Sightglass measurements, each relabelled with the diagnostic process ordinal.
fn process_measurements(
    stdout: &[u8],
    process_ordinal: u32,
) -> super::model::PerformanceResult<Vec<serde_json::Value>> {
    let mut process_measurements: Vec<serde_json::Value> = serde_json::from_slice(stdout)
        .map_err(|error| super::model::PerformanceDenial::new(format!("Sightglass raw JSON is malformed: {error}")))?;
    for measurement in &mut process_measurements {
        let process = measurement
            .get_mut("process")
            .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass raw JSON omits its diagnostic process"))?;
        *process = serde_json::Value::from(process_ordinal);
    }
    Ok(process_measurements)
}

pub fn sightglass_arguments(suite: &super::model::BenchmarkSuite) -> Vec<std::ffi::OsString> {
    let mut arguments = vec![
        std::ffi::OsString::from(SIGHTGLASS_BENCHMARK_COMMAND),
        std::ffi::OsString::from(SIGHTGLASS_PROCESSES_FLAG),
        std::ffi::OsString::from(SIGHTGLASS_SUBRUN_PROCESSES.to_string()),
        std::ffi::OsString::from(SIGHTGLASS_ITERATIONS_FLAG),
        std::ffi::OsString::from(suite.sampling.iterations_per_process.to_string()),
        std::ffi::OsString::from(SIGHTGLASS_MEASURE_FLAG),
        std::ffi::OsString::from(&suite.measurement),
        std::ffi::OsString::from(SIGHTGLASS_RAW_FLAG),
        std::ffi::OsString::from(SIGHTGLASS_OUTPUT_FORMAT_FLAG),
        std::ffi::OsString::from(SIGHTGLASS_JSON_FORMAT),
    ];
    if suite.pin_to_single_core {
        arguments.push(std::ffi::OsString::from(SIGHTGLASS_PIN_FLAG));
    }
    arguments
}

struct SightglassSubprocessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn supervision_denial(error: crate::error::MoltenError) -> super::model::PerformanceDenial {
    super::model::PerformanceDenial::new(format!("Sightglass supervision clock denied: {error}"))
}

struct SubprocessInput<'a> {
    runner: &'a std::path::Path,
    engine: &'a std::path::Path,
    benchmark: &'a std::path::Path,
    suite: &'a super::model::BenchmarkSuite,
    stdout_limit: usize,
    timeout: std::time::Duration,
}

fn run_sightglass_subprocess(
    input: SubprocessInput<'_>,
) -> super::model::PerformanceResult<SightglassSubprocessOutput> {
    let SubprocessInput {
        runner,
        engine,
        benchmark,
        suite,
        stdout_limit,
        timeout,
    } = input;
    let mut command = std::process::Command::new(runner);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .args(sightglass_arguments(suite))
        .arg(SIGHTGLASS_ENGINE_FLAG)
        .arg(engine)
        .arg(SIGHTGLASS_ARGUMENT_SEPARATOR)
        .arg(benchmark);
    let mut child = command.spawn().map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass process could not start: {error}"))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass process stdout pipe is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass process stderr pipe is unavailable"))?;
    let stdout_reader = std::thread::spawn(move || read_bounded(stdout, stdout_limit));
    let stderr_reader = std::thread::spawn(move || read_bounded(stderr, MAX_DIAGNOSTIC_STDERR_BYTES));
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(timeout).map_err(supervision_denial)?;
    let status = loop {
        match child.try_wait().map_err(|error| {
            super::model::PerformanceDenial::new(format!("Sightglass process status failed: {error}"))
        })? {
            Some(status) => break status,
            None if deadline.is_expired().map_err(supervision_denial)? => {
                let kill_diagnostic = child.kill().err().map_or_else(String::new, |error| format!(": {error}"));
                let wait_diagnostic = child.wait().err().map_or_else(String::new, |error| format!(": {error}"));
                let stdout_diagnostic = join_bounded_reader(stdout_reader, "stdout")
                    .err()
                    .map_or_else(String::new, |error| format!(": {error}"));
                let stderr_diagnostic = join_bounded_reader(stderr_reader, "stderr")
                    .err()
                    .map_or_else(String::new, |error| format!(": {error}"));
                return Err(super::model::PerformanceDenial::new(format!(
                    "Sightglass process exceeded its admitted runtime{kill_diagnostic}{wait_diagnostic}{stdout_diagnostic}{stderr_diagnostic}"
                )));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(PROCESS_POLL_INTERVAL_MILLISECONDS)),
        }
    };
    let stdout = join_bounded_reader(stdout_reader, "stdout")?;
    let stderr = join_bounded_reader(stderr_reader, "stderr")?;
    if !status.success() {
        return Err(super::model::PerformanceDenial::new(format!(
            "Sightglass process denied with status {status}: {}",
            bounded_diagnostic(&stderr)
        )));
    }
    Ok(SightglassSubprocessOutput { stdout, stderr })
}
