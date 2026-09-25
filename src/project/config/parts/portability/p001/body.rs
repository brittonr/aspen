
#[cfg(test)]
mod tests {
    use super::*;

    const MATCHING_REVISION: &str = "89675cd4f585f837323c049e4a25f7b94c903038";
    const DRIFT_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn file(path: &str, contents: &str, release_scoped: bool) -> ConfigFileRecord {
        ConfigFileRecord {
            path: path.to_string(),
            contents: contents.to_string(),
            release_scoped,
        }
    }

    fn pin(dependency: &str, cargo_revision: &str, nix_revision: &str) -> SourcePinRecord {
        SourcePinRecord {
            dependency: dependency.to_string(),
            cargo_revision: cargo_revision.to_string(),
            nix_revision: nix_revision.to_string(),
        }
    }

    // r[verify molten.project.config_portability.relocatable_paths]
    // r[verify molten.project.config_portability.toolchain_pin]
    // r[verify molten.project.config_portability.git_source_pin_drift]
    // r[verify molten.project.config_portability.config_lint]
    // r[verify molten.project.config_portability.named_config_constants]
    #[test]
    fn config_portability_accepts_relocatable_pinned_inputs() {
        let report = build_config_portability_report(&ConfigPortabilityInput {
            files: vec![
                file("flake.nix", "url = \"path:../cairn\"\nprofile_block_context=16", true),
                file("rust-toolchain.toml", "channel = \"nightly-2026-05-26\"", true),
            ],
            source_pins: vec![pin("basalt", MATCHING_REVISION, MATCHING_REVISION)],
        })
        .expect("config report");
        assert_eq!(report.decision, DECISION_PASS);
        assert_eq!(report.compared_source_pins, vec![format!("basalt@{MATCHING_REVISION}")]);
    }

    #[test]
    fn config_portability_denies_home_paths_floating_toolchain_and_pin_drift() {
        let report = build_config_portability_report(&ConfigPortabilityInput {
            files: vec![
                file(".pre-commit-config.yaml", "path:/home/brittonr/git/cairn", true),
                file("rust-toolchain.toml", "channel = \"nightly\"", true),
                file("release-profile.ncl", "release_ref = \"blake3:000000000000\"", true),
            ],
            source_pins: vec![pin("basalt", MATCHING_REVISION, DRIFT_REVISION)],
        })
        .expect("config report");
        assert_eq!(report.decision, DECISION_DENY);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("user-home-path:.pre-commit-config.yaml"))
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("floating-release-toolchain:rust-toolchain.toml"))
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("placeholder-release-ref:release-profile.ncl"))
        );
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("source-pin-drift:basalt")));
    }
}
