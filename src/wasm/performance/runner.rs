use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use cap_fs_ext::FollowSymlinks;
use cap_fs_ext::OpenOptionsFollowExt;
use serde::Deserialize;

use super::comparison::validate_suite_instance;
use super::model::BenchmarkSuite;
use super::model::PerformanceDenial;
use super::model::PerformancePhase;
use super::model::PerformanceProfile;
use super::model::PerformanceResult;
use super::model::PerformanceSample;
use super::model::PhaseSamples;
use super::model::content_ref;
use super::profile::validate_performance_profile;

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
    pub program: &'a Path,
    pub engine: &'a Path,
    pub benchmark: &'a Path,
    pub benchmark_ref: &'a str,
    pub profile: &'a PerformanceProfile,
    pub suite: &'a BenchmarkSuite,
    pub expected_architecture: &'a str,
    pub max_output_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SightglassProcessResult {
    pub phases: Vec<PhaseSamples>,
    pub diagnostic_stderr: String,
}

pub fn run_sightglass_process(
    invocation: &SightglassProcessInvocation<'_>,
) -> PerformanceResult<SightglassProcessResult> {
    validate_performance_profile(invocation.profile)?;
    validate_suite_instance(invocation.profile, invocation.suite)?;
    if invocation.expected_architecture.trim().is_empty()
        || invocation.max_output_bytes == 0
        || invocation.max_output_bytes > invocation.profile.comparison.max_sightglass_output_bytes
    {
        return Err(PerformanceDenial::new("Sightglass invocation requires an architecture and reviewed output bound"));
    }
    if !invocation.suite.workload_refs.iter().any(|value| value == invocation.benchmark_ref) {
        return Err(PerformanceDenial::new("Sightglass benchmark identity is absent from the admitted suite"));
    }
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
    let stdout_limit = usize::try_from(invocation.max_output_bytes)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass output bound is unsupported: {error}")))?;
    let total_timeout = Duration::from_secs(invocation.profile.comparison.max_sightglass_run_seconds);
    let started = Instant::now();
    let mut total_stdout_bytes = 0_usize;
    let mut diagnostic_stderr_bytes = Vec::new();
    let mut raw_measurements = Vec::<serde_json::Value>::new();
    for process_ordinal in 0..invocation.suite.sampling.processes {
        let remaining_timeout = total_timeout
            .checked_sub(started.elapsed())
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| PerformanceDenial::new("Sightglass suite exceeded its admitted total runtime"))?;
        let remaining_stdout = stdout_limit
            .checked_sub(total_stdout_bytes)
            .filter(|remaining| *remaining > 0)
            .ok_or_else(|| PerformanceDenial::new("Sightglass suite exceeded its admitted total output"))?;
        let output = run_sightglass_subprocess(
            &runner_file.process_path,
            &engine_file.process_path,
            &benchmark_file.process_path,
            invocation.suite,
            remaining_stdout,
            remaining_timeout,
        )?;
        total_stdout_bytes = total_stdout_bytes
            .checked_add(output.stdout.len())
            .ok_or_else(|| PerformanceDenial::new("Sightglass suite output accounting overflowed"))?;
        let diagnostic_total = diagnostic_stderr_bytes
            .len()
            .checked_add(output.stderr.len())
            .ok_or_else(|| PerformanceDenial::new("Sightglass diagnostic accounting overflowed"))?;
        if diagnostic_total > MAX_DIAGNOSTIC_STDERR_BYTES {
            return Err(PerformanceDenial::new("Sightglass suite diagnostic stderr exceeded its admitted bound"));
        }
        diagnostic_stderr_bytes.extend(output.stderr);
        let mut process_measurements: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
            .map_err(|error| PerformanceDenial::new(format!("Sightglass raw JSON is malformed: {error}")))?;
        for measurement in &mut process_measurements {
            let process = measurement
                .get_mut("process")
                .ok_or_else(|| PerformanceDenial::new("Sightglass raw JSON omits its diagnostic process"))?;
            *process = serde_json::Value::from(process_ordinal);
        }
        raw_measurements.extend(process_measurements);
    }
    let stdout = serde_json::to_vec(&raw_measurements)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass raw JSON normalization failed: {error}")))?;
    if stdout.len() > stdout_limit {
        return Err(PerformanceDenial::new("Sightglass normalized suite output exceeded its admitted bound"));
    }
    let diagnostic_stderr = bounded_diagnostic(&diagnostic_stderr_bytes);
    let phases =
        parse_sightglass_measurements(invocation.profile, invocation.suite, invocation.expected_architecture, &stdout)?;
    Ok(SightglassProcessResult {
        phases,
        diagnostic_stderr,
    })
}

pub fn sightglass_arguments(suite: &BenchmarkSuite) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from(SIGHTGLASS_BENCHMARK_COMMAND),
        OsString::from(SIGHTGLASS_PROCESSES_FLAG),
        OsString::from(SIGHTGLASS_SUBRUN_PROCESSES.to_string()),
        OsString::from(SIGHTGLASS_ITERATIONS_FLAG),
        OsString::from(suite.sampling.iterations_per_process.to_string()),
        OsString::from(SIGHTGLASS_MEASURE_FLAG),
        OsString::from(&suite.measurement),
        OsString::from(SIGHTGLASS_RAW_FLAG),
        OsString::from(SIGHTGLASS_OUTPUT_FORMAT_FLAG),
        OsString::from(SIGHTGLASS_JSON_FORMAT),
    ];
    if suite.pin_to_single_core {
        arguments.push(OsString::from(SIGHTGLASS_PIN_FLAG));
    }
    arguments
}

struct SightglassSubprocessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_sightglass_subprocess(
    runner: &Path,
    engine: &Path,
    benchmark: &Path,
    suite: &BenchmarkSuite,
    stdout_limit: usize,
    timeout: Duration,
) -> PerformanceResult<SightglassSubprocessOutput> {
    let mut command = Command::new(runner);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(sightglass_arguments(suite))
        .arg(SIGHTGLASS_ENGINE_FLAG)
        .arg(engine)
        .arg(SIGHTGLASS_ARGUMENT_SEPARATOR)
        .arg(benchmark);
    let mut child = command
        .spawn()
        .map_err(|error| PerformanceDenial::new(format!("Sightglass process could not start: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PerformanceDenial::new("Sightglass process stdout pipe is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PerformanceDenial::new("Sightglass process stderr pipe is unavailable"))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, stdout_limit));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAX_DIAGNOSTIC_STDERR_BYTES));
    let started = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| PerformanceDenial::new(format!("Sightglass process status failed: {error}")))?
        {
            Some(status) => break status,
            None if started.elapsed() >= timeout => {
                let kill_diagnostic = child.kill().err().map_or_else(String::new, |error| format!(": {error}"));
                let _ = child.wait();
                let _ = join_bounded_reader(stdout_reader, "stdout");
                let _ = join_bounded_reader(stderr_reader, "stderr");
                return Err(PerformanceDenial::new(format!(
                    "Sightglass process exceeded its admitted runtime{kill_diagnostic}"
                )));
            }
            None => thread::sleep(Duration::from_millis(PROCESS_POLL_INTERVAL_MILLISECONDS)),
        }
    };
    let stdout = join_bounded_reader(stdout_reader, "stdout")?;
    let stderr = join_bounded_reader(stderr_reader, "stderr")?;
    if !status.success() {
        return Err(PerformanceDenial::new(format!(
            "Sightglass process denied with status {status}: {}",
            bounded_diagnostic(&stderr)
        )));
    }
    Ok(SightglassSubprocessOutput { stdout, stderr })
}

pub fn parse_sightglass_measurements(
    profile: &PerformanceProfile,
    suite: &BenchmarkSuite,
    expected_architecture: &str,
    bytes: &[u8],
) -> PerformanceResult<Vec<PhaseSamples>> {
    validate_performance_profile(profile)?;
    validate_suite_instance(profile, suite)?;
    let measurements: Vec<RawSightglassMeasurement> = serde_json::from_slice(bytes)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass raw JSON is malformed: {error}")))?;
    if measurements.is_empty() {
        return Err(PerformanceDenial::new("Sightglass raw JSON contains no measurements"));
    }
    let admitted_processes = measurements
        .iter()
        .filter(|measurement| measurement.event == suite.measurement)
        .filter_map(|measurement| {
            PerformancePhase::parse(&measurement.phase)
                .filter(|phase| suite.phases.contains(phase))
                .map(|_| measurement.process)
        })
        .collect::<BTreeSet<_>>();
    let expected_processes = usize::try_from(suite.sampling.processes)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass process count is unsupported: {error}")))?;
    if admitted_processes.len() != expected_processes {
        return Err(PerformanceDenial::new("Sightglass raw JSON has an incomplete or extra diagnostic process set"));
    }
    let process_ordinals = admitted_processes
        .into_iter()
        .enumerate()
        .map(|(ordinal, diagnostic_process)| {
            u32::try_from(ordinal)
                .map(|ordinal| (diagnostic_process, ordinal))
                .map_err(|error| PerformanceDenial::new(format!("Sightglass process ordinal is unsupported: {error}")))
        })
        .collect::<PerformanceResult<BTreeMap<_, _>>>()?;
    let mut phases = Vec::<PhaseSamples>::new();
    let mut diagnostic_identity = None;
    for measurement in measurements {
        if measurement.arch != expected_architecture {
            return Err(PerformanceDenial::new("Sightglass measurement architecture differs from the admitted host"));
        }
        if measurement.engine.trim().is_empty() || measurement.wasm.trim().is_empty() {
            return Err(PerformanceDenial::new(
                "Sightglass measurement omits its diagnostic engine or benchmark label",
            ));
        }
        let current_identity = (measurement.engine.clone(), measurement.engine_flags.clone(), measurement.wasm.clone());
        if diagnostic_identity.as_ref().is_some_and(|expected| expected != &current_identity) {
            return Err(PerformanceDenial::new(
                "Sightglass raw output mixes diagnostic engine or benchmark identities",
            ));
        }
        diagnostic_identity = Some(current_identity);
        if measurement.event != suite.measurement {
            continue;
        }
        let phase = PerformancePhase::parse(&measurement.phase)
            .ok_or_else(|| PerformanceDenial::new("Sightglass measurement uses an unsupported phase"))?;
        if !suite.phases.contains(&phase) {
            return Err(PerformanceDenial::new("Sightglass measurement phase is not admitted by the suite"));
        }
        let process = process_ordinals.get(&measurement.process).copied().ok_or_else(|| {
            PerformanceDenial::new("Sightglass selected measurement uses an unadmitted diagnostic process")
        })?;
        let sample = PerformanceSample {
            process,
            iteration: measurement.iteration,
            count: measurement.count,
        };
        match phases.iter_mut().find(|group| group.phase == phase && group.event == measurement.event) {
            Some(group) => group.samples.push(sample),
            None => phases.push(PhaseSamples {
                phase,
                event: measurement.event,
                samples: vec![sample],
            }),
        }
    }
    for required in &suite.phases {
        if !phases.iter().any(|group| group.phase == *required) {
            return Err(PerformanceDenial::new(format!(
                "Sightglass raw JSON omits the {} phase for the selected event",
                required.as_str()
            )));
        }
    }
    phases.sort_by(|left, right| (left.phase, &left.event).cmp(&(right.phase, &right.event)));
    let expected_samples = usize::try_from(suite.sampling.expected_samples_per_phase()?)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass sample count is unsupported: {error}")))?;
    let expected_iterations = usize::try_from(suite.sampling.iterations_per_process)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass iteration count is unsupported: {error}")))?;
    if phases.len() != suite.phases.len() {
        return Err(PerformanceDenial::new("Sightglass output contains duplicate or extra phase/event groups"));
    }
    for phase in &mut phases {
        phase.samples.sort_by_key(|sample| (sample.process, sample.iteration));
        let complete_coordinate_grid = (0..suite.sampling.processes).all(|process| {
            let iterations = phase
                .samples
                .iter()
                .filter(|sample| sample.process == process)
                .map(|sample| sample.iteration)
                .collect::<Vec<_>>();
            iterations.len() == expected_iterations
                && iterations.iter().copied().eq(0..suite.sampling.iterations_per_process)
        });
        if phase.samples.len() != expected_samples
            || !complete_coordinate_grid
            || phase
                .samples
                .iter()
                .any(|sample| sample.count == 0 || sample.count > profile.comparison.max_sample_value)
            || phase
                .samples
                .windows(ADJACENT_PAIR_WIDTH)
                .any(|pair| (pair[0].process, pair[0].iteration) == (pair[1].process, pair[1].iteration))
        {
            return Err(PerformanceDenial::new(format!(
                "Sightglass {} samples are incomplete, duplicate, zero, or over bound",
                phase.phase.as_str()
            )));
        }
    }
    Ok(phases)
}

#[derive(Debug, Deserialize)]
struct RawSightglassMeasurement {
    arch: String,
    engine: String,
    #[serde(default)]
    engine_flags: Option<String>,
    wasm: String,
    process: u32,
    iteration: u32,
    phase: String,
    event: String,
    count: u64,
}

struct AdmittedProcessFile {
    _file: cap_std::fs::File,
    process_path: PathBuf,
}

fn open_admitted_process_file(
    path: &Path,
    expected_ref: &str,
    maximum_bytes: u64,
    label: &str,
) -> PerformanceResult<AdmittedProcessFile> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let leaf = path
        .file_name()
        .ok_or_else(|| PerformanceDenial::new(format!("Sightglass {label} path has no file name")))?;
    let directory = cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority()).map_err(|error| {
        PerformanceDenial::new(format!("Sightglass could not open {label} parent authority: {error}"))
    })?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory.open_with(Path::new(leaf), &options).map_err(|error| {
        PerformanceDenial::new(format!("Sightglass could not open no-follow {label} artifact: {error}"))
    })?;
    let metadata = file
        .metadata()
        .map_err(|error| PerformanceDenial::new(format!("Sightglass {label} metadata failed: {error}")))?;
    if !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(PerformanceDenial::new(format!("Sightglass {label} artifact is not a bounded regular file")));
    }
    let maximum_bytes = usize::try_from(maximum_bytes).map_err(|error| {
        PerformanceDenial::new(format!("Sightglass {label} artifact bound is unsupported: {error}"))
    })?;
    let bytes = read_bounded(&mut file, maximum_bytes)
        .map_err(|error| PerformanceDenial::new(format!("Sightglass {label} artifact read failed: {error}")))?;
    if content_ref(&bytes) != expected_ref {
        return Err(PerformanceDenial::new(format!(
            "Sightglass {label} artifact differs from its admitted content identity"
        )));
    }
    if !metadata.permissions().readonly() {
        return Err(PerformanceDenial::new(format!("Sightglass {label} artifact is mutable after content admission")));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| PerformanceDenial::new(format!("Sightglass {label} rewind failed: {error}")))?;
    let process_path = process_file_path(&file, label)?;
    Ok(AdmittedProcessFile {
        _file: file,
        process_path,
    })
}

#[cfg(target_os = "linux")]
fn process_file_path(file: &cap_std::fs::File, _label: &str) -> PerformanceResult<PathBuf> {
    use std::os::fd::AsRawFd;

    Ok(PathBuf::from(format!("/proc/{}/fd/{}", std::process::id(), file.as_raw_fd())))
}

#[cfg(not(target_os = "linux"))]
fn process_file_path(_file: &cap_std::fs::File, label: &str) -> PerformanceResult<PathBuf> {
    Err(PerformanceDenial::new(format!(
        "Sightglass same-handle {label} execution is unsupported on this host"
    )))
}

fn read_bounded(mut reader: impl Read, maximum_bytes: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; BOUNDED_READ_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(bytes);
        }
        let remaining = maximum_bytes.saturating_sub(bytes.len());
        if read > remaining {
            return Err("stream exceeded its admitted byte bound".to_string());
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

fn join_bounded_reader(reader: JoinHandle<Result<Vec<u8>, String>>, label: &str) -> PerformanceResult<Vec<u8>> {
    reader
        .join()
        .map_err(|_| PerformanceDenial::new(format!("Sightglass {label} reader panicked")))?
        .map_err(|error| PerformanceDenial::new(format!("Sightglass {label} read failed: {error}")))
}

fn bounded_diagnostic(stderr: &[u8]) -> String {
    let bounded = stderr.get(..MAX_DIAGNOSTIC_STDERR_BYTES).unwrap_or(stderr);
    String::from_utf8_lossy(bounded).into_owned()
}
