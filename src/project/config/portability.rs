type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const CONFIG_PORTABILITY_SCHEMA: &str = "molten.project.config-portability-report.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const MAX_CONFIG_FILES: usize = 128;
const MAX_SOURCE_PINS: usize = 64;
const MAX_DIAGNOSTICS: usize = 4096;
const FLOATING_NIGHTLY_DOUBLE_QUOTED: &str = "channel = \"nightly\"";
const FLOATING_NIGHTLY_SINGLE_QUOTED: &str = "channel = 'nightly'";
const USER_HOME_PATH_MARKER: &str = "/home/";
const REVIEWED_HOME_PLACEHOLDER: &str = "/home/<user>/";
const PLACEHOLDER_BLAKE3_ZERO_PREFIX: &str = "blake3:000000";
const PLACEHOLDER_REF_MARKER: &str = "placeholder-ref";
const TODO_MARKER: &str = "TODO";
const RELEASE_SCOPE_MARKER: &str = "release";
const EVIDENCE_ONLY_CAVEAT: &str = "config portability receipts are authoring evidence only and do not grant runtime authority, policy, provenance, resource, transport, source-gate, retention, release, deployment, or execution trust";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFileRecord {
    pub path: String,
    pub contents: String,
    pub release_scoped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePinRecord {
    pub dependency: String,
    pub cargo_revision: String,
    pub nix_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPortabilityInput {
    pub files: Vec<ConfigFileRecord>,
    pub source_pins: Vec<SourcePinRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPortabilityReport {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub compared_source_pins: Vec<String>,
    pub report_ref: String,
    pub value: IoValue,
}

// r[impl molten.project.config_portability.relocatable_paths]
// r[impl molten.project.config_portability.toolchain_pin]
// r[impl molten.project.config_portability.git_source_pin_drift]
// r[impl molten.project.config_portability.config_lint]
// r[impl molten.project.config_portability.named_config_constants]
pub fn build_config_portability_report(input: &ConfigPortabilityInput) -> Result<ConfigPortabilityReport> {
    ensure_config_file_bound(input.files.len())?;
    ensure_source_pin_bound(input.source_pins.len())?;
    let mut diagnostics = Vec::new();
    for file in &input.files {
        validate_file_record(file)?;
        lint_config_file(file, &mut diagnostics);
    }
    let compared_source_pins = lint_source_pins(&input.source_pins, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = config_portability_value(input, decision, &diagnostics, &compared_source_pins)?;
    let report_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ConfigPortabilityReport {
        decision: decision.to_string(),
        diagnostics,
        compared_source_pins,
        report_ref,
        value,
    })
}

fn validate_file_record(file: &ConfigFileRecord) -> Result<()> {
    validate_text("config file path", &file.path)?;
    validate_text("config file contents", &file.contents)
}

fn lint_config_file(file: &ConfigFileRecord, diagnostics: &mut Vec<String>) {
    for (line_index, line) in file.contents.lines().enumerate() {
        let line_number = line_index + 1;
        if has_user_home_path(line) {
            diagnostics.push(format!("user-home-path:{}:{line_number}", file.path));
        }
        if line.contains(FLOATING_NIGHTLY_DOUBLE_QUOTED) || line.contains(FLOATING_NIGHTLY_SINGLE_QUOTED) {
            diagnostics.push(format!("floating-release-toolchain:{}:{line_number}", file.path));
        }
        if file.release_scoped && has_placeholder_release_ref(line) {
            diagnostics.push(format!("placeholder-release-ref:{}:{line_number}", file.path));
        }
    }
}

fn has_user_home_path(line: &str) -> bool {
    line.contains(USER_HOME_PATH_MARKER) && !line.contains(REVIEWED_HOME_PLACEHOLDER)
}

fn has_placeholder_release_ref(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains(PLACEHOLDER_BLAKE3_ZERO_PREFIX)
        || lower.contains(PLACEHOLDER_REF_MARKER)
        || (line.contains(RELEASE_SCOPE_MARKER) && line.contains(TODO_MARKER))
}

fn lint_source_pins(pins: &[SourcePinRecord], diagnostics: &mut Vec<String>) -> Result<Vec<String>> {
    let mut compared = OrderedSet::new();
    let mut seen = OrderedMap::new();
    for pin in pins {
        validate_source_pin(pin)?;
        if pin.cargo_revision != pin.nix_revision {
            diagnostics.push(format!(
                "source-pin-drift:{}:cargo={}:nix={}",
                pin.dependency, pin.cargo_revision, pin.nix_revision
            ));
        }
        compared.insert(format!("{}@{}", pin.dependency, pin.cargo_revision));
        if let Some(previous) = seen.insert(pin.dependency.clone(), pin.cargo_revision.clone())
            && previous != pin.cargo_revision
        {
            diagnostics
                .push(format!("conflicting-cargo-source-pin:{}:{}:{}", pin.dependency, previous, pin.cargo_revision));
        }
    }
    Ok(compared.into_iter().collect())
}

fn validate_source_pin(pin: &SourcePinRecord) -> Result<()> {
    validate_text("source pin dependency", &pin.dependency)?;
    validate_text("source pin cargo revision", &pin.cargo_revision)?;
    validate_text("source pin nix revision", &pin.nix_revision)
}

fn config_portability_value(
    input: &ConfigPortabilityInput,
    decision: &str,
    diagnostics: &[String],
    compared_source_pins: &[String],
) -> Result<IoValue> {
    Ok(record("config-portability-report-v1", vec![
        string(CONFIG_PORTABILITY_SCHEMA),
        field_string("decision", decision),
        field_sequence("files", file_record_values(&input.files)?),
        field_sequence("source-pins", source_pin_values(&input.source_pins)?),
        field_sequence("compared-source-pins", string_values(compared_source_pins)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn file_record_values(files: &[ConfigFileRecord]) -> Result<Vec<IoValue>> {
    files
        .iter()
        .map(|file| {
            Ok(record("file", vec![
                field_string("path", &file.path),
                field_string("contents-ref", &crate::preserves_rail::content_ref_from_bytes(file.contents.as_bytes())),
                record("release-scoped", vec![bool_value(file.release_scoped)]),
            ]))
        })
        .collect()
}

fn source_pin_values(pins: &[SourcePinRecord]) -> Result<Vec<IoValue>> {
    pins.iter()
        .map(|pin| {
            Ok(record("source-pin", vec![
                field_string("dependency", &pin.dependency),
                field_string("cargo-revision", &pin.cargo_revision),
                field_string("nix-revision", &pin.nix_revision),
            ]))
        })
        .collect()
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_config_file_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_CONFIG_FILES, "config files")
}

fn ensure_source_pin_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_SOURCE_PINS, "source pins")
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "config diagnostics")
}

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
