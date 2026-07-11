use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::fabric_time::CanonicalTimeEventKind;
use molten::fabric_time::ExecutableFabricTimeFixtureRun;
use molten::fabric_time::FabricTimeFixtureSelection;

const ARTIFACT_INDEX_WIDTH: usize = 4;
const FIXED_ARTIFACT_COUNT: usize = 3;
const MAX_ARTIFACTS: usize = 4_096;
const MAX_REPORT_ARTIFACT_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedArtifact {
    relative_path: PathBuf,
    content: String,
}

pub(super) fn run_fixture(selection: FabricTimeFixtureSelection, out: PathBuf) -> Result<()> {
    let run = molten::fabric_time::run_executable_fabric_time_fixture(selection)?;
    let plan = plan_fixture_artifacts(&run)?;
    write_artifacts(&out, &plan)?;
    println!(
        "fabric-time fixture ok profile={} report={} evidence={} live-domain={} simulation-domain={} entropy-source={} out={}",
        selection.as_str(),
        run.report.report_ref,
        run.events.len(),
        run.live_conformance.domain.as_str(),
        run.simulation_conformance.domain.as_str(),
        run.production_entropy_source,
        out.display(),
    );
    Ok(())
}

pub(super) fn show(report: PathBuf) -> Result<()> {
    let metadata = std::fs::metadata(&report).map_err(MoltenError::from)?;
    if metadata.len() > MAX_REPORT_ARTIFACT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-time report is {} bytes; maximum is {MAX_REPORT_ARTIFACT_BYTES}",
            metadata.len()
        )));
    }
    let source = std::fs::read_to_string(&report).map_err(MoltenError::from)?;
    let value = molten::preserves_rail::parse_text(&source)?;
    let readback = molten::fabric_time::parse_fabric_time_run_readback(&value)?;
    println!(
        "fabric-time report ok profile={} generation={} final-ticks={} timers={} scheduler={} entropy={} deadline-lease={} faults={} live-clock={} conformance={} report={}",
        readback.profile_kind,
        readback.generation,
        readback.final_time_ticks,
        readback.timer_events,
        readback.scheduler_events,
        readback.entropy_events,
        readback.deadline_lease_events,
        readback.fault_events,
        readback.live_clock_observed,
        readback.shared_conformance_passed,
        readback.report_ref,
    );
    Ok(())
}

fn plan_fixture_artifacts(run: &ExecutableFabricTimeFixtureRun) -> Result<Vec<PlannedArtifact>> {
    let artifact_count = run
        .events
        .len()
        .checked_add(FIXED_ARTIFACT_COUNT)
        .ok_or_else(|| MoltenError::invalid_harness("fabric-time artifact count overflow"))?;
    if artifact_count > MAX_ARTIFACTS {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-time artifact count {artifact_count} exceeds {MAX_ARTIFACTS}"
        )));
    }
    let mut artifacts = vec![
        planned("profiles/live.preserves", &run.live_profile.value)?,
        planned("profiles/deterministic-simulation.preserves", &run.simulation_profile.value)?,
        planned("report.preserves", &run.report.value)?,
    ];
    for (index, event) in run.events.iter().enumerate() {
        let kind = event_kind(event.kind);
        let filename = format!("evidence/{index:0width$}-{kind}.preserves", width = ARTIFACT_INDEX_WIDTH);
        artifacts.push(planned(filename, &event.value)?);
    }
    validate_artifact_plan(&artifacts)?;
    Ok(artifacts)
}

fn planned(relative_path: impl Into<PathBuf>, value: &preserves::IOValue) -> Result<PlannedArtifact> {
    Ok(PlannedArtifact {
        relative_path: relative_path.into(),
        content: molten::preserves_rail::to_text(value)?,
    })
}

fn validate_artifact_plan(artifacts: &[PlannedArtifact]) -> Result<()> {
    let mut paths = std::collections::BTreeSet::new();
    for artifact in artifacts {
        validate_relative_artifact_path(&artifact.relative_path)?;
        if !paths.insert(artifact.relative_path.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate fabric-time artifact path {}",
                artifact.relative_path.display()
            )));
        }
    }
    Ok(())
}

fn validate_relative_artifact_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(MoltenError::invalid_harness("fabric-time artifact path must be non-empty and relative"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-time artifact path escapes output root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn write_artifacts(root: &Path, artifacts: &[PlannedArtifact]) -> Result<()> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    for artifact in artifacts {
        let path = root.join(&artifact.relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
        }
        std::fs::write(path, artifact.content.as_bytes()).map_err(MoltenError::from)?;
    }
    Ok(())
}

const fn event_kind(kind: CanonicalTimeEventKind) -> &'static str {
    match kind {
        CanonicalTimeEventKind::ClockAnomaly => "clock-anomaly",
        CanonicalTimeEventKind::Timer => "timer",
        CanonicalTimeEventKind::Scheduler => "scheduler",
        CanonicalTimeEventKind::Entropy => "entropy",
        CanonicalTimeEventKind::Deadline => "deadline",
        CanonicalTimeEventKind::Lease => "lease",
        CanonicalTimeEventKind::Fault => "fault",
        CanonicalTimeEventKind::Conformance => "conformance",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_plan_is_bounded_relative_and_complete() {
        let run =
            molten::fabric_time::run_executable_fabric_time_fixture(FabricTimeFixtureSelection::Both).expect("fixture");
        let plan = plan_fixture_artifacts(&run).expect("plan");
        assert_eq!(plan.len(), run.events.len() + FIXED_ARTIFACT_COUNT);
        assert!(plan.iter().all(|artifact| artifact.relative_path.is_relative()));
        assert!(plan.iter().any(|artifact| artifact.relative_path == Path::new("report.preserves")));
    }

    #[test]
    fn artifact_path_rejects_parent_escape() {
        let error =
            validate_relative_artifact_path(Path::new("../escape.preserves")).expect_err("parent escape must fail");
        assert!(error.to_string().contains("escapes output root"));
    }
}
