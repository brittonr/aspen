use std::collections::BTreeMap;
use std::collections::BTreeSet;

const SHA1_HEX_LENGTH: usize = 40;
const BLAKE3_HEX_LENGTH: usize = 64;
const MAX_DEPENDENCY_ROWS: usize = 64;
const MAX_ARCHIVE_RECEIPTS: usize = 16;
const MAX_EVIDENCE_FILES_PER_RECEIPT: usize = 16;
const MAX_DISTRIBUTION_ARTIFACTS: usize = 64;
const REQUIRED_AGPL_LICENSE: &str = "AGPL-3.0-or-later";
const REQUIRED_CANONICAL_VALENCE_PACKAGE: &str = "valence-core";
const REQUIRED_HTTPS_PREFIX: &str = "https://";
const REQUIRED_RADICLE_PREFIX: &str = "rad://";
const REQUIRED_ARCHIVE_STATUS: &str = "archived";
const LEGAL_ADVICE_NON_CLAIM: &str = "not legal advice";
const UNIVERSAL_COMPLIANCE_NON_CLAIM: &str = "does not prove compliance in every jurisdiction";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Git,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportPolicy {
    Https,
    PrivateRadicle,
    SshPinnedWithNixArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseDisposition {
    Runtime,
    OptionalRuntime,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyExpectation {
    pub manifest_dependency: String,
    pub package_name: String,
    pub package_version: String,
    pub source_kind: SourceKind,
    pub source_coordinate: String,
    pub immutable_revision: String,
    pub nix_input: String,
    pub transport_policy: TransportPolicy,
    pub disposition: ReleaseDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyObservation {
    pub manifest_dependency: String,
    pub package_name: String,
    pub package_version: String,
    pub manifest_source_coordinate: Option<String>,
    pub manifest_revision: Option<String>,
    pub lock_source_coordinate: Option<String>,
    pub lock_revision: Option<String>,
    pub nix_input: String,
    pub nix_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackageIdentity {
    pub package_name: String,
    pub package_version: String,
    pub source_coordinate: String,
    pub immutable_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalValenceAuthority {
    pub source_coordinate: String,
    pub immutable_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFileExpectation {
    pub relative_path: String,
    pub blake3: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReceiptExpectation {
    pub id: String,
    pub source_coordinate: String,
    pub source_revision: String,
    pub nix_input: String,
    pub archive_path: String,
    pub status: String,
    pub evidence_files: Vec<EvidenceFileExpectation>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFileObservation {
    pub relative_path: String,
    pub blake3: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReceiptObservation {
    pub id: String,
    pub nix_revision: Option<String>,
    pub archive_present: bool,
    pub evidence_files: Vec<EvidenceFileObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionProfile {
    pub license: String,
    pub source_coordinate: String,
    pub source_revision: String,
    pub notice_artifacts: Vec<String>,
    pub source_export_artifacts: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionObservation {
    pub present_artifacts: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDependencyInput {
    pub dependencies: Vec<DependencyExpectation>,
    pub observations: Vec<DependencyObservation>,
    pub resolved_package_identities: Vec<ResolvedPackageIdentity>,
    pub unprofiled_git_dependencies: Vec<String>,
    pub canonical_valence: CanonicalValenceAuthority,
    pub archive_receipts: Vec<ArchiveReceiptExpectation>,
    pub archive_observations: Vec<ArchiveReceiptObservation>,
    pub distribution: DistributionProfile,
    pub distribution_observation: DistributionObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCode {
    BoundExceeded,
    DuplicateDependency,
    DuplicatePackageIdentity,
    FloatingRevision,
    InvalidArchiveReceipt,
    InvalidDistributionProfile,
    InvalidSourceRow,
    LockDrift,
    ManifestDrift,
    MissingArchiveReceipt,
    MissingDistributionArtifact,
    MissingSourceRow,
    NixDrift,
    NonCanonicalValence,
    UnprofiledGitDependency,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseDependencyDiagnostic {
    pub code: DiagnosticCode,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDependencyReport {
    pub diagnostics: Vec<ReleaseDependencyDiagnostic>,
    pub canonical_material: String,
}

impl ReleaseDependencyReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

// r[impl molten.project.reproducible_dependencies.contract]
// r[impl molten.project.reproducible_dependencies.drift_validation]
// r[impl molten.project.reproducible_dependencies.canonical_valence]
// r[impl molten.project.reproducible_dependencies.unique_valence_identity]
// r[impl molten.project.reproducible_dependencies.cross_repo_dependencies]
// r[impl molten.project.agpl_distribution_profile.contract]
pub fn validate_release_dependencies(input: &ReleaseDependencyInput) -> ReleaseDependencyReport {
    let mut diagnostics = Vec::new();
    validate_bounds(input, &mut diagnostics);
    validate_dependency_rows(input, &mut diagnostics);
    validate_resolved_package_identities(input, &mut diagnostics);
    validate_unprofiled_dependencies(input, &mut diagnostics);
    validate_canonical_valence(input, &mut diagnostics);
    validate_archive_receipts(input, &mut diagnostics);
    validate_distribution(input, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    let canonical_material = canonical_report_material(input, &diagnostics);
    ReleaseDependencyReport {
        diagnostics,
        canonical_material,
    }
}

fn validate_bounds(input: &ReleaseDependencyInput, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    check_bound("dependency rows", input.dependencies.len(), MAX_DEPENDENCY_ROWS, diagnostics);
    check_bound("dependency observations", input.observations.len(), MAX_DEPENDENCY_ROWS, diagnostics);
    check_bound(
        "resolved package identities",
        input.resolved_package_identities.len(),
        MAX_DEPENDENCY_ROWS,
        diagnostics,
    );
    check_bound("archive receipts", input.archive_receipts.len(), MAX_ARCHIVE_RECEIPTS, diagnostics);
    check_bound("archive observations", input.archive_observations.len(), MAX_ARCHIVE_RECEIPTS, diagnostics);
    check_bound(
        "notice artifacts",
        input.distribution.notice_artifacts.len(),
        MAX_DISTRIBUTION_ARTIFACTS,
        diagnostics,
    );
    check_bound(
        "source export artifacts",
        input.distribution.source_export_artifacts.len(),
        MAX_DISTRIBUTION_ARTIFACTS,
        diagnostics,
    );
    for receipt in &input.archive_receipts {
        check_bound(
            &format!("archive evidence files for {}", receipt.id),
            receipt.evidence_files.len(),
            MAX_EVIDENCE_FILES_PER_RECEIPT,
            diagnostics,
        );
    }
}

fn check_bound(subject: &str, observed: usize, maximum: usize, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    if observed <= maximum {
        return;
    }
    push_diagnostic(
        diagnostics,
        DiagnosticCode::BoundExceeded,
        subject,
        format!("observed {observed} entries; maximum is {maximum}"),
    );
}

fn validate_dependency_rows(input: &ReleaseDependencyInput, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    let observations = observation_index(&input.observations, diagnostics);
    let mut manifest_dependencies = BTreeSet::new();
    let mut package_identities = BTreeMap::<(String, String), BTreeSet<(String, String)>>::new();

    for expectation in &input.dependencies {
        validate_expectation(expectation, diagnostics);
        if !manifest_dependencies.insert(expectation.manifest_dependency.clone()) {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::DuplicateDependency,
                &expectation.manifest_dependency,
                "manifest dependency appears more than once in the release profile",
            );
        }
        package_identities
            .entry((expectation.package_name.clone(), expectation.package_version.clone()))
            .or_default()
            .insert((expectation.source_coordinate.clone(), expectation.immutable_revision.clone()));

        let Some(observation) = observations.get(expectation.manifest_dependency.as_str()) else {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::MissingSourceRow,
                &expectation.manifest_dependency,
                "manifest, lock, and Nix observations are missing",
            );
            continue;
        };
        validate_observation(expectation, observation, diagnostics);
    }

    for ((package_name, package_version), sources) in package_identities {
        if sources.len() <= 1 {
            continue;
        }
        push_diagnostic(
            diagnostics,
            DiagnosticCode::DuplicatePackageIdentity,
            format!("{package_name}@{package_version}"),
            format!("canonical package identity resolves from distinct sources: {sources:?}"),
        );
    }
}

fn observation_index<'a>(
    observations: &'a [DependencyObservation],
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) -> BTreeMap<&'a str, &'a DependencyObservation> {
    let mut index = BTreeMap::new();
    for observation in observations {
        if index.insert(observation.manifest_dependency.as_str(), observation).is_some() {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::DuplicateDependency,
                &observation.manifest_dependency,
                "dependency observation appears more than once",
            );
        }
    }
    index
}

fn validate_expectation(expectation: &DependencyExpectation, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    if expectation.manifest_dependency.trim().is_empty()
        || expectation.package_name.trim().is_empty()
        || expectation.package_version.trim().is_empty()
        || expectation.source_coordinate.trim().is_empty()
        || expectation.nix_input.trim().is_empty()
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            "source row contains an empty required field",
        );
    }
    if !is_exact_revision(&expectation.immutable_revision) {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::FloatingRevision,
            &expectation.manifest_dependency,
            format!("immutable revision must be {SHA1_HEX_LENGTH} lowercase hexadecimal characters"),
        );
    }
    let transport_valid = match expectation.transport_policy {
        TransportPolicy::Https => expectation.source_coordinate.starts_with(REQUIRED_HTTPS_PREFIX),
        TransportPolicy::PrivateRadicle => {
            expectation.source_coordinate.starts_with(REQUIRED_RADICLE_PREFIX)
                && !expectation.nix_input.trim().is_empty()
        }
        TransportPolicy::SshPinnedWithNixArchive => {
            expectation.source_coordinate.starts_with("ssh://git@github.com/")
                && !expectation.nix_input.trim().is_empty()
        }
    };
    if expectation.source_kind != SourceKind::Git || !transport_valid {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            "release Git source transport does not match its reviewed HTTPS or pinned-SSH-with-Nix policy",
        );
    }
}

fn validate_observation(
    expectation: &DependencyExpectation,
    observation: &DependencyObservation,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    compare_required(
        DiagnosticCode::ManifestDrift,
        &expectation.manifest_dependency,
        "manifest source coordinate",
        &expectation.source_coordinate,
        observation.manifest_source_coordinate.as_deref(),
        diagnostics,
    );
    compare_required(
        DiagnosticCode::ManifestDrift,
        &expectation.manifest_dependency,
        "manifest revision",
        &expectation.immutable_revision,
        observation.manifest_revision.as_deref(),
        diagnostics,
    );
    compare_required(
        DiagnosticCode::LockDrift,
        &expectation.manifest_dependency,
        "lock source coordinate",
        &expectation.source_coordinate,
        observation.lock_source_coordinate.as_deref(),
        diagnostics,
    );
    compare_required(
        DiagnosticCode::LockDrift,
        &expectation.manifest_dependency,
        "lock revision",
        &expectation.immutable_revision,
        observation.lock_revision.as_deref(),
        diagnostics,
    );
    if expectation.nix_input != observation.nix_input {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::NixDrift,
            &expectation.manifest_dependency,
            format!("expected Nix input {}, observed {}", expectation.nix_input, observation.nix_input),
        );
    }
    compare_required(
        DiagnosticCode::NixDrift,
        &expectation.manifest_dependency,
        "Nix revision",
        &expectation.immutable_revision,
        observation.nix_revision.as_deref(),
        diagnostics,
    );
    if expectation.package_name != observation.package_name
        || expectation.package_version != observation.package_version
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidSourceRow,
            &expectation.manifest_dependency,
            format!(
                "expected package {}@{}, observed {}@{}",
                expectation.package_name,
                expectation.package_version,
                observation.package_name,
                observation.package_version
            ),
        );
    }
}

fn compare_required(
    code: DiagnosticCode,
    subject: &str,
    field: &str,
    expected: &str,
    observed: Option<&str>,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    match observed {
        Some(actual) if actual == expected => {}
        Some(actual) => push_diagnostic(
            diagnostics,
            code,
            subject,
            format!("{field} mismatch: expected {expected}, observed {actual}"),
        ),
        None => push_diagnostic(diagnostics, code, subject, format!("{field} is missing; expected {expected}")),
    }
}

fn validate_resolved_package_identities(
    input: &ReleaseDependencyInput,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    let mut sources = BTreeMap::<(String, String), BTreeSet<(String, String)>>::new();
    for identity in &input.resolved_package_identities {
        sources
            .entry((identity.package_name.clone(), identity.package_version.clone()))
            .or_default()
            .insert((identity.source_coordinate.clone(), identity.immutable_revision.clone()));
    }
    for ((package_name, package_version), source_set) in sources {
        if source_set.len() <= 1 {
            continue;
        }
        push_diagnostic(
            diagnostics,
            DiagnosticCode::DuplicatePackageIdentity,
            format!("{package_name}@{package_version}"),
            format!("resolved graph contains distinct source identities: {source_set:?}"),
        );
    }
}

fn validate_unprofiled_dependencies(
    input: &ReleaseDependencyInput,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    let mut dependencies = input.unprofiled_git_dependencies.clone();
    dependencies.sort();
    dependencies.dedup();
    for dependency in dependencies {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::UnprofiledGitDependency,
            &dependency,
            "Git dependency is not declared by the reviewed release profile",
        );
    }
}

fn validate_canonical_valence(input: &ReleaseDependencyInput, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    let valence_rows: Vec<&DependencyExpectation> = input
        .dependencies
        .iter()
        .filter(|row| row.package_name == REQUIRED_CANONICAL_VALENCE_PACKAGE)
        .collect();
    if valence_rows.len() != 1 {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::NonCanonicalValence,
            REQUIRED_CANONICAL_VALENCE_PACKAGE,
            format!("expected exactly one canonical Valence dependency row, observed {}", valence_rows.len()),
        );
        return;
    }
    let row = valence_rows[0];
    if row.source_coordinate != input.canonical_valence.source_coordinate
        || row.immutable_revision != input.canonical_valence.immutable_revision
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::NonCanonicalValence,
            REQUIRED_CANONICAL_VALENCE_PACKAGE,
            format!(
                "expected canonical source {}#{}, observed {}#{}",
                input.canonical_valence.source_coordinate,
                input.canonical_valence.immutable_revision,
                row.source_coordinate,
                row.immutable_revision
            ),
        );
    }

    let resolved_valence: Vec<&ResolvedPackageIdentity> = input
        .resolved_package_identities
        .iter()
        .filter(|identity| identity.package_name == REQUIRED_CANONICAL_VALENCE_PACKAGE)
        .collect();
    if resolved_valence.len() != 1
        || resolved_valence[0].source_coordinate != input.canonical_valence.source_coordinate
        || resolved_valence[0].immutable_revision != input.canonical_valence.immutable_revision
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::NonCanonicalValence,
            REQUIRED_CANONICAL_VALENCE_PACKAGE,
            "resolved graph must contain exactly one accepted standalone Valence source",
        );
    }
}

fn validate_archive_receipts(input: &ReleaseDependencyInput, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    let observations: BTreeMap<&str, &ArchiveReceiptObservation> = input
        .archive_observations
        .iter()
        .map(|observation| (observation.id.as_str(), observation))
        .collect();
    for receipt in &input.archive_receipts {
        validate_archive_expectation(receipt, diagnostics);
        let Some(observation) = observations.get(receipt.id.as_str()) else {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::MissingArchiveReceipt,
                &receipt.id,
                "archive receipt observation is missing",
            );
            continue;
        };
        compare_required(
            DiagnosticCode::InvalidArchiveReceipt,
            &receipt.id,
            "archive source revision",
            &receipt.source_revision,
            observation.nix_revision.as_deref(),
            diagnostics,
        );
        if !observation.archive_present {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::MissingArchiveReceipt,
                &receipt.id,
                format!("archive path is missing: {}", receipt.archive_path),
            );
        }
        validate_evidence_files(receipt, observation, diagnostics);
    }
}

fn validate_archive_expectation(
    receipt: &ArchiveReceiptExpectation,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    if receipt.id.trim().is_empty()
        || receipt.source_coordinate.trim().is_empty()
        || receipt.nix_input.trim().is_empty()
        || !is_safe_relative_path(&receipt.archive_path)
        || receipt.evidence_files.is_empty()
        || !is_exact_revision(&receipt.source_revision)
        || receipt.status != REQUIRED_ARCHIVE_STATUS
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidArchiveReceipt,
            &receipt.id,
            "archive receipt must bind a source, exact revision, archived status, path, and evidence files",
        );
    }
    if !contains_boundary_non_claims(&receipt.non_claims) {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidArchiveReceipt,
            &receipt.id,
            "archive receipt must retain upstream/downstream correctness and release non-claims",
        );
    }
    for evidence in &receipt.evidence_files {
        if !is_safe_relative_path(&evidence.relative_path) || !is_blake3_hex(&evidence.blake3) {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::InvalidArchiveReceipt,
                &receipt.id,
                format!("invalid archive evidence binding: {}", evidence.relative_path),
            );
        }
    }
}

fn contains_boundary_non_claims(non_claims: &[String]) -> bool {
    let joined = non_claims.join(" ").to_ascii_lowercase();
    joined.contains("does not prove upstream correctness")
        && joined.contains("does not prove downstream integration correctness")
        && joined.contains("release eligibility")
}

fn validate_evidence_files(
    receipt: &ArchiveReceiptExpectation,
    observation: &ArchiveReceiptObservation,
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
) {
    let observed: BTreeMap<&str, Option<&str>> = observation
        .evidence_files
        .iter()
        .map(|file| (file.relative_path.as_str(), file.blake3.as_deref()))
        .collect();
    for expected in &receipt.evidence_files {
        compare_required(
            DiagnosticCode::InvalidArchiveReceipt,
            &receipt.id,
            &format!("evidence digest for {}", expected.relative_path),
            &expected.blake3,
            observed.get(expected.relative_path.as_str()).copied().flatten(),
            diagnostics,
        );
    }
}

fn validate_distribution(input: &ReleaseDependencyInput, diagnostics: &mut Vec<ReleaseDependencyDiagnostic>) {
    let profile = &input.distribution;
    if profile.license != REQUIRED_AGPL_LICENSE
        || !profile.source_coordinate.starts_with(REQUIRED_HTTPS_PREFIX)
        || !is_exact_revision(&profile.source_revision)
        || profile.notice_artifacts.is_empty()
        || profile.source_export_artifacts.is_empty()
    {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidDistributionProfile,
            "distribution",
            "AGPL profile must bind license, HTTPS source, exact revision, notices, and source exports",
        );
    }
    let non_claims = profile.non_claims.join(" ").to_ascii_lowercase();
    if !non_claims.contains(LEGAL_ADVICE_NON_CLAIM) || !non_claims.contains(UNIVERSAL_COMPLIANCE_NON_CLAIM) {
        push_diagnostic(
            diagnostics,
            DiagnosticCode::InvalidDistributionProfile,
            "distribution",
            "distribution profile must retain legal-advice and universal-compliance non-claims",
        );
    }

    let required_artifacts = profile.notice_artifacts.iter().chain(profile.source_export_artifacts.iter());
    for artifact in required_artifacts {
        if !is_safe_relative_path(artifact) {
            push_diagnostic(
                diagnostics,
                DiagnosticCode::InvalidDistributionProfile,
                artifact,
                "distribution artifact path must be safe and repository-relative",
            );
            continue;
        }
        if input.distribution_observation.present_artifacts.contains(artifact) {
            continue;
        }
        push_diagnostic(
            diagnostics,
            DiagnosticCode::MissingDistributionArtifact,
            artifact,
            "configured distribution artifact is missing",
        );
    }
}

fn is_safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.starts_with('/') || value.contains('\\') {
        return false;
    }
    value.split('/').all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn is_exact_revision(value: &str) -> bool {
    value.len() == SHA1_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_blake3_hex(value: &str) -> bool {
    value.len() == BLAKE3_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_diagnostic(
    diagnostics: &mut Vec<ReleaseDependencyDiagnostic>,
    code: DiagnosticCode,
    subject: impl Into<String>,
    message: impl Into<String>,
) {
    diagnostics.push(ReleaseDependencyDiagnostic {
        code,
        subject: subject.into(),
        message: message.into(),
    });
}

fn canonical_report_material(input: &ReleaseDependencyInput, diagnostics: &[ReleaseDependencyDiagnostic]) -> String {
    let mut rows = dependency_material_rows(input);
    rows.extend(archive_material_rows(input));
    rows.extend(distribution_material_rows(input));
    rows.extend(diagnostics.iter().map(|diagnostic| {
        format!("diagnostic\t{:?}\t{}\t{}", diagnostic.code, diagnostic.subject, diagnostic.message)
    }));
    rows.sort();
    rows.dedup();
    let mut material = rows.join("\n");
    material.push('\n');
    material
}

fn dependency_material_rows(input: &ReleaseDependencyInput) -> Vec<String> {
    let mut rows: Vec<String> = input
        .dependencies
        .iter()
        .map(|row| {
            format!(
                "dependency\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}",
                row.manifest_dependency,
                row.package_name,
                row.package_version,
                row.source_coordinate,
                row.immutable_revision,
                row.nix_input,
                row.transport_policy,
                row.disposition
            )
        })
        .collect();
    rows.extend(input.observations.iter().map(|row| {
        format!(
            "observation\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{}\t{:?}",
            row.manifest_dependency,
            row.package_name,
            row.package_version,
            row.manifest_source_coordinate,
            row.manifest_revision,
            row.lock_source_coordinate,
            row.lock_revision,
            row.nix_input,
            row.nix_revision
        )
    }));
    rows.extend(input.resolved_package_identities.iter().map(|identity| {
        format!(
            "resolved\t{}\t{}\t{}\t{}",
            identity.package_name, identity.package_version, identity.source_coordinate, identity.immutable_revision
        )
    }));
    rows.extend(input.unprofiled_git_dependencies.iter().map(|dependency| format!("unprofiled\t{dependency}")));
    rows
}

fn archive_material_rows(input: &ReleaseDependencyInput) -> Vec<String> {
    let mut rows = Vec::new();
    for receipt in &input.archive_receipts {
        rows.push(format!(
            "archive\t{}\t{}\t{}\t{}\t{}\t{}",
            receipt.id,
            receipt.source_coordinate,
            receipt.source_revision,
            receipt.nix_input,
            receipt.archive_path,
            receipt.status
        ));
        rows.extend(
            receipt
                .evidence_files
                .iter()
                .map(|file| format!("archive-evidence\t{}\t{}\t{}", receipt.id, file.relative_path, file.blake3)),
        );
        rows.extend(
            receipt.non_claims.iter().map(|non_claim| format!("archive-non-claim\t{}\t{non_claim}", receipt.id)),
        );
    }
    for observation in &input.archive_observations {
        rows.push(format!(
            "archive-observation\t{}\t{:?}\t{}",
            observation.id, observation.nix_revision, observation.archive_present
        ));
        rows.extend(observation.evidence_files.iter().map(|file| {
            format!("archive-evidence-observation\t{}\t{}\t{:?}", observation.id, file.relative_path, file.blake3)
        }));
    }
    rows
}

fn distribution_material_rows(input: &ReleaseDependencyInput) -> Vec<String> {
    let mut rows = vec![format!(
        "distribution\t{}\t{}\t{}",
        input.distribution.license, input.distribution.source_coordinate, input.distribution.source_revision
    )];
    rows.extend(
        input
            .distribution
            .notice_artifacts
            .iter()
            .map(|artifact| format!("distribution-notice\t{artifact}")),
    );
    rows.extend(
        input
            .distribution
            .source_export_artifacts
            .iter()
            .map(|artifact| format!("distribution-source\t{artifact}")),
    );
    rows.extend(input.distribution.non_claims.iter().map(|non_claim| format!("distribution-non-claim\t{non_claim}")));
    rows.extend(
        input
            .distribution_observation
            .present_artifacts
            .iter()
            .map(|artifact| format!("distribution-present\t{artifact}")),
    );
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASALT_REVISION: &str = "89675cd4f585f837323c049e4a25f7b94c903038";
    const EXECUTABLE_EXTENT_REVISION: &str = "025d9636f0161777710dac37b3c210ca0ad9483f";
    const VALENCE_REVISION: &str = "5f1c2ba5072c6f9622fa59b1af20502985f569fd";
    const OCTET_REVISION: &str = "4367300e10740ecc99ba4b2171ace561b4787327";
    const DRIFT_REVISION: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const EVIDENCE_DIGEST: &str = "3abda77b5931c5ef6dcdde504f71dfce06f95d1d6c43f087cd35f8816147f7e2";
    const ALTERNATE_EVIDENCE_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const BASALT_SOURCE: &str = "https://github.com/OnixResearch/basalt.git";
    const EXECUTABLE_EXTENT_SOURCE: &str = "rad://z37R1bP1kHcELs89RNbQRaqbCVKxB";
    const VALENCE_SOURCE: &str = "https://github.com/OnixResearch/valence.git";
    const ASPEN_SOURCE: &str = "https://github.com/OnixResearch/aspen.git";
    const ARCHIVE_NON_CLAIM: &str = "dependency satisfaction does not prove upstream correctness or remote trust; dependency satisfaction does not prove downstream integration correctness or release eligibility";
    const DISTRIBUTION_NON_CLAIM: &str =
        "this profile is not legal advice and does not prove compliance in every jurisdiction";

    fn dependency(
        manifest_dependency: &str,
        package_name: &str,
        package_version: &str,
        source: &str,
        revision: &str,
        nix_input: &str,
    ) -> DependencyExpectation {
        DependencyExpectation {
            manifest_dependency: manifest_dependency.to_owned(),
            package_name: package_name.to_owned(),
            package_version: package_version.to_owned(),
            source_kind: SourceKind::Git,
            source_coordinate: source.to_owned(),
            immutable_revision: revision.to_owned(),
            nix_input: nix_input.to_owned(),
            transport_policy: TransportPolicy::Https,
            disposition: ReleaseDisposition::Runtime,
        }
    }

    fn observation(expectation: &DependencyExpectation) -> DependencyObservation {
        DependencyObservation {
            manifest_dependency: expectation.manifest_dependency.clone(),
            package_name: expectation.package_name.clone(),
            package_version: expectation.package_version.clone(),
            manifest_source_coordinate: Some(expectation.source_coordinate.clone()),
            manifest_revision: Some(expectation.immutable_revision.clone()),
            lock_source_coordinate: Some(expectation.source_coordinate.clone()),
            lock_revision: Some(expectation.immutable_revision.clone()),
            nix_input: expectation.nix_input.clone(),
            nix_revision: Some(expectation.immutable_revision.clone()),
        }
    }

    fn receipt(id: &str, revision: &str) -> ArchiveReceiptExpectation {
        ArchiveReceiptExpectation {
            id: id.to_owned(),
            source_coordinate: VALENCE_SOURCE.to_owned(),
            source_revision: revision.to_owned(),
            nix_input: format!("{id}-src"),
            archive_path: "cairn/archive/reviewed-change".to_owned(),
            status: REQUIRED_ARCHIVE_STATUS.to_owned(),
            evidence_files: vec![EvidenceFileExpectation {
                relative_path: "cairn/archive/reviewed-change/tasks.md".to_owned(),
                blake3: EVIDENCE_DIGEST.to_owned(),
            }],
            non_claims: vec![ARCHIVE_NON_CLAIM.to_owned()],
        }
    }

    fn receipt_observation(receipt: &ArchiveReceiptExpectation) -> ArchiveReceiptObservation {
        ArchiveReceiptObservation {
            id: receipt.id.clone(),
            nix_revision: Some(receipt.source_revision.clone()),
            archive_present: true,
            evidence_files: receipt
                .evidence_files
                .iter()
                .map(|file| EvidenceFileObservation {
                    relative_path: file.relative_path.clone(),
                    blake3: Some(file.blake3.clone()),
                })
                .collect(),
        }
    }

    fn valid_input() -> ReleaseDependencyInput {
        let basalt = dependency("basalt", "basalt", "0.1.0", BASALT_SOURCE, BASALT_REVISION, "basalt-src");
        let valence = dependency(
            "valence",
            REQUIRED_CANONICAL_VALENCE_PACKAGE,
            "0.1.0",
            VALENCE_SOURCE,
            VALENCE_REVISION,
            "valence-src",
        );
        let valence_receipt = receipt("valence-integrity", VALENCE_REVISION);
        let mut octet_receipt = receipt("octet-cutover", OCTET_REVISION);
        octet_receipt.source_coordinate = "https://github.com/OnixResearch/octet.git".to_owned();
        ReleaseDependencyInput {
            dependencies: vec![basalt.clone(), valence.clone()],
            observations: vec![observation(&valence), observation(&basalt)],
            resolved_package_identities: vec![
                ResolvedPackageIdentity {
                    package_name: basalt.package_name.clone(),
                    package_version: basalt.package_version.clone(),
                    source_coordinate: basalt.source_coordinate.clone(),
                    immutable_revision: basalt.immutable_revision.clone(),
                },
                ResolvedPackageIdentity {
                    package_name: valence.package_name.clone(),
                    package_version: valence.package_version.clone(),
                    source_coordinate: valence.source_coordinate.clone(),
                    immutable_revision: valence.immutable_revision.clone(),
                },
            ],
            unprofiled_git_dependencies: Vec::new(),
            canonical_valence: CanonicalValenceAuthority {
                source_coordinate: VALENCE_SOURCE.to_owned(),
                immutable_revision: VALENCE_REVISION.to_owned(),
            },
            archive_receipts: vec![valence_receipt.clone(), octet_receipt.clone()],
            archive_observations: vec![
                receipt_observation(&octet_receipt),
                receipt_observation(&valence_receipt),
            ],
            distribution: DistributionProfile {
                license: REQUIRED_AGPL_LICENSE.to_owned(),
                source_coordinate: ASPEN_SOURCE.to_owned(),
                source_revision: BASALT_REVISION.to_owned(),
                notice_artifacts: vec!["LICENSE".to_owned()],
                source_export_artifacts: vec!["Cargo.toml".to_owned()],
                non_claims: vec![DISTRIBUTION_NON_CLAIM.to_owned()],
            },
            distribution_observation: DistributionObservation {
                present_artifacts: BTreeSet::from(["Cargo.toml".to_owned(), "LICENSE".to_owned()]),
            },
        }
    }

    // r[verify molten.project.reproducible_dependencies.fixtures.positive]
    // r[verify molten.project.agpl_distribution_profile.contract]
    #[test]
    fn exact_pins_canonical_valence_archives_and_agpl_profile_pass() {
        let first = validate_release_dependencies(&valid_input());
        let mut reordered = valid_input();
        reordered.dependencies.reverse();
        reordered.observations.reverse();
        reordered.archive_receipts.reverse();
        reordered.archive_observations.reverse();
        let second = validate_release_dependencies(&reordered);

        assert!(first.is_valid());
        assert!(second.is_valid());
        assert_eq!(first.canonical_material, second.canonical_material);
    }

    // r[verify molten.project.reproducible_dependencies.fixtures.negative]
    #[test]
    fn exact_ssh_pin_with_matching_nix_archive_passes() {
        let mut input = valid_input();
        let source = "ssh://git@github.com/OnixResearch/basalt.git".to_owned();
        input.dependencies[0].source_coordinate = source.clone();
        input.dependencies[0].transport_policy = TransportPolicy::SshPinnedWithNixArchive;
        input.observations[1].manifest_source_coordinate = Some(source.clone());
        input.observations[1].lock_source_coordinate = Some(source.clone());
        input.resolved_package_identities[0].source_coordinate = source;

        assert!(validate_release_dependencies(&input).is_valid());
    }

    #[test]
    fn private_radicle_pin_requires_its_exact_transport_policy() {
        let mut admitted = valid_input();
        admitted.dependencies[0].source_coordinate = EXECUTABLE_EXTENT_SOURCE.to_owned();
        admitted.dependencies[0].immutable_revision = EXECUTABLE_EXTENT_REVISION.to_owned();
        admitted.dependencies[0].transport_policy = TransportPolicy::PrivateRadicle;
        admitted.dependencies[0].disposition = ReleaseDisposition::OptionalRuntime;
        admitted.observations[1].manifest_source_coordinate = Some(EXECUTABLE_EXTENT_SOURCE.to_owned());
        admitted.observations[1].manifest_revision = Some(EXECUTABLE_EXTENT_REVISION.to_owned());
        admitted.observations[1].lock_source_coordinate = Some(EXECUTABLE_EXTENT_SOURCE.to_owned());
        admitted.observations[1].lock_revision = Some(EXECUTABLE_EXTENT_REVISION.to_owned());
        admitted.observations[1].nix_revision = Some(EXECUTABLE_EXTENT_REVISION.to_owned());
        admitted.resolved_package_identities[0].source_coordinate = EXECUTABLE_EXTENT_SOURCE.to_owned();
        admitted.resolved_package_identities[0].immutable_revision = EXECUTABLE_EXTENT_REVISION.to_owned();
        assert!(validate_release_dependencies(&admitted).is_valid());

        admitted.dependencies[0].transport_policy = TransportPolicy::Https;
        let denied = validate_release_dependencies(&admitted);
        assert!(!denied.is_valid());
        assert!(denied.diagnostics.iter().any(|item| item.code == DiagnosticCode::InvalidSourceRow));
    }

    #[test]
    fn report_identity_binds_archive_evidence_bytes() {
        let first = validate_release_dependencies(&valid_input());
        let mut changed = valid_input();
        changed.archive_receipts[0].evidence_files[0].blake3 = ALTERNATE_EVIDENCE_DIGEST.to_owned();
        changed.archive_observations[1].evidence_files[0].blake3 = Some(ALTERNATE_EVIDENCE_DIGEST.to_owned());
        let second = validate_release_dependencies(&changed);

        assert!(first.is_valid());
        assert!(second.is_valid());
        assert_ne!(first.canonical_material, second.canonical_material);
    }

    #[test]
    fn floating_and_drifting_sources_fail_closed() {
        let mut input = valid_input();
        input.dependencies[0].immutable_revision = "main".to_owned();
        input.observations[0].lock_revision = Some(DRIFT_REVISION.to_owned());
        input.observations[0].nix_revision = None;
        let report = validate_release_dependencies(&input);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::FloatingRevision));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::LockDrift));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::NixDrift));
    }

    #[test]
    fn noncanonical_and_duplicate_valence_sources_fail_closed() {
        let mut input = valid_input();
        let hosted = dependency(
            "legacy-octet",
            REQUIRED_CANONICAL_VALENCE_PACKAGE,
            "0.1.0",
            "https://github.com/OnixResearch/octet.git",
            OCTET_REVISION,
            "octet-cutover-src",
        );
        input.observations.push(observation(&hosted));
        input.resolved_package_identities.push(ResolvedPackageIdentity {
            package_name: hosted.package_name.clone(),
            package_version: hosted.package_version.clone(),
            source_coordinate: hosted.source_coordinate.clone(),
            immutable_revision: hosted.immutable_revision.clone(),
        });
        input.dependencies.push(hosted);
        let report = validate_release_dependencies(&input);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::DuplicatePackageIdentity));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::NonCanonicalValence));
    }

    #[test]
    fn stale_archives_and_incomplete_distribution_evidence_fail_closed() {
        let mut input = valid_input();
        input.archive_observations[0].archive_present = false;
        input.archive_observations[0].evidence_files[0].blake3 = Some(DRIFT_REVISION.to_owned());
        input.distribution_observation.present_artifacts.clear();
        input.distribution.non_claims.clear();
        input.distribution.source_export_artifacts.push("../outside-source".to_owned());
        let report = validate_release_dependencies(&input);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::MissingArchiveReceipt));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::InvalidArchiveReceipt));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::MissingDistributionArtifact));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::InvalidDistributionProfile));
    }

    #[test]
    fn unsupported_transport_and_unprofiled_git_dependency_fail_closed() {
        let mut input = valid_input();
        input.dependencies[0].source_coordinate = "ssh://git@github.com/OnixResearch/basalt.git".to_owned();
        input.dependencies[0].transport_policy = TransportPolicy::Https;
        input.unprofiled_git_dependencies.push("floating-helper".to_owned());
        let report = validate_release_dependencies(&input);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::InvalidSourceRow));
        assert!(report.diagnostics.iter().any(|item| item.code == DiagnosticCode::UnprofiledGitDependency));
    }
}
