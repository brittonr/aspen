
pub fn parse_sightglass_measurements(
    profile: &super::model::PerformanceProfile,
    suite: &super::model::BenchmarkSuite,
    expected_architecture: &str,
    bytes: &[u8],
) -> super::model::PerformanceResult<Vec<super::model::PhaseSamples>> {
    super::profile::validate_performance_profile(profile)?;
    super::comparison::validate_suite_instance(profile, suite)?;
    let measurements: Vec<RawSightglassMeasurement> = serde_json::from_slice(bytes)
        .map_err(|error| super::model::PerformanceDenial::new(format!("Sightglass raw JSON is malformed: {error}")))?;
    if measurements.is_empty() {
        return Err(super::model::PerformanceDenial::new("Sightglass raw JSON contains no measurements"));
    }
    let process_ordinals = diagnostic_process_ordinals(&measurements, suite)?;
    let mut phases = group_phase_samples(measurements, suite, expected_architecture, &process_ordinals)?;
    validate_phase_grid(&mut phases, profile, suite)?;
    Ok(phases)
}

/// Maps each diagnostic process that produced a selected-phase measurement to its dense ordinal,
/// requiring exactly the suite's process count.
fn diagnostic_process_ordinals(
    measurements: &[RawSightglassMeasurement],
    suite: &super::model::BenchmarkSuite,
) -> super::model::PerformanceResult<std::collections::BTreeMap<u32, u32>> {
    let admitted_processes = measurements
        .iter()
        .filter(|measurement| measurement.event == suite.measurement)
        .filter_map(|measurement| {
            super::model::PerformancePhase::parse(&measurement.phase)
                .filter(|phase| suite.phases.contains(phase))
                .map(|_| measurement.process)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let expected_processes = usize::try_from(suite.sampling.processes).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass process count is unsupported: {error}"))
    })?;
    if admitted_processes.len() != expected_processes {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass raw JSON has an incomplete or extra diagnostic process set",
        ));
    }
    admitted_processes
        .into_iter()
        .enumerate()
        .map(|(ordinal, diagnostic_process)| {
            u32::try_from(ordinal).map(|ordinal| (diagnostic_process, ordinal)).map_err(|error| {
                super::model::PerformanceDenial::new(format!("Sightglass process ordinal is unsupported: {error}"))
            })
        })
        .collect::<super::model::PerformanceResult<std::collections::BTreeMap<_, _>>>()
}

/// Groups the selected measurements by phase, requiring one host architecture and one diagnostic
/// identity.
fn group_phase_samples(
    measurements: Vec<RawSightglassMeasurement>,
    suite: &super::model::BenchmarkSuite,
    expected_architecture: &str,
    process_ordinals: &std::collections::BTreeMap<u32, u32>,
) -> super::model::PerformanceResult<Vec<super::model::PhaseSamples>> {
    // Selected groups share the suite event and an admitted phase, so each suite phase yields at most
    // one group.
    let mut phases = Vec::<super::model::PhaseSamples>::with_capacity(suite.phases.len());
    let mut diagnostic_identity = None;
    for measurement in measurements {
        if measurement.arch != expected_architecture {
            return Err(super::model::PerformanceDenial::new(
                "Sightglass measurement architecture differs from the admitted host",
            ));
        }
        if measurement.engine.trim().is_empty() || measurement.wasm.trim().is_empty() {
            return Err(super::model::PerformanceDenial::new(
                "Sightglass measurement omits its diagnostic engine or benchmark label",
            ));
        }
        let current_identity = (measurement.engine.clone(), measurement.engine_flags.clone(), measurement.wasm.clone());
        if diagnostic_identity.as_ref().is_some_and(|expected| expected != &current_identity) {
            return Err(super::model::PerformanceDenial::new(
                "Sightglass raw output mixes diagnostic engine or benchmark identities",
            ));
        }
        diagnostic_identity = Some(current_identity);
        if measurement.event != suite.measurement {
            continue;
        }
        let phase = super::model::PerformancePhase::parse(&measurement.phase)
            .ok_or_else(|| super::model::PerformanceDenial::new("Sightglass measurement uses an unsupported phase"))?;
        if !suite.phases.contains(&phase) {
            return Err(super::model::PerformanceDenial::new(
                "Sightglass measurement phase is not admitted by the suite",
            ));
        }
        let process = process_ordinals.get(&measurement.process).copied().ok_or_else(|| {
            super::model::PerformanceDenial::new(
                "Sightglass selected measurement uses an unadmitted diagnostic process",
            )
        })?;
        let sample = super::model::PerformanceSample {
            process,
            iteration: measurement.iteration,
            count: measurement.count,
        };
        match phases.iter_mut().find(|group| group.phase == phase && group.event == measurement.event) {
            Some(group) => group.samples.push(sample),
            None => phases.push(super::model::PhaseSamples {
                phase,
                event: measurement.event,
                samples: vec![sample],
            }),
        }
    }
    Ok(phases)
}

/// Every admitted phase must hold a complete, duplicate-free, in-bound process-by-iteration sample
/// grid.
fn validate_phase_grid(
    phases: &mut [super::model::PhaseSamples],
    profile: &super::model::PerformanceProfile,
    suite: &super::model::BenchmarkSuite,
) -> super::model::PerformanceResult<()> {
    for required in &suite.phases {
        if !phases.iter().any(|group| group.phase == *required) {
            return Err(super::model::PerformanceDenial::new(format!(
                "Sightglass raw JSON omits the {} phase for the selected event",
                required.as_str()
            )));
        }
    }
    phases.sort_by(|left, right| (left.phase, &left.event).cmp(&(right.phase, &right.event)));
    let expected_samples = usize::try_from(suite.sampling.expected_samples_per_phase()?).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass sample count is unsupported: {error}"))
    })?;
    let expected_iterations = usize::try_from(suite.sampling.iterations_per_process).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass iteration count is unsupported: {error}"))
    })?;
    if phases.len() != suite.phases.len() {
        return Err(super::model::PerformanceDenial::new(
            "Sightglass output contains duplicate or extra phase/event groups",
        ));
    }
    for phase in phases.iter_mut() {
        phase.samples.sort_by_key(|sample| (sample.process, sample.iteration));
        let is_complete_coordinate_grid = (0..suite.sampling.processes).all(|process| {
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
            || !is_complete_coordinate_grid
            || phase
                .samples
                .iter()
                .any(|sample| sample.count == 0 || sample.count > profile.comparison.max_sample_value)
            || phase
                .samples
                .windows(ADJACENT_PAIR_WIDTH)
                .any(|pair| (pair[0].process, pair[0].iteration) == (pair[1].process, pair[1].iteration))
        {
            return Err(super::model::PerformanceDenial::new(format!(
                "Sightglass {} samples are incomplete, duplicate, zero, or over bound",
                phase.phase.as_str()
            )));
        }
    }
    Ok(())
}

const fn absent_engine_flags() -> Option<String> {
    None
}

#[derive(Debug, serde::Deserialize)]
struct RawSightglassMeasurement {
    arch: String,
    engine: String,
    #[serde(default = "absent_engine_flags")]
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
    process_path: std::path::PathBuf,
}

fn open_admitted_process_file(
    path: &std::path::Path,
    expected_ref: &str,
    maximum_bytes: u64,
    label: &str,
) -> super::model::PerformanceResult<AdmittedProcessFile> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let leaf = path
        .file_name()
        .ok_or_else(|| super::model::PerformanceDenial::new(format!("Sightglass {label} path has no file name")))?;
    let directory = cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority()).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass could not open {label} parent authority: {error}"))
    })?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = directory.open_with(std::path::Path::new(leaf), &options).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass could not open no-follow {label} artifact: {error}"))
    })?;
    let metadata = file.metadata().map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass {label} metadata failed: {error}"))
    })?;
    if !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(super::model::PerformanceDenial::new(format!(
            "Sightglass {label} artifact is not a bounded regular file"
        )));
    }
    let maximum_bytes = usize::try_from(maximum_bytes).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass {label} artifact bound is unsupported: {error}"))
    })?;
    let bytes = read_bounded(&mut file, maximum_bytes).map_err(|error| {
        super::model::PerformanceDenial::new(format!("Sightglass {label} artifact read failed: {error}"))
    })?;
    if super::model::content_ref(&bytes) != expected_ref {
        return Err(super::model::PerformanceDenial::new(format!(
            "Sightglass {label} artifact differs from its admitted content identity"
        )));
    }
    if !metadata.permissions().readonly() {
        return Err(super::model::PerformanceDenial::new(format!(
            "Sightglass {label} artifact is mutable after content admission"
        )));
    }
    file.seek(std::io::SeekFrom::Start(0))
        .map_err(|error| super::model::PerformanceDenial::new(format!("Sightglass {label} rewind failed: {error}")))?;
    let process_path = process_file_path(&file, label)?;
    Ok(AdmittedProcessFile {
        _file: file,
        process_path,
    })
}

#[cfg(target_os = "linux")]
fn process_file_path(file: &cap_std::fs::File, _label: &str) -> super::model::PerformanceResult<std::path::PathBuf> {
    use std::os::fd::AsRawFd;

    Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{}", std::process::id(), file.as_raw_fd())))
}

#[cfg(not(target_os = "linux"))]
fn process_file_path(_file: &cap_std::fs::File, label: &str) -> super::model::PerformanceResult<std::path::PathBuf> {
    Err(super::model::PerformanceDenial::new(format!(
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
