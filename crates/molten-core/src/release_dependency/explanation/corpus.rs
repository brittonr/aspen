//! Focused fixture corpus for the issue #11 explanation comparison.
//!
//! Base fixture (`valid_input`): Git expectations `[basalt, valence]`,
//! observations `[valence, basalt]`, archive receipts
//! `[valence-integrity, octet-cutover]`, archive observations
//! `[octet-cutover, valence-integrity]`.

use std::collections::BTreeSet;

use super::ExplanationRule;
use crate::release_dependency::DependencyPlane;
use crate::release_dependency::DiagnosticCode;
use crate::release_dependency::ManifestTable;
use crate::release_dependency::RegistryDependencyExpectation;
use crate::release_dependency::RegistryDependencyObservation;
use crate::release_dependency::ReleaseDependencyInput;
use crate::release_dependency::ReleaseDisposition;
use crate::release_dependency::SourceKind;
use crate::release_dependency::TransportPolicy;
use crate::release_dependency::tests::valid_input;
use crate::release_dependency::validate_release_dependencies;

pub(super) const BASALT: &str = "basalt";
pub(super) const VALENCE: &str = "valence";
pub(super) const BASALT_OBSERVATION: usize = 1;
pub(super) const VALENCE_OBSERVATION: usize = 0;
pub(super) const VALENCE_ARCHIVE: &str = "valence-integrity";
pub(super) const OCTET_ARCHIVE: &str = "octet-cutover";
pub(super) const OCTET_ARCHIVE_OBSERVATION: usize = 0;
pub(super) const VALENCE_ARCHIVE_OBSERVATION: usize = 1;
pub(super) const FORK_SOURCE: &str = "https://github.com/OnixResearch/basalt-fork.git";
pub(super) const DRIFT_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ART: &str = "art";
const ART_VERSION: &str = "1.0.0";
const ART_CHECKSUM: &str = "41e7ba0ded1ee3730fe4445fbdf4754756b2edd86617f19507c89d3b71383277";
const DRIFT_CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";

pub(super) struct CorpusCase {
    pub(super) name: &'static str,
    pub(super) snapshot: ReleaseDependencyInput,
    /// Expected `(rule, subject)` explanations, one per expectation occurrence.
    pub(super) expected: Vec<(ExplanationRule, &'static str)>,
}

fn case(
    name: &'static str,
    snapshot: ReleaseDependencyInput,
    expected: &[(ExplanationRule, &'static str)],
) -> CorpusCase {
    CorpusCase {
        name,
        snapshot,
        expected: expected.to_vec(),
    }
}

pub(super) fn corpus() -> Vec<CorpusCase> {
    use ExplanationRule::ArchiveObservationAbsent;
    use ExplanationRule::ArchiveReportedAbsent;
    use ExplanationRule::GitLockRevision;
    use ExplanationRule::GitLockSourceCoordinate;
    use ExplanationRule::RegistryLockChecksum;

    vec![
        case("valid", valid_input(), &[]),
        case("lock-source-drift", lock_source_drift(), &[(GitLockSourceCoordinate, BASALT)]),
        case("lock-revision-drift-and-missing", lock_revision_drift_and_missing(), &[
            (GitLockRevision, BASALT),
            (GitLockRevision, VALENCE),
        ]),
        case("absent-dependency-observation", absent_dependency_observation(), &[]),
        case("absent-archive-observation", absent_archive_observation(), &[(
            ArchiveObservationAbsent,
            OCTET_ARCHIVE,
        )]),
        case("archive-reported-absent", archive_reported_absent(), &[(ArchiveReportedAbsent, VALENCE_ARCHIVE)]),
        case("duplicate-observation-last-clean", duplicate_observation_last_clean(), &[]),
        case("duplicate-observation-last-drifted", duplicate_observation_last_drifted(), &[(
            GitLockRevision,
            BASALT,
        )]),
        case("duplicate-archive-observation-last-present", duplicate_archive_observation_last_present(), &[]),
        case("duplicate-expectation", duplicate_expectation(), &[
            (GitLockRevision, BASALT),
            (GitLockRevision, BASALT),
        ]),
        case("crossed-packages", crossed_packages(), &[
            (GitLockSourceCoordinate, BASALT),
            (GitLockRevision, BASALT),
            (GitLockSourceCoordinate, VALENCE),
            (GitLockRevision, VALENCE),
        ]),
        case("registry-lock-drift", registry_lock_drift(), &[(RegistryLockChecksum, ART)]),
    ]
}

pub(super) fn lock_source_drift() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.observations[BASALT_OBSERVATION].lock_source_coordinate = Some(FORK_SOURCE.to_owned());
    input
}

fn lock_revision_drift_and_missing() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.observations[BASALT_OBSERVATION].lock_revision = Some(DRIFT_REVISION.to_owned());
    input.observations[VALENCE_OBSERVATION].lock_revision = None;
    input
}

pub(super) fn absent_dependency_observation() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.observations.remove(BASALT_OBSERVATION);
    input
}

pub(super) fn absent_archive_observation() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.archive_observations.remove(OCTET_ARCHIVE_OBSERVATION);
    input
}

fn archive_reported_absent() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.archive_observations[VALENCE_ARCHIVE_OBSERVATION].archive_present = false;
    input
}

/// A drifted basalt observation precedes the clean one; the validator reads the last.
fn duplicate_observation_last_clean() -> ReleaseDependencyInput {
    let mut input = valid_input();
    let mut drifted = input.observations[BASALT_OBSERVATION].clone();
    drifted.lock_revision = Some(DRIFT_REVISION.to_owned());
    input.observations.insert(0, drifted);
    input
}

/// Observations `[valence, basalt (clean), basalt (drifted)]`; the drifted occurrence is effective.
pub(super) fn duplicate_observation_last_drifted() -> ReleaseDependencyInput {
    let mut input = valid_input();
    let mut drifted = input.observations[BASALT_OBSERVATION].clone();
    drifted.lock_revision = Some(DRIFT_REVISION.to_owned());
    input.observations.push(drifted);
    input
}

fn duplicate_archive_observation_last_present() -> ReleaseDependencyInput {
    let mut input = valid_input();
    let mut absent = input.archive_observations[VALENCE_ARCHIVE_OBSERVATION].clone();
    absent.archive_present = false;
    input.archive_observations.insert(0, absent);
    input
}

fn duplicate_expectation() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.dependencies.push(input.dependencies[0].clone());
    input.observations[BASALT_OBSERVATION].lock_revision = Some(DRIFT_REVISION.to_owned());
    input
}

/// Each observation carries the other package's lock source and revision.
pub(super) fn crossed_packages() -> ReleaseDependencyInput {
    let mut input = valid_input();
    let basalt = input.observations[BASALT_OBSERVATION].clone();
    let valence = input.observations[VALENCE_OBSERVATION].clone();
    input.observations[BASALT_OBSERVATION].lock_source_coordinate = valence.lock_source_coordinate;
    input.observations[BASALT_OBSERVATION].lock_revision = valence.lock_revision;
    input.observations[VALENCE_OBSERVATION].lock_source_coordinate = basalt.lock_source_coordinate;
    input.observations[VALENCE_OBSERVATION].lock_revision = basalt.lock_revision;
    input
}

fn registry_lock_drift() -> ReleaseDependencyInput {
    let mut input = valid_input();
    input.registry_dependencies = vec![RegistryDependencyExpectation {
        manifest_dependency: ART.to_owned(),
        package_name: ART.to_owned(),
        package_version: ART_VERSION.to_owned(),
        source_kind: SourceKind::CratesIo,
        source_coordinate: CRATES_IO_SOURCE.to_owned(),
        checksum: ART_CHECKSUM.to_owned(),
        transport_policy: TransportPolicy::CratesIo,
        plane: DependencyPlane::Implementation,
        disposition: ReleaseDisposition::OptionalRuntime,
    }];
    input.registry_observations = vec![RegistryDependencyObservation {
        manifest_dependency: ART.to_owned(),
        manifest_table: Some(ManifestTable::Dependencies),
        manifest_version_requirement: Some(format!("={ART_VERSION}")),
        is_manifest_optional: true,
        lock_package_version: Some(ART_VERSION.to_owned()),
        lock_source_coordinate: Some(CRATES_IO_SOURCE.to_owned()),
        lock_checksum: Some(DRIFT_CHECKSUM.to_owned()),
    }];
    input
}

/// The validator's rows for the two selected families, as a set (the validator deduplicates).
pub(super) fn selected_validator_rows(snapshot: &ReleaseDependencyInput) -> BTreeSet<(DiagnosticCode, String)> {
    validate_release_dependencies(snapshot)
        .diagnostics
        .into_iter()
        .filter(|row| matches!(row.code, DiagnosticCode::LockDrift | DiagnosticCode::MissingArchiveReceipt))
        .map(|row| (row.code, row.subject))
        .collect()
}
