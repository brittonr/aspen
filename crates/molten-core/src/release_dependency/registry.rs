use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::DependencyPlane;
use super::DiagnosticCode;
use super::ManifestTable;
use super::RegistryDependencyExpectation;
use super::RegistryDependencyObservation;
use super::ReleaseDependencyDiagnostic;
use super::ReleaseDependencyInput;
use super::ReleaseDisposition;
use super::SourceKind;
use super::TransportPolicy;
use super::compare_required;
use super::push_diagnostic;

const CRATES_IO_SOURCE_COORDINATE: &str = "registry+https://github.com/rust-lang/crates.io-index";
const SHA256_HEX_LENGTH: usize = 64;
const EXACT_VERSION_OPERATOR: &str = "=";

// r[impl aspen.marble_store.pinning]
pub(super) fn validate_registry_rows(
    input: &ReleaseDependencyInput,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    let observations = registry_observation_index(&input.registry_observations, diagnostics);
    let git_dependencies: BTreeSet<&str> =
        input.dependencies.iter().map(|row| row.manifest_dependency.as_str()).collect();
    let mut registry_dependencies = BTreeSet::new();

    for expectation in &input.registry_dependencies {
        validate_registry_expectation(expectation, diagnostics);
        let manifest_dependency = expectation.manifest_dependency.as_str();
        if git_dependencies.contains(manifest_dependency) {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::DuplicateDependency,
                manifest_dependency,
                "manifest dependency is declared by both a Git row and a registry row",
            );
        }
        if !registry_dependencies.insert(manifest_dependency) {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::DuplicateDependency,
                manifest_dependency,
                "registry dependency appears more than once in the release profile",
            );
        }
        let Some(observation) = observations.get(manifest_dependency) else {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::MissingSourceRow,
                manifest_dependency,
                "manifest and lock observations are missing for the registry pin",
            );
            continue;
        };
        validate_registry_observation(expectation, observation, diagnostics);
    }
}

fn registry_observation_index<'a>(
    observations: &'a [RegistryDependencyObservation],
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) -> BTreeMap<&'a str, &'a RegistryDependencyObservation> {
    let mut index = BTreeMap::new();
    for observation in observations {
        if index.insert(observation.manifest_dependency.as_str(), observation).is_some() {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::DuplicateDependency,
                &observation.manifest_dependency,
                "registry dependency observation appears more than once",
            );
        }
    }
    index
}

fn validate_registry_expectation(
    expectation: &RegistryDependencyExpectation,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    if expectation.manifest_dependency.trim().is_empty()
        || expectation.package_name.trim().is_empty()
        || expectation.package_version.trim().is_empty()
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            "registry row contains an empty required field",
        );
    }
    if expectation.source_kind != SourceKind::CratesIo
        || expectation.transport_policy != TransportPolicy::CratesIo
        || expectation.source_coordinate != CRATES_IO_SOURCE_COORDINATE
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            format!("registry row must use crates.io source kind, transport, and {CRATES_IO_SOURCE_COORDINATE}"),
        );
    }
    if !is_sha256_hex(&expectation.checksum) {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::FloatingRevision,
            &expectation.manifest_dependency,
            format!("registry checksum must be {SHA256_HEX_LENGTH} lowercase hexadecimal characters"),
        );
    }
    let is_development_plane = expectation.plane == DependencyPlane::Development;
    let is_development_disposition = expectation.disposition == ReleaseDisposition::Development;
    if is_development_plane != is_development_disposition {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            format!(
                "registry plane {:?} disagrees with release disposition {:?}",
                expectation.plane, expectation.disposition
            ),
        );
    }
}

fn validate_registry_observation(
    expectation: &RegistryDependencyExpectation,
    observation: &RegistryDependencyObservation,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    let subject = expectation.manifest_dependency.as_str();
    let expected_table = plane_manifest_table(expectation.plane);
    match observation.manifest_table {
        Some(table) if table == expected_table => {}
        Some(table) => push_diagnostic(
            diagnostics,
            DiagnosticCode::PlaneDrift,
            subject,
            format!("{:?} plane requires manifest table {expected_table:?}, observed {table:?}", expectation.plane),
        ),
        None => push_diagnostic(
            diagnostics,
            DiagnosticCode::ManifestDrift,
            subject,
            "registry dependency is absent from the root manifest",
        ),
    }
    compare_required(
        DiagnosticCode::FloatingVersion,
        subject,
        "manifest version requirement",
        &format!("{EXACT_VERSION_OPERATOR}{}", expectation.package_version),
        observation.manifest_version_requirement.as_deref(),
        diagnostics,
    );
    let is_expected_optional = expectation.disposition == ReleaseDisposition::OptionalRuntime;
    if observation.is_manifest_optional != is_expected_optional {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::ManifestDrift,
            subject,
            format!(
                "manifest optional flag is {}, release disposition {:?} requires {is_expected_optional}",
                observation.is_manifest_optional, expectation.disposition
            ),
        );
    }
    compare_required(
        DiagnosticCode::LockDrift,
        subject,
        "lock package version",
        &expectation.package_version,
        observation.lock_package_version.as_deref(),
        diagnostics,
    );
    compare_required(
        DiagnosticCode::LockDrift,
        subject,
        "lock source coordinate",
        &expectation.source_coordinate,
        observation.lock_source_coordinate.as_deref(),
        diagnostics,
    );
    compare_required(
        DiagnosticCode::LockDrift,
        subject,
        "lock checksum",
        &expectation.checksum,
        observation.lock_checksum.as_deref(),
        diagnostics,
    );
}

fn plane_manifest_table(plane: DependencyPlane) -> ManifestTable {
    match plane {
        DependencyPlane::Implementation => ManifestTable::Dependencies,
        DependencyPlane::Development => ManifestTable::DevDependencies,
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn registry_material_rows(input: &ReleaseDependencyInput) -> Vec<String> {
    let mut rows = Vec::with_capacity(input.registry_dependencies.len() + input.registry_observations.len());
    rows.extend(input.registry_dependencies.iter().map(|row| {
        format!(
            "registry\t{}\t{}\t{}\t{:?}\t{}\t{}\t{:?}\t{:?}\t{:?}",
            row.manifest_dependency,
            row.package_name,
            row.package_version,
            row.source_kind,
            row.source_coordinate,
            row.checksum,
            row.transport_policy,
            row.plane,
            row.disposition
        )
    }));
    rows.extend(input.registry_observations.iter().map(|row| {
        format!(
            "registry-observation\t{}\t{:?}\t{:?}\t{}\t{:?}\t{:?}\t{:?}",
            row.manifest_dependency,
            row.manifest_table,
            row.manifest_version_requirement,
            row.is_manifest_optional,
            row.lock_package_version,
            row.lock_source_coordinate,
            row.lock_checksum
        )
    }));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_dependency::MAX_REGISTRY_ROWS;
    use crate::release_dependency::tests::valid_input;
    use crate::release_dependency::validate_release_dependencies;

    const ART_CHECKSUM: &str = "41e7ba0ded1ee3730fe4445fbdf4754756b2edd86617f19507c89d3b71383277";
    const MARBLE_CHECKSUM: &str = "c00d04008994fed635a52323bbe1dd8c0947292183585ddf8f1090615fafc3c6";
    const DRIFT_CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn registry_row(name: &str, version: &str, checksum: &str) -> RegistryDependencyExpectation {
        RegistryDependencyExpectation {
            manifest_dependency: name.to_owned(),
            package_name: name.to_owned(),
            package_version: version.to_owned(),
            source_kind: SourceKind::CratesIo,
            source_coordinate: CRATES_IO_SOURCE_COORDINATE.to_owned(),
            checksum: checksum.to_owned(),
            transport_policy: TransportPolicy::CratesIo,
            plane: DependencyPlane::Implementation,
            disposition: ReleaseDisposition::OptionalRuntime,
        }
    }

    fn registry_observation(expectation: &RegistryDependencyExpectation) -> RegistryDependencyObservation {
        RegistryDependencyObservation {
            manifest_dependency: expectation.manifest_dependency.clone(),
            manifest_table: Some(plane_manifest_table(expectation.plane)),
            manifest_version_requirement: Some(format!("={}", expectation.package_version)),
            is_manifest_optional: expectation.disposition == ReleaseDisposition::OptionalRuntime,
            lock_package_version: Some(expectation.package_version.clone()),
            lock_source_coordinate: Some(expectation.source_coordinate.clone()),
            lock_checksum: Some(expectation.checksum.clone()),
        }
    }

    /// Valid input with the reviewed art and marble pins at indices 0 and 1.
    fn marble_input() -> ReleaseDependencyInput {
        let art = registry_row("art", "1.0.0", ART_CHECKSUM);
        let marble = registry_row("marble", "16.0.2", MARBLE_CHECKSUM);
        let mut input = valid_input();
        input.registry_observations = vec![registry_observation(&marble), registry_observation(&art)];
        input.registry_dependencies = vec![art, marble];
        input
    }

    fn has_code(input: &ReleaseDependencyInput, code: DiagnosticCode) -> bool {
        validate_release_dependencies(input).diagnostics.iter().any(|item| item.code == code)
    }

    // r[verify aspen.marble_store.pinning]
    #[test]
    fn marble_and_art_pins_classify_onto_implementation_edges() {
        let report = validate_release_dependencies(&marble_input());

        assert_eq!(report.diagnostics, Vec::new());
        for (name, version, checksum) in [("art", "1.0.0", ART_CHECKSUM), ("marble", "16.0.2", MARBLE_CHECKSUM)] {
            let identity = format!("registry\t{name}\t{name}\t{version}\tCratesIo\t{CRATES_IO_SOURCE_COORDINATE}");
            let edge = format!("{identity}\t{checksum}\tCratesIo\tImplementation\tOptionalRuntime\n");
            assert!(report.canonical_material.contains(&edge), "missing implementation edge for {name}");
        }

        let mut reordered = marble_input();
        reordered.registry_dependencies.reverse();
        reordered.registry_observations.reverse();
        assert_eq!(validate_release_dependencies(&reordered).canonical_material, report.canonical_material);
    }

    #[test]
    fn development_plane_pin_on_dev_dependencies_passes() {
        let mut input = marble_input();
        input.registry_dependencies[0].plane = DependencyPlane::Development;
        input.registry_dependencies[0].disposition = ReleaseDisposition::Development;
        input.registry_observations[1] = registry_observation(&input.registry_dependencies[0]);

        let report = validate_release_dependencies(&input);
        assert_eq!(report.diagnostics, Vec::new());
        assert!(report.canonical_material.contains("\tDevelopment\tDevelopment\n"));
    }

    #[test]
    fn floating_manifest_requirement_fails_closed() {
        for requirement in [Some("16.0.2"), Some("^16.0.2"), Some("=16.0.3"), None] {
            let mut input = marble_input();
            input.registry_observations[0].manifest_version_requirement = requirement.map(str::to_owned);
            assert!(has_code(&input, DiagnosticCode::FloatingVersion), "requirement {requirement:?} admitted");
        }
    }

    #[test]
    fn manifest_table_outside_the_plane_fails_closed() {
        for table in [ManifestTable::DevDependencies, ManifestTable::BuildDependencies] {
            let mut input = marble_input();
            input.registry_observations[0].manifest_table = Some(table);
            assert!(has_code(&input, DiagnosticCode::PlaneDrift), "table {table:?} admitted");
        }

        let mut development = marble_input();
        development.registry_dependencies[1].plane = DependencyPlane::Development;
        development.registry_dependencies[1].disposition = ReleaseDisposition::Development;
        development.registry_observations[0].is_manifest_optional = false;
        assert!(has_code(&development, DiagnosticCode::PlaneDrift));
    }

    #[test]
    fn absent_manifest_entry_and_optional_flag_drift_fail_closed() {
        let mut absent = marble_input();
        absent.registry_observations[0].manifest_table = None;
        assert!(has_code(&absent, DiagnosticCode::ManifestDrift));

        let mut required = marble_input();
        required.registry_observations[0].is_manifest_optional = false;
        assert!(has_code(&required, DiagnosticCode::ManifestDrift));

        let mut runtime = marble_input();
        runtime.registry_dependencies[1].disposition = ReleaseDisposition::Runtime;
        assert!(has_code(&runtime, DiagnosticCode::ManifestDrift));
    }

    #[test]
    fn lock_version_source_and_checksum_drift_fail_closed() {
        let mut version = marble_input();
        version.registry_observations[0].lock_package_version = Some("16.0.3".to_owned());
        assert!(has_code(&version, DiagnosticCode::LockDrift));

        let mut source = marble_input();
        source.registry_observations[0].lock_source_coordinate = Some("registry+https://example.com/index".to_owned());
        assert!(has_code(&source, DiagnosticCode::LockDrift));

        let mut checksum = marble_input();
        checksum.registry_observations[0].lock_checksum = Some(DRIFT_CHECKSUM.to_owned());
        assert!(has_code(&checksum, DiagnosticCode::LockDrift));

        let mut unlocked = marble_input();
        unlocked.registry_observations[0].lock_checksum = None;
        assert!(has_code(&unlocked, DiagnosticCode::LockDrift));
    }

    #[test]
    fn missing_registry_observation_fails_closed() {
        let mut input = marble_input();
        input.registry_observations.remove(0);
        let report = validate_release_dependencies(&input);

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| { item.code == DiagnosticCode::MissingSourceRow && item.subject == "marble" })
        );
    }

    #[test]
    fn malformed_checksum_fails_closed() {
        let uppercase = MARBLE_CHECKSUM.to_ascii_uppercase();
        for checksum in ["latest", &MARBLE_CHECKSUM[1..], uppercase.as_str()] {
            let mut input = marble_input();
            input.registry_dependencies[1].checksum = checksum.to_owned();
            input.registry_observations[0].lock_checksum = Some(checksum.to_owned());
            assert!(has_code(&input, DiagnosticCode::FloatingRevision), "checksum {checksum} admitted");
        }
    }

    #[test]
    fn non_crates_io_source_or_plane_disposition_mismatch_fails_closed() {
        let mut transport = marble_input();
        transport.registry_dependencies[1].transport_policy = TransportPolicy::Https;
        assert!(has_code(&transport, DiagnosticCode::InvalidSourceRow));

        let mut kind = marble_input();
        kind.registry_dependencies[1].source_kind = SourceKind::Git;
        assert!(has_code(&kind, DiagnosticCode::InvalidSourceRow));

        let mut mismatch = marble_input();
        mismatch.registry_dependencies[1].disposition = ReleaseDisposition::Development;
        mismatch.registry_observations[0].is_manifest_optional = false;
        assert!(has_code(&mismatch, DiagnosticCode::InvalidSourceRow));
    }

    #[test]
    fn duplicate_registry_rows_observations_and_git_collisions_fail_closed() {
        let mut row = marble_input();
        row.registry_dependencies.push(row.registry_dependencies[1].clone());
        assert!(has_code(&row, DiagnosticCode::DuplicateDependency));

        let mut observation = marble_input();
        observation.registry_observations.push(observation.registry_observations[0].clone());
        assert!(has_code(&observation, DiagnosticCode::DuplicateDependency));

        let mut collision = marble_input();
        let git_name = collision.dependencies[0].manifest_dependency.clone();
        collision.registry_dependencies[1].manifest_dependency = git_name.clone();
        collision.registry_observations[0].manifest_dependency = git_name;
        let report = validate_release_dependencies(&collision);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| { item.code == DiagnosticCode::DuplicateDependency && item.subject == "basalt" })
        );
    }

    #[test]
    fn registry_row_count_is_bounded() {
        let mut input = marble_input();
        let template = input.registry_dependencies[0].clone();
        input.registry_dependencies = (0..=MAX_REGISTRY_ROWS)
            .map(|index| {
                let mut row = template.clone();
                row.manifest_dependency = format!("registry-{index}");
                row
            })
            .collect();
        let report = validate_release_dependencies(&input);

        assert!(
            report.diagnostics.iter().any(|item| {
                item.code == DiagnosticCode::BoundExceeded && item.subject == "registry dependency rows"
            })
        );
    }
}
