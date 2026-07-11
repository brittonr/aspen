use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::system_extension::ExecutableSystemExtensionFixtureRun;
use molten::system_extension::ExecutionProfile;
use molten::system_extension::HostEvidence;

const ARTIFACT_INDEX_WIDTH: usize = 3;
const MAX_STATUS_ARTIFACT_BYTES: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedArtifact {
    relative_path: PathBuf,
    content: String,
}

pub(super) fn run_fixture(profile: ExecutionProfile, out: PathBuf) -> Result<()> {
    let run = molten::system_extension::run_executable_system_extension_fixture(profile)?;
    let plan = plan_fixture_artifacts(&run)?;
    write_artifacts(&out, &plan)?;
    println!(
        "system-extension fixture ok profile={} manifest={} evidence={} status={} out={}",
        profile.as_str(),
        run.manifest_ref,
        run.evidence.len(),
        run.final_status.status_ref,
        out.display()
    );
    Ok(())
}

pub(super) fn show(status: PathBuf) -> Result<()> {
    let metadata = std::fs::metadata(&status).map_err(MoltenError::from)?;
    if metadata.len() > MAX_STATUS_ARTIFACT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "system-extension status artifact is {} bytes; maximum is {MAX_STATUS_ARTIFACT_BYTES}",
            metadata.len()
        )));
    }
    let source = std::fs::read_to_string(&status).map_err(MoltenError::from)?;
    let value = molten::preserves_rail::parse_text(&source)?;
    let readback = molten::system_extension::parse_operator_status_readback(&value)?;
    println!(
        "system-extension status extension={} service={} generation={} phase={} profile={} health={} restarts={} invocations={} checkpoint={} ref={}",
        readback.extension_id,
        readback.service_id,
        readback.generation,
        readback.phase,
        readback.execution_profile,
        readback.health,
        readback.restart_attempts,
        readback.invocation_count,
        readback.checkpoint_ref.as_deref().unwrap_or("none"),
        readback.status_ref,
    );
    Ok(())
}

fn plan_fixture_artifacts(run: &ExecutableSystemExtensionFixtureRun) -> Result<Vec<PlannedArtifact>> {
    let mut artifacts = vec![
        planned("manifest.preserves", &run.manifest_value)?,
        planned("upgraded-status.preserves", &run.upgraded_status.value)?,
        planned("rolled-back-status.preserves", &run.rolled_back_status.value)?,
        planned("recovered-status.preserves", &run.recovered_status.value)?,
        planned("status.preserves", &run.final_status.value)?,
    ];
    for (index, evidence) in run.evidence.iter().enumerate() {
        let (kind, value) = match evidence {
            HostEvidence::Lifecycle(receipt) => ("lifecycle", &receipt.value),
            HostEvidence::Callback(receipt) => ("callback", &receipt.value),
            HostEvidence::EffectCompletion(receipt) => ("effect-completion", &receipt.value),
            HostEvidence::Migration(receipt) => ("migration", &receipt.value),
            HostEvidence::Readiness(receipt) => ("readiness", &receipt.value),
        };
        let filename = format!("evidence/{index:0width$}-{kind}.preserves", width = ARTIFACT_INDEX_WIDTH);
        artifacts.push(planned(filename, value)?);
    }
    Ok(artifacts)
}

fn planned(relative_path: impl Into<PathBuf>, value: &preserves::IOValue) -> Result<PlannedArtifact> {
    Ok(PlannedArtifact {
        relative_path: relative_path.into(),
        content: molten::preserves_rail::to_text(value)?,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    const FIXED_ARTIFACT_COUNT: usize = 5;

    // r[verify molten.system_extension.operator_readback]
    #[test]
    fn fixture_plan_has_bounded_relative_paths_and_a_status_artifact() {
        let run =
            molten::system_extension::run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent)
                .expect("fixture run");
        let plan = plan_fixture_artifacts(&run).expect("artifact plan");

        assert!(plan.iter().any(|artifact| artifact.relative_path == Path::new("status.preserves")));
        assert!(plan.iter().all(|artifact| artifact.relative_path.is_relative()));
        assert!(plan.iter().all(|artifact| {
            !artifact.relative_path.components().any(|component| component == std::path::Component::ParentDir)
        }));
        assert_eq!(plan.len(), run.evidence.len() + FIXED_ARTIFACT_COUNT);
    }

    // r[verify molten.system_extension.operator_readback]
    #[test]
    fn status_parser_rejects_callback_receipts_as_operator_status() {
        let run =
            molten::system_extension::run_executable_system_extension_fixture(ExecutionProfile::SandboxedComponent)
                .expect("fixture run");
        let callback = run
            .evidence
            .iter()
            .find_map(|evidence| match evidence {
                HostEvidence::Callback(receipt) => Some(&receipt.value),
                HostEvidence::EffectCompletion(_)
                | HostEvidence::Lifecycle(_)
                | HostEvidence::Migration(_)
                | HostEvidence::Readiness(_) => None,
            })
            .expect("callback receipt");

        let error = molten::system_extension::parse_operator_status_readback(callback)
            .expect_err("callback receipt is not a status artifact");
        assert!(error.to_string().contains("canonical system-extension status"));
    }
}
