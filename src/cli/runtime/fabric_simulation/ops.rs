use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::fabric_simulation::CanonicalSimulatedWorld;
use molten::fabric_simulation::CanonicalSimulationDifferential;
use molten::fabric_simulation::CanonicalSimulationObservation;
use molten::fabric_simulation::CanonicalSimulationPortEvent;
use molten::fabric_simulation::CanonicalSimulationReproBundle;
use molten::fabric_simulation::CanonicalSimulationRun;
use molten::fabric_simulation::CanonicalSimulationShrink;
use molten::fabric_simulation::ReferenceShrinkFixture;
use molten::fabric_simulation::ReferenceSimulationFixtureRun;

const ARTIFACT_INDEX_WIDTH: usize = 4;
const RUN_FIXED_ARTIFACT_COUNT: usize = 4;
const EXPORT_ARTIFACT_COUNT: usize = 4;
const SHRINK_ARTIFACT_COUNT: usize = 3;
const MAX_ARTIFACTS: usize = 4_096;
const MAX_REPORT_ARTIFACT_BYTES: u64 = 1_048_576;
const MAX_ARTIFACT_CONTENT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedArtifact {
    relative_path: PathBuf,
    content: String,
}

pub(super) fn preflight() -> Result<()> {
    let world = molten::fabric_simulation::build_reference_simulated_world()?;
    println!(
        "fabric-simulation preflight ok world={} nodes={} ports={} workload={} faults={} profile={}",
        world.world_ref,
        world.admitted.manifest.nodes.len(),
        world.admitted.manifest.port_profiles.len(),
        world.admitted.manifest.workload.len(),
        world.admitted.manifest.faults.len(),
        world.admitted.manifest.claim_profile.as_str(),
    );
    Ok(())
}

pub(super) fn run(out: PathBuf) -> Result<()> {
    let fixture = molten::fabric_simulation::run_reference_simulation_fixture()?;
    let plan = plan_run_artifacts(&fixture)?;
    write_artifacts(&out, &plan)?;
    println!(
        "fabric-simulation run ok world={} run={} bundle={} choices={} events={} invariants={} decision={} out={}",
        fixture.world.world_ref,
        fixture.run.run_ref,
        fixture.bundle.bundle_ref,
        fixture.run.summary.choice_records.len(),
        fixture.observations.len(),
        fixture.run.summary.invariant_results.len(),
        fixture.run.summary.decision.as_str(),
        out.display(),
    );
    Ok(())
}

pub(super) fn replay(report: PathBuf) -> Result<()> {
    let value = read_report(&report)?;
    let expected = molten::fabric_simulation::parse_simulation_run_readback(&value)?;
    let replay = molten::fabric_simulation::run_reference_simulation_fixture()?;
    if expected.world_ref != replay.world.world_ref {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation replay world mismatch: expected={} actual={}",
            expected.world_ref, replay.world.world_ref
        )));
    }
    if expected.run_ref != replay.run.run_ref {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation replay diverged: expected={} actual={}",
            expected.run_ref, replay.run.run_ref
        )));
    }
    println!(
        "fabric-simulation replay ok world={} run={} choices={} events={} decision={}",
        replay.world.world_ref,
        replay.run.run_ref,
        replay.run.summary.choice_records.len(),
        replay.observations.len(),
        replay.run.summary.decision.as_str(),
    );
    Ok(())
}

pub(super) fn shrink(out: PathBuf) -> Result<()> {
    let fixture = molten::fabric_simulation::run_reference_shrink_fixture()?;
    let plan = plan_shrink_artifacts(&fixture)?;
    write_artifacts(&out, &plan)?;
    println!(
        "fabric-simulation shrink ok original={} shrunk={} receipt={} attempts={} removed-workload={} failure-preserved={} out={}",
        fixture.original_world.world_ref,
        fixture.shrunk_world.world_ref,
        fixture.shrink.shrink_ref,
        fixture.shrink.result.attempts,
        fixture.shrink.result.removed_workload_steps,
        fixture.shrink.result.failure_preserved,
        out.display(),
    );
    Ok(())
}

pub(super) fn inspect(report: PathBuf) -> Result<()> {
    let value = read_report(&report)?;
    let readback = molten::fabric_simulation::parse_simulation_run_readback(&value)?;
    println!(
        "fabric-simulation report ok profile={} decision={} world={} choices={} events={} invariants={} resources={} virtual-ticks={} final-states={} divergence={} run={}",
        readback.profile,
        readback.decision,
        readback.world_ref,
        readback.choice_count,
        readback.event_count,
        readback.invariant_count,
        readback.resource_units,
        readback.virtual_ticks,
        readback.final_state_refs.len(),
        readback.first_divergence.as_deref().unwrap_or("none"),
        readback.run_ref,
    );
    Ok(())
}

pub(super) fn export(out: PathBuf) -> Result<()> {
    let fixture = molten::fabric_simulation::run_reference_simulation_fixture()?;
    let plan = plan_export_artifacts(&fixture)?;
    write_artifacts(&out, &plan)?;
    println!(
        "fabric-simulation export ok world={} run={} bundle={} differential={} artifacts={} out={}",
        fixture.world.world_ref,
        fixture.run.run_ref,
        fixture.bundle.bundle_ref,
        fixture.differential.report_ref,
        plan.len(),
        out.display(),
    );
    Ok(())
}

fn plan_run_artifacts(fixture: &ReferenceSimulationFixtureRun) -> Result<Vec<PlannedArtifact>> {
    let artifact_count = fixture
        .observations
        .len()
        .checked_add(fixture.port_events.len())
        .and_then(|count| count.checked_add(RUN_FIXED_ARTIFACT_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("fabric-simulation artifact count overflow"))?;
    if artifact_count > MAX_ARTIFACTS {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation artifact count {artifact_count} exceeds {MAX_ARTIFACTS}"
        )));
    }
    let mut artifacts = vec![
        world_artifact("world.preserves", &fixture.world)?,
        run_artifact("report.preserves", &fixture.run)?,
        bundle_artifact("bundle.preserves", &fixture.bundle)?,
        differential_artifact("differential.preserves", &fixture.differential)?,
    ];
    for (index, observation) in fixture.observations.iter().enumerate() {
        let path = format!("observations/{index:0width$}.preserves", width = ARTIFACT_INDEX_WIDTH);
        artifacts.push(observation_artifact(path, observation)?);
    }
    for (index, event) in fixture.port_events.iter().enumerate() {
        let path =
            format!("port-events/{index:0width$}-{}.preserves", event.class.as_str(), width = ARTIFACT_INDEX_WIDTH);
        artifacts.push(port_event_artifact(path, event)?);
    }
    validate_artifact_plan(&artifacts)?;
    Ok(artifacts)
}

fn plan_export_artifacts(fixture: &ReferenceSimulationFixtureRun) -> Result<Vec<PlannedArtifact>> {
    let artifacts = vec![
        world_artifact("world.preserves", &fixture.world)?,
        run_artifact("report.preserves", &fixture.run)?,
        bundle_artifact("bundle.preserves", &fixture.bundle)?,
        differential_artifact("differential.preserves", &fixture.differential)?,
    ];
    if artifacts.len() != EXPORT_ARTIFACT_COUNT {
        return Err(MoltenError::invalid_harness("fabric-simulation export artifact count drifted"));
    }
    validate_artifact_plan(&artifacts)?;
    Ok(artifacts)
}

fn plan_shrink_artifacts(fixture: &ReferenceShrinkFixture) -> Result<Vec<PlannedArtifact>> {
    let artifacts = vec![
        world_artifact("original-world.preserves", &fixture.original_world)?,
        world_artifact("shrunk-world.preserves", &fixture.shrunk_world)?,
        shrink_artifact("shrink.preserves", &fixture.shrink)?,
    ];
    if artifacts.len() != SHRINK_ARTIFACT_COUNT {
        return Err(MoltenError::invalid_harness("fabric-simulation shrink artifact count drifted"));
    }
    validate_artifact_plan(&artifacts)?;
    Ok(artifacts)
}

fn world_artifact(path: impl Into<PathBuf>, world: &CanonicalSimulatedWorld) -> Result<PlannedArtifact> {
    planned(path, &world.value)
}

fn run_artifact(path: impl Into<PathBuf>, run: &CanonicalSimulationRun) -> Result<PlannedArtifact> {
    planned(path, &run.value)
}

fn bundle_artifact(path: impl Into<PathBuf>, bundle: &CanonicalSimulationReproBundle) -> Result<PlannedArtifact> {
    planned(path, &bundle.value)
}

fn differential_artifact(
    path: impl Into<PathBuf>,
    differential: &CanonicalSimulationDifferential,
) -> Result<PlannedArtifact> {
    planned(path, &differential.value)
}

fn observation_artifact(
    path: impl Into<PathBuf>,
    observation: &CanonicalSimulationObservation,
) -> Result<PlannedArtifact> {
    planned(path, &observation.value)
}

fn port_event_artifact(path: impl Into<PathBuf>, event: &CanonicalSimulationPortEvent) -> Result<PlannedArtifact> {
    planned(path, &event.value)
}

fn shrink_artifact(path: impl Into<PathBuf>, shrink: &CanonicalSimulationShrink) -> Result<PlannedArtifact> {
    planned(path, &shrink.value)
}

fn planned(path: impl Into<PathBuf>, value: &preserves::IOValue) -> Result<PlannedArtifact> {
    let content = molten::preserves_rail::to_text(value)?;
    if content.len() > MAX_ARTIFACT_CONTENT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation artifact is {} bytes; maximum is {MAX_ARTIFACT_CONTENT_BYTES}",
            content.len()
        )));
    }
    Ok(PlannedArtifact {
        relative_path: path.into(),
        content,
    })
}

fn validate_artifact_plan(artifacts: &[PlannedArtifact]) -> Result<()> {
    if artifacts.len() > MAX_ARTIFACTS {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation artifact plan exceeds {MAX_ARTIFACTS} entries"
        )));
    }
    let mut paths = std::collections::BTreeSet::new();
    for artifact in artifacts {
        validate_relative_artifact_path(&artifact.relative_path)?;
        if !paths.insert(artifact.relative_path.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate fabric-simulation artifact path {}",
                artifact.relative_path.display()
            )));
        }
    }
    Ok(())
}

fn validate_relative_artifact_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(MoltenError::invalid_harness("fabric-simulation artifact path must be non-empty and relative"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation artifact path escapes output root: {}",
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

fn read_report(path: &Path) -> Result<preserves::IOValue> {
    let metadata = std::fs::metadata(path).map_err(MoltenError::from)?;
    if metadata.len() > MAX_REPORT_ARTIFACT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "fabric-simulation report is {} bytes; maximum is {MAX_REPORT_ARTIFACT_BYTES}",
            metadata.len()
        )));
    }
    let source = std::fs::read_to_string(path).map_err(MoltenError::from)?;
    molten::preserves_rail::parse_text(&source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_and_export_plans_are_bounded_relative_and_secret_free() {
        let fixture = molten::fabric_simulation::run_reference_simulation_fixture().expect("fixture");
        let run = plan_run_artifacts(&fixture).expect("run plan");
        let export = plan_export_artifacts(&fixture).expect("export plan");

        assert_eq!(run.len(), fixture.observations.len() + fixture.port_events.len() + RUN_FIXED_ARTIFACT_COUNT);
        assert_eq!(export.len(), EXPORT_ARTIFACT_COUNT);
        assert!(run.iter().all(|artifact| artifact.relative_path.is_relative()));
        assert!(run.iter().all(|artifact| !artifact.content.contains("private-key")));
        assert!(run.iter().any(|artifact| artifact.relative_path == Path::new("report.preserves")));
        assert!(run.iter().any(|artifact| artifact.relative_path == Path::new("bundle.preserves")));
    }

    #[test]
    fn artifact_plan_rejects_parent_escape_and_duplicate_paths() {
        let escape =
            validate_relative_artifact_path(Path::new("../escape.preserves")).expect_err("parent escape must deny");
        let duplicate = vec![
            PlannedArtifact {
                relative_path: PathBuf::from("same.preserves"),
                content: "one".to_string(),
            },
            PlannedArtifact {
                relative_path: PathBuf::from("same.preserves"),
                content: "two".to_string(),
            },
        ];
        let duplicate_error = validate_artifact_plan(&duplicate).expect_err("duplicate path must deny");

        assert!(escape.to_string().contains("escapes output root"));
        assert!(duplicate_error.to_string().contains("duplicate fabric-simulation artifact path"));
    }

    #[test]
    fn shrink_plan_contains_valid_original_shrunk_and_receipt_artifacts() {
        let fixture = molten::fabric_simulation::run_reference_shrink_fixture().expect("shrink fixture");
        let plan = plan_shrink_artifacts(&fixture).expect("shrink plan");

        assert_eq!(plan.len(), SHRINK_ARTIFACT_COUNT);
        assert!(fixture.shrink.result.failure_preserved);
        assert!(fixture.shrink.result.removed_workload_steps > 0);
        assert!(plan.iter().any(|artifact| artifact.relative_path == Path::new("original-world.preserves")));
        assert!(plan.iter().any(|artifact| artifact.relative_path == Path::new("shrunk-world.preserves")));
        assert!(plan.iter().any(|artifact| artifact.relative_path == Path::new("shrink.preserves")));
    }
}
