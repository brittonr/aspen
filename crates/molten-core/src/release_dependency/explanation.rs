//! Snapshot-bound, bounded explanations for the selected release-dependency
//! diagnostic families, `LockDrift` and `MissingArchiveReceipt`.
//!
//! The explanation is a view over the same immutable [`ReleaseDependencyInput`]
//! the validator decides on. Every evaluation starts fresh from one snapshot and
//! binds its result to that snapshot's identity, the rule revision, the declared
//! collection states and the limits. It grants no authority: a refused or
//! incomplete evaluation carries no report, and a completed report only mirrors
//! [`validate_release_dependencies`] for the same snapshot.

mod identity;
mod verify;

#[cfg(test)]
mod comparison;
#[cfg(test)]
mod corpus;
#[cfg(test)]
mod relational_pilot;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeMap;

use self::identity::binding_identity;
use self::identity::snapshot_identity;
pub use self::verify::ReferenceIssue;
pub use self::verify::verify_explanation_references;
use super::DiagnosticCode;
use super::MAX_ARCHIVE_RECEIPTS;
use super::MAX_DEPENDENCY_ROWS;
use super::MAX_REGISTRY_ROWS;
use super::ReleaseDependencyInput;
use super::validate_release_dependencies;

/// Revision of the explanation rules, bound into every explanation.
pub const EXPLANATION_RULE_REVISION: &str = "molten.release-dependency.explanation.v1";
/// Largest evaluation step budget a caller may request.
pub const MAX_EXPLANATION_STEPS: usize = 4_096;
/// Largest projected fact budget a caller may request.
pub const MAX_EXPLANATION_FACTS: usize = 512;
/// Deepest derivation a caller may request: a field fact, or an absence over a completed relation.
pub const MAX_EXPLANATION_DEPTH: u8 = 2;
/// Largest rendered explanation output a caller may request.
pub const MAX_EXPLANATION_OUTPUT_BYTES: usize = 32_768;

const FIELD_RULE_DEPTH: u8 = 1;
const ABSENCE_RULE_DEPTH: u8 = 2;
const REPORTED_ABSENT_VALUE: &str = "false";

/// Whether a shell finished collecting one observation relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollectionState {
    Completed,
    Incomplete,
    Unavailable,
}

/// Declared collection state of every observation relation the rules read.
///
/// Only a relation declared completed can support an absence check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionDeclaration {
    pub dependency_observations: CollectionState,
    pub registry_observations: CollectionState,
    pub archive_observations: CollectionState,
}

impl CollectionDeclaration {
    pub const COMPLETED: Self = Self {
        dependency_observations: CollectionState::Completed,
        registry_observations: CollectionState::Completed,
        archive_observations: CollectionState::Completed,
    };

    fn states(self) -> [(FactRelation, CollectionState); 3] {
        [
            (FactRelation::DependencyObservation, self.dependency_observations),
            (FactRelation::RegistryObservation, self.registry_observations),
            (FactRelation::ArchiveObservation, self.archive_observations),
        ]
    }
}

/// Bounds for one evaluation. Every field must be positive and within its `MAX_EXPLANATION_*` cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplanationLimits {
    pub max_steps: usize,
    pub max_facts: usize,
    pub max_depth: u8,
    pub max_output_bytes: usize,
}

impl ExplanationLimits {
    pub const MAXIMUM: Self = Self {
        max_steps: MAX_EXPLANATION_STEPS,
        max_facts: MAX_EXPLANATION_FACTS,
        max_depth: MAX_EXPLANATION_DEPTH,
        max_output_bytes: MAX_EXPLANATION_OUTPUT_BYTES,
    };

    fn is_supported(self) -> bool {
        (1..=MAX_EXPLANATION_STEPS).contains(&self.max_steps)
            && (1..=MAX_EXPLANATION_FACTS).contains(&self.max_facts)
            && (1..=MAX_EXPLANATION_DEPTH).contains(&self.max_depth)
            && (1..=MAX_EXPLANATION_OUTPUT_BYTES).contains(&self.max_output_bytes)
    }
}

/// Input relation a fact reference points into. Occurrences index the snapshot's row vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactRelation {
    DependencyExpectation,
    DependencyObservation,
    RegistryExpectation,
    RegistryObservation,
    ArchiveExpectation,
    ArchiveObservation,
}

impl FactRelation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DependencyExpectation => "dependency-expectation",
            Self::DependencyObservation => "dependency-observation",
            Self::RegistryExpectation => "registry-expectation",
            Self::RegistryObservation => "registry-observation",
            Self::ArchiveExpectation => "archive-expectation",
            Self::ArchiveObservation => "archive-observation",
        }
    }
}

/// Row field named by a fact reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplainedField {
    SourceCoordinate,
    ImmutableRevision,
    PackageVersion,
    Checksum,
    LockSourceCoordinate,
    LockRevision,
    LockPackageVersion,
    LockChecksum,
    ArchivePath,
    ArchivePresent,
}

impl ExplainedField {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SourceCoordinate => "source_coordinate",
            Self::ImmutableRevision => "immutable_revision",
            Self::PackageVersion => "package_version",
            Self::Checksum => "checksum",
            Self::LockSourceCoordinate => "lock_source_coordinate",
            Self::LockRevision => "lock_revision",
            Self::LockPackageVersion => "lock_package_version",
            Self::LockChecksum => "lock_checksum",
            Self::ArchivePath => "archive_path",
            Self::ArchivePresent => "archive_present",
        }
    }
}

/// One supporting fact of an explanation, identified by snapshot occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SupportRef {
    /// A field of one row occurrence. `None` records a present row whose field is missing.
    Field {
        relation: FactRelation,
        occurrence: usize,
        field: ExplainedField,
        value: Option<String>,
    },
    /// No row keyed by `key` exists in a relation declared completed.
    Absent { relation: FactRelation, key: String },
}

/// Explanation rule. Each rule refines one validator diagnostic family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplanationRule {
    GitLockSourceCoordinate,
    GitLockRevision,
    RegistryLockPackageVersion,
    RegistryLockSourceCoordinate,
    RegistryLockChecksum,
    ArchiveObservationAbsent,
    ArchiveReportedAbsent,
}

impl ExplanationRule {
    pub const fn id(self) -> &'static str {
        match self {
            Self::GitLockSourceCoordinate => "release-dependency/lock-drift/git-source-coordinate",
            Self::GitLockRevision => "release-dependency/lock-drift/git-revision",
            Self::RegistryLockPackageVersion => "release-dependency/lock-drift/registry-package-version",
            Self::RegistryLockSourceCoordinate => "release-dependency/lock-drift/registry-source-coordinate",
            Self::RegistryLockChecksum => "release-dependency/lock-drift/registry-checksum",
            Self::ArchiveObservationAbsent => "release-dependency/missing-archive-receipt/observation-absent",
            Self::ArchiveReportedAbsent => "release-dependency/missing-archive-receipt/reported-absent",
        }
    }

    pub const fn code(self) -> DiagnosticCode {
        match self.shape() {
            RuleShape::Drift(_) => DiagnosticCode::LockDrift,
            RuleShape::ArchiveObservationAbsent | RuleShape::ArchiveReportedAbsent => {
                DiagnosticCode::MissingArchiveReceipt
            }
        }
    }

    /// Derivation depth: 1 for a direct field comparison, 2 for an absence over a completed
    /// relation.
    pub const fn depth(self) -> u8 {
        match self.shape() {
            RuleShape::ArchiveObservationAbsent => ABSENCE_RULE_DEPTH,
            RuleShape::Drift(_) | RuleShape::ArchiveReportedAbsent => FIELD_RULE_DEPTH,
        }
    }

    const fn shape(self) -> RuleShape {
        match self {
            Self::GitLockSourceCoordinate => RuleShape::Drift(GIT_LOCK_SOURCE),
            Self::GitLockRevision => RuleShape::Drift(GIT_LOCK_REVISION),
            Self::RegistryLockPackageVersion => RuleShape::Drift(REGISTRY_LOCK_VERSION),
            Self::RegistryLockSourceCoordinate => RuleShape::Drift(REGISTRY_LOCK_SOURCE),
            Self::RegistryLockChecksum => RuleShape::Drift(REGISTRY_LOCK_CHECKSUM),
            Self::ArchiveObservationAbsent => RuleShape::ArchiveObservationAbsent,
            Self::ArchiveReportedAbsent => RuleShape::ArchiveReportedAbsent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleShape {
    Drift(DriftRule),
    ArchiveObservationAbsent,
    ArchiveReportedAbsent,
}

/// A drift rule fires when the effective observation field differs from, or lacks, the expected
/// field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DriftRule {
    rule: ExplanationRule,
    expected: (FactRelation, ExplainedField),
    observed: (FactRelation, ExplainedField),
}

const GIT_LOCK_SOURCE: DriftRule = DriftRule {
    rule: ExplanationRule::GitLockSourceCoordinate,
    expected: (FactRelation::DependencyExpectation, ExplainedField::SourceCoordinate),
    observed: (FactRelation::DependencyObservation, ExplainedField::LockSourceCoordinate),
};
const GIT_LOCK_REVISION: DriftRule = DriftRule {
    rule: ExplanationRule::GitLockRevision,
    expected: (FactRelation::DependencyExpectation, ExplainedField::ImmutableRevision),
    observed: (FactRelation::DependencyObservation, ExplainedField::LockRevision),
};
const REGISTRY_LOCK_VERSION: DriftRule = DriftRule {
    rule: ExplanationRule::RegistryLockPackageVersion,
    expected: (FactRelation::RegistryExpectation, ExplainedField::PackageVersion),
    observed: (FactRelation::RegistryObservation, ExplainedField::LockPackageVersion),
};
const REGISTRY_LOCK_SOURCE: DriftRule = DriftRule {
    rule: ExplanationRule::RegistryLockSourceCoordinate,
    expected: (FactRelation::RegistryExpectation, ExplainedField::SourceCoordinate),
    observed: (FactRelation::RegistryObservation, ExplainedField::LockSourceCoordinate),
};
const REGISTRY_LOCK_CHECKSUM: DriftRule = DriftRule {
    rule: ExplanationRule::RegistryLockChecksum,
    expected: (FactRelation::RegistryExpectation, ExplainedField::Checksum),
    observed: (FactRelation::RegistryObservation, ExplainedField::LockChecksum),
};

/// One explained diagnostic row with its occurrence-identified support.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Explanation {
    pub rule: ExplanationRule,
    pub subject: String,
    pub support: Vec<SupportRef>,
}

impl Explanation {
    pub const fn code(&self) -> DiagnosticCode {
        self.rule.code()
    }

    /// Deterministic display text; its byte length is what the output bound counts.
    pub fn render(&self) -> String {
        let support = self
            .support
            .iter()
            .map(|reference| match reference {
                SupportRef::Field {
                    relation,
                    occurrence,
                    field,
                    value,
                } => format!(
                    "{}#{occurrence}.{}={}",
                    relation.label(),
                    field.label(),
                    value.as_deref().unwrap_or("missing")
                ),
                SupportRef::Absent { relation, key } => format!("absent {}[{key}]", relation.label()),
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!("{} {} <- {support}\n", self.rule.id(), self.subject)
    }
}

/// Exact inputs an explanation report is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationBinding {
    pub snapshot_ref: String,
    pub rule_revision: String,
    pub collections: CollectionDeclaration,
    pub limits: ExplanationLimits,
}

impl ExplanationBinding {
    pub fn binding_ref(&self) -> String {
        binding_identity(self)
    }
}

/// Resources one completed evaluation used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplanationUsage {
    pub facts: usize,
    pub steps: usize,
    pub depth: u8,
    pub output_bytes: usize,
}

/// Completed evaluation over one snapshot. Explanations are sorted and capped by the output bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationReport {
    pub binding: ExplanationBinding,
    /// The validator's decision for the same snapshot; the explanation adds no authority.
    pub validator_admits: bool,
    pub explanations: Vec<Explanation>,
    pub omitted_explanations: usize,
    pub usage: ExplanationUsage,
}

/// Input row bound the explanation projection cannot represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedInput {
    RowBound {
        relation: FactRelation,
        observed: usize,
        maximum: usize,
    },
    Limits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationBudget {
    Facts,
    Steps,
    Depth,
}

/// Explicit result of one explanation evaluation.
///
/// Only `Completed` and `Truncated` carry a report; every other outcome is a
/// refusal and never yields a release pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplanationOutcome {
    Completed(ExplanationReport),
    /// Evaluation completed but the output bound omitted some explanations.
    Truncated(ExplanationReport),
    Unavailable {
        snapshot_ref: String,
        relation: FactRelation,
    },
    Incomplete {
        snapshot_ref: String,
        relation: FactRelation,
    },
    Unsupported {
        snapshot_ref: String,
        reason: UnsupportedInput,
    },
    BudgetExhausted {
        snapshot_ref: String,
        budget: ExplanationBudget,
        limit: usize,
    },
}

impl ExplanationOutcome {
    pub const fn report(&self) -> Option<&ExplanationReport> {
        match self {
            Self::Completed(report) | Self::Truncated(report) => Some(report),
            Self::Unavailable { .. }
            | Self::Incomplete { .. }
            | Self::Unsupported { .. }
            | Self::BudgetExhausted { .. } => None,
        }
    }

    /// True only for a completed evaluation whose same-snapshot validator report admits the
    /// release.
    pub fn admits_release(&self) -> bool {
        self.report().is_some_and(|report| report.validator_admits)
    }
}

/// One evaluation request: the immutable snapshot, its declared collection states, and limits.
#[derive(Debug, Clone, Copy)]
pub struct ExplanationRequest<'a> {
    pub snapshot: &'a ReleaseDependencyInput,
    pub collections: CollectionDeclaration,
    pub limits: ExplanationLimits,
}

// r[impl molten.project.reproducible_dependencies.explanation_outcomes]
// r[impl molten.project.reproducible_dependencies.explanation_references]
/// Explain the selected diagnostic families of one snapshot, fresh and bounded.
pub fn explain_release_dependencies(request: &ExplanationRequest<'_>) -> ExplanationOutcome {
    let snapshot = request.snapshot;
    let snapshot_ref = snapshot_identity(snapshot);
    if !request.limits.is_supported() {
        return ExplanationOutcome::Unsupported {
            snapshot_ref,
            reason: UnsupportedInput::Limits,
        };
    }
    if let Some(reason) = unsupported_row_bound(snapshot) {
        return ExplanationOutcome::Unsupported { snapshot_ref, reason };
    }
    if let Some(outcome) = uncompleted_collection(&snapshot_ref, request.collections) {
        return outcome;
    }
    let facts = projected_fact_count(snapshot);
    if facts > request.limits.max_facts {
        return budget_exhausted(snapshot_ref, ExplanationBudget::Facts, request.limits.max_facts);
    }
    let mut meter = StepMeter {
        used: 0,
        limit: request.limits.max_steps,
    };
    let Ok(mut explanations) = derive_explanations(snapshot, &mut meter) else {
        return budget_exhausted(snapshot_ref, ExplanationBudget::Steps, request.limits.max_steps);
    };
    let depth = explanations.iter().map(|explanation| explanation.rule.depth()).max().unwrap_or(0);
    if depth > request.limits.max_depth {
        return budget_exhausted(snapshot_ref, ExplanationBudget::Depth, usize::from(request.limits.max_depth));
    }
    explanations.sort();
    let derived = explanations.len();
    let (explanations, output_bytes) = bound_output(explanations, request.limits.max_output_bytes);
    let report = ExplanationReport {
        binding: ExplanationBinding {
            snapshot_ref,
            rule_revision: EXPLANATION_RULE_REVISION.to_owned(),
            collections: request.collections,
            limits: request.limits,
        },
        validator_admits: validate_release_dependencies(snapshot).is_valid(),
        omitted_explanations: derived - explanations.len(),
        explanations,
        usage: ExplanationUsage {
            facts,
            steps: meter.used,
            depth,
            output_bytes,
        },
    };
    if report.omitted_explanations == 0 {
        ExplanationOutcome::Completed(report)
    } else {
        ExplanationOutcome::Truncated(report)
    }
}

const fn budget_exhausted(snapshot_ref: String, budget: ExplanationBudget, limit: usize) -> ExplanationOutcome {
    ExplanationOutcome::BudgetExhausted {
        snapshot_ref,
        budget,
        limit,
    }
}

fn unsupported_row_bound(snapshot: &ReleaseDependencyInput) -> Option<UnsupportedInput> {
    [
        (FactRelation::DependencyExpectation, snapshot.dependencies.len(), MAX_DEPENDENCY_ROWS),
        (FactRelation::DependencyObservation, snapshot.observations.len(), MAX_DEPENDENCY_ROWS),
        (FactRelation::RegistryExpectation, snapshot.registry_dependencies.len(), MAX_REGISTRY_ROWS),
        (FactRelation::RegistryObservation, snapshot.registry_observations.len(), MAX_REGISTRY_ROWS),
        (FactRelation::ArchiveExpectation, snapshot.archive_receipts.len(), MAX_ARCHIVE_RECEIPTS),
        (FactRelation::ArchiveObservation, snapshot.archive_observations.len(), MAX_ARCHIVE_RECEIPTS),
    ]
    .into_iter()
    .find(|(_, observed, maximum)| observed > maximum)
    .map(|(relation, observed, maximum)| UnsupportedInput::RowBound {
        relation,
        observed,
        maximum,
    })
}

fn uncompleted_collection(snapshot_ref: &str, collections: CollectionDeclaration) -> Option<ExplanationOutcome> {
    collections.states().into_iter().find_map(|(relation, state)| match state {
        CollectionState::Completed => None,
        CollectionState::Incomplete => Some(ExplanationOutcome::Incomplete {
            snapshot_ref: snapshot_ref.to_owned(),
            relation,
        }),
        CollectionState::Unavailable => Some(ExplanationOutcome::Unavailable {
            snapshot_ref: snapshot_ref.to_owned(),
            relation,
        }),
    })
}

fn projected_fact_count(snapshot: &ReleaseDependencyInput) -> usize {
    snapshot.dependencies.len()
        + snapshot.observations.len()
        + snapshot.registry_dependencies.len()
        + snapshot.registry_observations.len()
        + snapshot.archive_receipts.len()
        + snapshot.archive_observations.len()
}

fn bound_output(explanations: Vec<Explanation>, max_output_bytes: usize) -> (Vec<Explanation>, usize) {
    let mut output_bytes = 0_usize;
    let mut shown = Vec::with_capacity(explanations.len());
    for explanation in explanations {
        let Some(next) = output_bytes.checked_add(explanation.render().len()) else {
            break;
        };
        if next > max_output_bytes {
            break;
        }
        output_bytes = next;
        shown.push(explanation);
    }
    (shown, output_bytes)
}

struct StepMeter {
    used: usize,
    limit: usize,
}

struct StepsExhausted;

impl StepMeter {
    fn charge(&mut self) -> Result<(), StepsExhausted> {
        if self.used >= self.limit {
            return Err(StepsExhausted);
        }
        self.used += 1;
        Ok(())
    }
}

fn derive_explanations(
    snapshot: &ReleaseDependencyInput,
    meter: &mut StepMeter,
) -> Result<Vec<Explanation>, StepsExhausted> {
    let mut explanations = Vec::new();
    derive_git_lock_drift(snapshot, meter, &mut explanations)?;
    derive_registry_lock_drift(snapshot, meter, &mut explanations)?;
    derive_archive_absence(snapshot, meter, &mut explanations)?;
    Ok(explanations)
}

/// Index rows by key; the last occurrence wins, exactly as the validator's observation index.
fn effective_occurrences<'a, T>(
    rows: &'a [T],
    key: impl Fn(&'a T) -> &'a str,
    meter: &mut StepMeter,
) -> Result<BTreeMap<&'a str, usize>, StepsExhausted> {
    let mut index = BTreeMap::new();
    for (occurrence, row) in rows.iter().enumerate() {
        meter.charge()?;
        index.insert(key(row), occurrence);
    }
    Ok(index)
}

#[derive(Clone, Copy)]
struct OccurrencePair<'a> {
    subject: &'a str,
    expected: usize,
    observed: usize,
}

fn derive_git_lock_drift(
    snapshot: &ReleaseDependencyInput,
    meter: &mut StepMeter,
    explanations: &mut Vec<Explanation>,
) -> Result<(), StepsExhausted> {
    let observations = effective_occurrences(&snapshot.observations, |row| row.manifest_dependency.as_str(), meter)?;
    for (expected, expectation) in snapshot.dependencies.iter().enumerate() {
        meter.charge()?;
        // A missing observation is the validator's `MissingSourceRow`, never lock drift.
        let Some(&observed) = observations.get(expectation.manifest_dependency.as_str()) else {
            continue;
        };
        let observation = &snapshot.observations[observed];
        let pair = OccurrencePair {
            subject: &expectation.manifest_dependency,
            expected,
            observed,
        };
        let source = observation.lock_source_coordinate.as_deref();
        push_drift(GIT_LOCK_SOURCE, pair, &expectation.source_coordinate, source, meter, explanations)?;
        let revision = observation.lock_revision.as_deref();
        push_drift(GIT_LOCK_REVISION, pair, &expectation.immutable_revision, revision, meter, explanations)?;
    }
    Ok(())
}

fn derive_registry_lock_drift(
    snapshot: &ReleaseDependencyInput,
    meter: &mut StepMeter,
    explanations: &mut Vec<Explanation>,
) -> Result<(), StepsExhausted> {
    let observations =
        effective_occurrences(&snapshot.registry_observations, |row| row.manifest_dependency.as_str(), meter)?;
    for (expected, expectation) in snapshot.registry_dependencies.iter().enumerate() {
        meter.charge()?;
        let Some(&observed) = observations.get(expectation.manifest_dependency.as_str()) else {
            continue;
        };
        let observation = &snapshot.registry_observations[observed];
        let pair = OccurrencePair {
            subject: &expectation.manifest_dependency,
            expected,
            observed,
        };
        let version = observation.lock_package_version.as_deref();
        push_drift(REGISTRY_LOCK_VERSION, pair, &expectation.package_version, version, meter, explanations)?;
        let source = observation.lock_source_coordinate.as_deref();
        push_drift(REGISTRY_LOCK_SOURCE, pair, &expectation.source_coordinate, source, meter, explanations)?;
        let checksum = observation.lock_checksum.as_deref();
        push_drift(REGISTRY_LOCK_CHECKSUM, pair, &expectation.checksum, checksum, meter, explanations)?;
    }
    Ok(())
}

fn push_drift(
    drift: DriftRule,
    pair: OccurrencePair<'_>,
    expected: &str,
    observed: Option<&str>,
    meter: &mut StepMeter,
    explanations: &mut Vec<Explanation>,
) -> Result<(), StepsExhausted> {
    meter.charge()?;
    if observed == Some(expected) {
        return Ok(());
    }
    meter.charge()?;
    explanations.push(Explanation {
        rule: drift.rule,
        subject: pair.subject.to_owned(),
        support: vec![
            SupportRef::Field {
                relation: drift.expected.0,
                occurrence: pair.expected,
                field: drift.expected.1,
                value: Some(expected.to_owned()),
            },
            SupportRef::Field {
                relation: drift.observed.0,
                occurrence: pair.observed,
                field: drift.observed.1,
                value: observed.map(str::to_owned),
            },
        ],
    });
    Ok(())
}

fn derive_archive_absence(
    snapshot: &ReleaseDependencyInput,
    meter: &mut StepMeter,
    explanations: &mut Vec<Explanation>,
) -> Result<(), StepsExhausted> {
    let observations = effective_occurrences(&snapshot.archive_observations, |row| row.id.as_str(), meter)?;
    for (expected, receipt) in snapshot.archive_receipts.iter().enumerate() {
        meter.charge()?;
        let (rule, evidence) = match observations.get(receipt.id.as_str()) {
            None => (ExplanationRule::ArchiveObservationAbsent, SupportRef::Absent {
                relation: FactRelation::ArchiveObservation,
                key: receipt.id.clone(),
            }),
            Some(&observed) if !snapshot.archive_observations[observed].archive_present => {
                (ExplanationRule::ArchiveReportedAbsent, SupportRef::Field {
                    relation: FactRelation::ArchiveObservation,
                    occurrence: observed,
                    field: ExplainedField::ArchivePresent,
                    value: Some(REPORTED_ABSENT_VALUE.to_owned()),
                })
            }
            Some(_) => continue,
        };
        meter.charge()?;
        let expectation = SupportRef::Field {
            relation: FactRelation::ArchiveExpectation,
            occurrence: expected,
            field: ExplainedField::ArchivePath,
            value: Some(receipt.archive_path.clone()),
        };
        explanations.push(Explanation {
            rule,
            subject: receipt.id.clone(),
            support: vec![expectation, evidence],
        });
    }
    Ok(())
}
