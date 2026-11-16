//! Independent check that displayed explanations use facts from the named snapshot.

use super::CollectionDeclaration;
use super::CollectionState;
use super::DriftRule;
use super::EXPLANATION_RULE_REVISION;
use super::ExplainedField;
use super::Explanation;
use super::ExplanationBinding;
use super::ExplanationRule;
use super::FactRelation;
use super::REPORTED_ABSENT_VALUE;
use super::RuleShape;
use super::SupportRef;
use super::identity::snapshot_identity;
use crate::release_dependency::ReleaseDependencyInput;

const TRUE_VALUE: &str = "true";

/// Why an explanation is not grounded in the named snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceIssue {
    /// The binding names a different snapshot, for example one replaced after a correction.
    StaleSnapshot {
        bound: String,
        actual: String,
    },
    RuleRevision {
        bound: String,
    },
    /// Depth or rendered output exceeds the bound limits.
    LimitsExceeded {
        rule: ExplanationRule,
    },
    SupportShape {
        rule: ExplanationRule,
    },
    MissingOccurrence {
        relation: FactRelation,
        occurrence: usize,
    },
    ValueMismatch {
        relation: FactRelation,
        occurrence: usize,
        field: ExplainedField,
    },
    /// The referenced row belongs to another subject; facts of different packages are never joined.
    CrossSubject {
        relation: FactRelation,
        occurrence: usize,
    },
    /// A duplicate observation the validator does not use (the last occurrence wins).
    NonEffectiveOccurrence {
        relation: FactRelation,
        occurrence: usize,
    },
    AbsenceNotCompleted {
        relation: FactRelation,
    },
    AbsenceContradicted {
        relation: FactRelation,
        key: String,
    },
    /// The referenced facts do not satisfy the rule body.
    NotDerived {
        rule: ExplanationRule,
    },
}

// r[impl molten.project.reproducible_dependencies.explanation_references]
/// Check every explanation against `snapshot` without trusting the evaluator.
pub fn verify_explanation_references(
    snapshot: &ReleaseDependencyInput,
    binding: &ExplanationBinding,
    explanations: &[Explanation],
) -> Result<(), ReferenceIssue> {
    let actual = snapshot_identity(snapshot);
    if binding.snapshot_ref != actual {
        return Err(ReferenceIssue::StaleSnapshot {
            bound: binding.snapshot_ref.clone(),
            actual,
        });
    }
    if binding.rule_revision != EXPLANATION_RULE_REVISION {
        return Err(ReferenceIssue::RuleRevision {
            bound: binding.rule_revision.clone(),
        });
    }
    let mut output_bytes = 0_usize;
    for explanation in explanations {
        output_bytes = output_bytes.saturating_add(explanation.render().len());
        if explanation.rule.depth() > binding.limits.max_depth || output_bytes > binding.limits.max_output_bytes {
            return Err(ReferenceIssue::LimitsExceeded { rule: explanation.rule });
        }
        verify_explanation(snapshot, binding.collections, explanation)?;
    }
    Ok(())
}

fn verify_explanation(
    snapshot: &ReleaseDependencyInput,
    collections: CollectionDeclaration,
    explanation: &Explanation,
) -> Result<(), ReferenceIssue> {
    let rule = explanation.rule;
    let [expected, evidence] = explanation.support.as_slice() else {
        return Err(ReferenceIssue::SupportShape { rule });
    };
    match rule.shape() {
        RuleShape::Drift(drift) => verify_drift(snapshot, explanation, drift, expected, evidence),
        RuleShape::ArchiveObservationAbsent => {
            verify_field(snapshot, explanation, ARCHIVE_PATH, expected)?;
            verify_absence(snapshot, collections, explanation, evidence)
        }
        RuleShape::ArchiveReportedAbsent => {
            verify_field(snapshot, explanation, ARCHIVE_PATH, expected)?;
            let reported = verify_field(snapshot, explanation, ARCHIVE_PRESENT, evidence)?;
            if reported != Some(REPORTED_ABSENT_VALUE) {
                return Err(ReferenceIssue::NotDerived { rule });
            }
            Ok(())
        }
    }
}

const ARCHIVE_PATH: (FactRelation, ExplainedField) = (FactRelation::ArchiveExpectation, ExplainedField::ArchivePath);
const ARCHIVE_PRESENT: (FactRelation, ExplainedField) =
    (FactRelation::ArchiveObservation, ExplainedField::ArchivePresent);

fn verify_drift(
    snapshot: &ReleaseDependencyInput,
    explanation: &Explanation,
    drift: DriftRule,
    expected: &SupportRef,
    observed: &SupportRef,
) -> Result<(), ReferenceIssue> {
    let expected = verify_field(snapshot, explanation, drift.expected, expected)?;
    let observed = verify_field(snapshot, explanation, drift.observed, observed)?;
    if expected.is_none() || observed == expected {
        return Err(ReferenceIssue::NotDerived { rule: drift.rule });
    }
    Ok(())
}

fn verify_absence(
    snapshot: &ReleaseDependencyInput,
    collections: CollectionDeclaration,
    explanation: &Explanation,
    evidence: &SupportRef,
) -> Result<(), ReferenceIssue> {
    let SupportRef::Absent { relation, key } = evidence else {
        return Err(ReferenceIssue::SupportShape { rule: explanation.rule });
    };
    if *relation != FactRelation::ArchiveObservation || *key != explanation.subject {
        return Err(ReferenceIssue::SupportShape { rule: explanation.rule });
    }
    if collections.archive_observations != CollectionState::Completed {
        return Err(ReferenceIssue::AbsenceNotCompleted { relation: *relation });
    }
    if snapshot.archive_observations.iter().any(|row| row.id == *key) {
        return Err(ReferenceIssue::AbsenceContradicted {
            relation: *relation,
            key: key.clone(),
        });
    }
    Ok(())
}

fn verify_field<'a>(
    snapshot: &'a ReleaseDependencyInput,
    explanation: &Explanation,
    slot: (FactRelation, ExplainedField),
    reference: &SupportRef,
) -> Result<Option<&'a str>, ReferenceIssue> {
    let SupportRef::Field {
        relation,
        occurrence,
        field,
        value,
    } = reference
    else {
        return Err(ReferenceIssue::SupportShape { rule: explanation.rule });
    };
    let (relation, occurrence, field) = (*relation, *occurrence, *field);
    if (relation, field) != slot {
        return Err(ReferenceIssue::SupportShape { rule: explanation.rule });
    }
    let resolved = resolve_field(snapshot, explanation.rule, relation, occurrence, field)?;
    if resolved.key != explanation.subject {
        return Err(ReferenceIssue::CrossSubject { relation, occurrence });
    }
    if resolved.value != value.as_deref() {
        return Err(ReferenceIssue::ValueMismatch {
            relation,
            occurrence,
            field,
        });
    }
    if let Some(effective) = effective_occurrence(snapshot, relation, resolved.key)
        && effective != occurrence
    {
        return Err(ReferenceIssue::NonEffectiveOccurrence { relation, occurrence });
    }
    Ok(resolved.value)
}

/// One referenced field: the row's join key and the value (`None` for a present row with a missing
/// field).
struct ResolvedField<'a> {
    key: &'a str,
    value: Option<&'a str>,
}

fn resolve_field(
    snapshot: &ReleaseDependencyInput,
    rule: ExplanationRule,
    relation: FactRelation,
    occurrence: usize,
    field: ExplainedField,
) -> Result<ResolvedField<'_>, ReferenceIssue> {
    let missing = ReferenceIssue::MissingOccurrence { relation, occurrence };
    let shape = ReferenceIssue::SupportShape { rule };
    let (key, value) = match relation {
        FactRelation::DependencyExpectation => {
            let row = snapshot.dependencies.get(occurrence).ok_or(missing)?;
            let value = match field {
                ExplainedField::SourceCoordinate => &row.source_coordinate,
                ExplainedField::ImmutableRevision => &row.immutable_revision,
                _ => return Err(shape),
            };
            (&row.manifest_dependency, Some(value.as_str()))
        }
        FactRelation::DependencyObservation => {
            let row = snapshot.observations.get(occurrence).ok_or(missing)?;
            let value = match field {
                ExplainedField::LockSourceCoordinate => &row.lock_source_coordinate,
                ExplainedField::LockRevision => &row.lock_revision,
                _ => return Err(shape),
            };
            (&row.manifest_dependency, value.as_deref())
        }
        FactRelation::RegistryExpectation => {
            let row = snapshot.registry_dependencies.get(occurrence).ok_or(missing)?;
            let value = match field {
                ExplainedField::PackageVersion => &row.package_version,
                ExplainedField::SourceCoordinate => &row.source_coordinate,
                ExplainedField::Checksum => &row.checksum,
                _ => return Err(shape),
            };
            (&row.manifest_dependency, Some(value.as_str()))
        }
        FactRelation::RegistryObservation => {
            let row = snapshot.registry_observations.get(occurrence).ok_or(missing)?;
            let value = match field {
                ExplainedField::LockPackageVersion => &row.lock_package_version,
                ExplainedField::LockSourceCoordinate => &row.lock_source_coordinate,
                ExplainedField::LockChecksum => &row.lock_checksum,
                _ => return Err(shape),
            };
            (&row.manifest_dependency, value.as_deref())
        }
        FactRelation::ArchiveExpectation => {
            let row = snapshot.archive_receipts.get(occurrence).ok_or(missing)?;
            if field != ExplainedField::ArchivePath {
                return Err(shape);
            }
            (&row.id, Some(row.archive_path.as_str()))
        }
        FactRelation::ArchiveObservation => {
            let row = snapshot.archive_observations.get(occurrence).ok_or(missing)?;
            if field != ExplainedField::ArchivePresent {
                return Err(shape);
            }
            (
                &row.id,
                Some(if row.archive_present {
                    TRUE_VALUE
                } else {
                    REPORTED_ABSENT_VALUE
                }),
            )
        }
    };
    Ok(ResolvedField { key, value })
}

/// For observation relations, the occurrence the validator reads for `key`: the last row with that
/// key.
fn effective_occurrence(snapshot: &ReleaseDependencyInput, relation: FactRelation, key: &str) -> Option<usize> {
    match relation {
        FactRelation::DependencyObservation => {
            snapshot.observations.iter().rposition(|row| row.manifest_dependency == key)
        }
        FactRelation::RegistryObservation => {
            snapshot.registry_observations.iter().rposition(|row| row.manifest_dependency == key)
        }
        FactRelation::ArchiveObservation => snapshot.archive_observations.iter().rposition(|row| row.id == key),
        FactRelation::DependencyExpectation | FactRelation::RegistryExpectation | FactRelation::ArchiveExpectation => {
            None
        }
    }
}
