
fn validate_relative_artifact_path(path: &std::path::Path) -> molten::error::Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(molten::error::MoltenError::invalid_harness(
            "fabric-simulation artifact path must be non-empty and relative",
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_)
        )
    }) {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "fabric-simulation artifact path escapes output root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn write_artifacts(root: &std::path::Path, artifacts: &[PlannedArtifact]) -> molten::error::Result<()> {
    std::fs::create_dir_all(root).map_err(molten::error::MoltenError::from)?;
    for artifact in artifacts {
        let path = root.join(&artifact.relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
        }
        std::fs::write(path, artifact.content.as_bytes()).map_err(molten::error::MoltenError::from)?;
    }
    Ok(())
}

fn read_report(path: &std::path::Path) -> molten::error::Result<preserves::IOValue> {
    let metadata = std::fs::metadata(path).map_err(molten::error::MoltenError::from)?;
    if metadata.len() > MAX_REPORT_ARTIFACT_BYTES {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "fabric-simulation report is {} bytes; maximum is {MAX_REPORT_ARTIFACT_BYTES}",
            metadata.len()
        )));
    }
    let source = std::fs::read_to_string(path).map_err(molten::error::MoltenError::from)?;
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
        assert!(run.iter().any(|artifact| artifact.relative_path == std::path::Path::new("report.preserves")));
        assert!(run.iter().any(|artifact| artifact.relative_path == std::path::Path::new("bundle.preserves")));
    }

    #[test]
    fn artifact_plan_rejects_parent_escape_and_duplicate_paths() {
        let escape = validate_relative_artifact_path(std::path::Path::new("../escape.preserves"))
            .expect_err("parent escape must deny");
        let duplicate = vec![
            PlannedArtifact {
                relative_path: std::path::PathBuf::from("same.preserves"),
                content: "one".to_string(),
            },
            PlannedArtifact {
                relative_path: std::path::PathBuf::from("same.preserves"),
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
        assert!(
            plan.iter()
                .any(|artifact| artifact.relative_path == std::path::Path::new("original-world.preserves"))
        );
        assert!(plan.iter().any(|artifact| artifact.relative_path == std::path::Path::new("shrunk-world.preserves")));
        assert!(plan.iter().any(|artifact| artifact.relative_path == std::path::Path::new("shrink.preserves")));
    }
}
