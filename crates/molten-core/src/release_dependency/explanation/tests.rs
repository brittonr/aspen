use super::corpus::BASALT;
use super::corpus::BASALT_OBSERVATION;
use super::corpus::OCTET_ARCHIVE;
use super::corpus::VALENCE;
use super::corpus::VALENCE_OBSERVATION;
use super::corpus::absent_archive_observation;
use super::corpus::absent_dependency_observation;
use super::corpus::corpus;
use super::corpus::crossed_packages;
use super::corpus::duplicate_observation_last_drifted;
use super::corpus::lock_source_drift;
use super::*;
use crate::release_dependency::tests::valid_input;

fn request(snapshot: &ReleaseDependencyInput) -> ExplanationRequest<'_> {
    ExplanationRequest {
        snapshot,
        collections: CollectionDeclaration::COMPLETED,
        limits: ExplanationLimits::MAXIMUM,
    }
}

fn completed(snapshot: &ReleaseDependencyInput) -> ExplanationReport {
    match explain_release_dependencies(&request(snapshot)) {
        ExplanationOutcome::Completed(report) => report,
        other => panic!("expected a completed explanation, got {other:?}"),
    }
}

fn with_limits(snapshot: &ReleaseDependencyInput, limits: ExplanationLimits) -> ExplanationOutcome {
    explain_release_dependencies(&ExplanationRequest {
        limits,
        ..request(snapshot)
    })
}

fn has_code(snapshot: &ReleaseDependencyInput, code: DiagnosticCode, subject: &str) -> bool {
    validate_release_dependencies(snapshot)
        .diagnostics
        .iter()
        .any(|row| row.code == code && row.subject == subject)
}

fn explanation(report: &ExplanationReport, rule: ExplanationRule, subject: &str) -> Explanation {
    report
        .explanations
        .iter()
        .find(|row| row.rule == rule && row.subject == subject)
        .cloned()
        .unwrap_or_else(|| panic!("missing {rule:?} for {subject}"))
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
// r[verify molten.project.reproducible_dependencies.explanation_references]
#[test]
fn corpus_explanations_follow_validator_rows_and_ground_in_their_snapshot() {
    for case in corpus() {
        let report = completed(&case.snapshot);
        let mut explained: Vec<_> = report.explanations.iter().map(|row| (row.rule, row.subject.as_str())).collect();
        explained.sort();
        let mut expected = case.expected.clone();
        expected.sort();
        assert_eq!(explained, expected, "{}", case.name);
        assert_eq!(report.validator_admits, validate_release_dependencies(&case.snapshot).is_valid(), "{}", case.name);
        assert_eq!(report.omitted_explanations, 0, "{}", case.name);
        verify_explanation_references(&case.snapshot, &report.binding, &report.explanations)
            .unwrap_or_else(|issue| panic!("{}: {issue:?}", case.name));
    }
}

#[test]
fn missing_observation_present_missing_field_and_archive_absence_stay_distinct() {
    let absent = absent_dependency_observation();
    assert!(has_code(&absent, DiagnosticCode::MissingSourceRow, BASALT));
    assert!(!has_code(&absent, DiagnosticCode::LockDrift, BASALT));
    assert!(completed(&absent).explanations.is_empty());

    let mut missing_field = valid_input();
    missing_field.observations[VALENCE_OBSERVATION].lock_revision = None;
    let report = completed(&missing_field);
    let drift = explanation(&report, ExplanationRule::GitLockRevision, VALENCE);
    let observed = &drift.support[1];
    assert!(
        matches!(observed, SupportRef::Field { value: None, occurrence, .. } if *occurrence == VALENCE_OBSERVATION)
    );
    assert!(!has_code(&missing_field, DiagnosticCode::MissingSourceRow, VALENCE));

    let absent_archive = completed(&absent_archive_observation());
    let absence = explanation(&absent_archive, ExplanationRule::ArchiveObservationAbsent, OCTET_ARCHIVE);
    assert!(matches!(&absence.support[1], SupportRef::Absent {
        relation: FactRelation::ArchiveObservation,
        ..
    }));
    assert_eq!(absence.code(), DiagnosticCode::MissingArchiveReceipt);
    assert_eq!(absence.rule.depth(), MAX_EXPLANATION_DEPTH);
}

#[test]
fn duplicate_observations_reference_the_occurrence_the_validator_reads() {
    let snapshot = duplicate_observation_last_drifted();
    let report = completed(&snapshot);
    let drift = explanation(&report, ExplanationRule::GitLockRevision, BASALT);
    let last = snapshot.observations.len() - 1;
    assert!(matches!(&drift.support[1], SupportRef::Field { occurrence, .. } if *occurrence == last));
    assert!(has_code(&snapshot, DiagnosticCode::DuplicateDependency, BASALT));

    let mut forged = drift;
    forged.support[1] = SupportRef::Field {
        relation: FactRelation::DependencyObservation,
        occurrence: BASALT_OBSERVATION,
        field: ExplainedField::LockRevision,
        value: snapshot.observations[BASALT_OBSERVATION].lock_revision.clone(),
    };
    assert_eq!(
        verify_explanation_references(&snapshot, &report.binding, &[forged]),
        Err(ReferenceIssue::NonEffectiveOccurrence {
            relation: FactRelation::DependencyObservation,
            occurrence: BASALT_OBSERVATION,
        })
    );
}

#[test]
fn facts_of_two_packages_are_never_joined() {
    let snapshot = crossed_packages();
    let report = completed(&snapshot);
    for row in &report.explanations {
        let SupportRef::Field { occurrence, .. } = &row.support[1] else {
            panic!("drift support must reference an observation field");
        };
        assert_eq!(snapshot.observations[*occurrence].manifest_dependency, row.subject);
    }

    // The valence row now carries exactly basalt's expected lock source: a value join would match
    // it.
    let mut forged = explanation(&report, ExplanationRule::GitLockSourceCoordinate, BASALT);
    forged.support[1] = SupportRef::Field {
        relation: FactRelation::DependencyObservation,
        occurrence: VALENCE_OBSERVATION,
        field: ExplainedField::LockSourceCoordinate,
        value: snapshot.observations[VALENCE_OBSERVATION].lock_source_coordinate.clone(),
    };
    assert_eq!(
        verify_explanation_references(&snapshot, &report.binding, &[forged]),
        Err(ReferenceIssue::CrossSubject {
            relation: FactRelation::DependencyObservation,
            occurrence: VALENCE_OBSERVATION,
        })
    );
}

#[test]
fn support_that_does_not_satisfy_the_rule_is_rejected() {
    let snapshot = valid_input();
    let report = completed(&snapshot);
    let revision = snapshot.dependencies[0].immutable_revision.clone();
    let fabricated = Explanation {
        rule: ExplanationRule::GitLockRevision,
        subject: BASALT.to_owned(),
        support: vec![
            SupportRef::Field {
                relation: FactRelation::DependencyExpectation,
                occurrence: 0,
                field: ExplainedField::ImmutableRevision,
                value: Some(revision.clone()),
            },
            SupportRef::Field {
                relation: FactRelation::DependencyObservation,
                occurrence: BASALT_OBSERVATION,
                field: ExplainedField::LockRevision,
                value: Some(revision),
            },
        ],
    };
    assert_eq!(
        verify_explanation_references(&snapshot, &report.binding, &[fabricated]),
        Err(ReferenceIssue::NotDerived {
            rule: ExplanationRule::GitLockRevision,
        })
    );
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
#[test]
fn replacing_a_snapshot_cannot_retain_stale_conclusions() {
    let drifted = lock_source_drift();
    let stale = completed(&drifted);
    assert_eq!(stale.explanations.len(), 1);

    let corrected = valid_input();
    let fresh = completed(&corrected);
    assert!(fresh.explanations.is_empty());
    assert_ne!(fresh.binding.snapshot_ref, stale.binding.snapshot_ref);
    assert!(matches!(
        verify_explanation_references(&corrected, &stale.binding, &stale.explanations),
        Err(ReferenceIssue::StaleSnapshot { .. })
    ));
    assert_eq!(
        verify_explanation_references(&corrected, &fresh.binding, &stale.explanations),
        Err(ReferenceIssue::ValueMismatch {
            relation: FactRelation::DependencyObservation,
            occurrence: BASALT_OBSERVATION,
            field: ExplainedField::LockSourceCoordinate,
        })
    );

    let mut removed = drifted;
    removed.observations.remove(BASALT_OBSERVATION);
    let after_removal = completed(&removed);
    assert!(after_removal.explanations.is_empty());
    assert!(has_code(&removed, DiagnosticCode::MissingSourceRow, BASALT));
    assert_eq!(
        verify_explanation_references(&removed, &after_removal.binding, &stale.explanations),
        Err(ReferenceIssue::MissingOccurrence {
            relation: FactRelation::DependencyObservation,
            occurrence: BASALT_OBSERVATION,
        })
    );
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
#[test]
fn unavailable_and_incomplete_collections_refuse_instead_of_passing() {
    let snapshot = valid_input();
    assert!(validate_release_dependencies(&snapshot).is_valid());
    let cases = [
        (
            CollectionDeclaration {
                archive_observations: CollectionState::Incomplete,
                ..CollectionDeclaration::COMPLETED
            },
            FactRelation::ArchiveObservation,
            false,
        ),
        (
            CollectionDeclaration {
                dependency_observations: CollectionState::Unavailable,
                ..CollectionDeclaration::COMPLETED
            },
            FactRelation::DependencyObservation,
            true,
        ),
        (
            CollectionDeclaration {
                registry_observations: CollectionState::Incomplete,
                ..CollectionDeclaration::COMPLETED
            },
            FactRelation::RegistryObservation,
            false,
        ),
    ];
    for (collections, relation, unavailable) in cases {
        let outcome = explain_release_dependencies(&ExplanationRequest {
            collections,
            ..request(&snapshot)
        });
        assert!(!outcome.admits_release(), "{relation:?}");
        assert!(outcome.report().is_none(), "{relation:?}");
        let refused = if unavailable {
            matches!(&outcome, ExplanationOutcome::Unavailable { relation: actual, .. } if *actual == relation)
        } else {
            matches!(&outcome, ExplanationOutcome::Incomplete { relation: actual, .. } if *actual == relation)
        };
        assert!(refused, "{outcome:?}");
    }
    assert!(completed(&snapshot).validator_admits);
}

#[test]
fn absence_support_requires_a_completed_relation() {
    let snapshot = absent_archive_observation();
    let report = completed(&snapshot);
    let mut binding = report.binding.clone();
    binding.collections.archive_observations = CollectionState::Incomplete;
    assert_eq!(
        verify_explanation_references(&snapshot, &binding, &report.explanations),
        Err(ReferenceIssue::AbsenceNotCompleted {
            relation: FactRelation::ArchiveObservation,
        })
    );

    let mut present = snapshot.clone();
    present.archive_observations.push(valid_input().archive_observations[0].clone());
    let mut rebound = report.binding.clone();
    rebound.snapshot_ref = completed(&present).binding.snapshot_ref;
    assert_eq!(
        verify_explanation_references(&present, &rebound, &report.explanations),
        Err(ReferenceIssue::AbsenceContradicted {
            relation: FactRelation::ArchiveObservation,
            key: OCTET_ARCHIVE.to_owned(),
        })
    );
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
#[test]
fn unsupported_rows_and_limits_refuse() {
    let mut oversized = valid_input();
    let extra = oversized.observations[0].clone();
    oversized.observations.resize(MAX_DEPENDENCY_ROWS + 1, extra);
    let outcome = explain_release_dependencies(&request(&oversized));
    assert!(matches!(
        outcome,
        ExplanationOutcome::Unsupported {
            reason: UnsupportedInput::RowBound {
                relation: FactRelation::DependencyObservation,
                observed,
                maximum: MAX_DEPENDENCY_ROWS,
            },
            ..
        } if observed == MAX_DEPENDENCY_ROWS + 1
    ));
    assert!(!outcome.admits_release());
    assert!(has_code(&oversized, DiagnosticCode::BoundExceeded, "dependency observations"));

    let snapshot = valid_input();
    for limits in [
        ExplanationLimits {
            max_steps: 0,
            ..ExplanationLimits::MAXIMUM
        },
        ExplanationLimits {
            max_steps: MAX_EXPLANATION_STEPS + 1,
            ..ExplanationLimits::MAXIMUM
        },
        ExplanationLimits {
            max_depth: MAX_EXPLANATION_DEPTH + 1,
            ..ExplanationLimits::MAXIMUM
        },
        ExplanationLimits {
            max_output_bytes: MAX_EXPLANATION_OUTPUT_BYTES + 1,
            ..ExplanationLimits::MAXIMUM
        },
    ] {
        let outcome = with_limits(&snapshot, limits);
        assert!(
            matches!(outcome, ExplanationOutcome::Unsupported {
                reason: UnsupportedInput::Limits,
                ..
            }),
            "{limits:?}"
        );
        assert!(!outcome.admits_release());
    }
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
#[test]
fn budget_exhaustion_refuses_without_a_pass() {
    let valid = valid_input();
    let facts = with_limits(&valid, ExplanationLimits {
        max_facts: 7,
        ..ExplanationLimits::MAXIMUM
    });
    assert!(matches!(facts, ExplanationOutcome::BudgetExhausted {
        budget: ExplanationBudget::Facts,
        limit: 7,
        ..
    }));
    assert!(!facts.admits_release());

    let steps = with_limits(&valid, ExplanationLimits {
        max_steps: 3,
        ..ExplanationLimits::MAXIMUM
    });
    assert!(matches!(steps, ExplanationOutcome::BudgetExhausted {
        budget: ExplanationBudget::Steps,
        limit: 3,
        ..
    }));
    assert!(!steps.admits_release());

    let shallow = ExplanationLimits {
        max_depth: 1,
        ..ExplanationLimits::MAXIMUM
    };
    let depth = with_limits(&absent_archive_observation(), shallow);
    assert!(matches!(depth, ExplanationOutcome::BudgetExhausted {
        budget: ExplanationBudget::Depth,
        limit: 1,
        ..
    }));
    assert!(matches!(with_limits(&lock_source_drift(), shallow), ExplanationOutcome::Completed(_)));
    assert!(matches!(with_limits(&valid, shallow), ExplanationOutcome::Completed(report) if report.validator_admits));
}

// r[verify molten.project.reproducible_dependencies.explanation_outcomes]
#[test]
fn output_bound_truncates_explicitly() {
    let snapshot = crossed_packages();
    let full = completed(&snapshot);
    assert_eq!(full.explanations.len(), 4);
    let first = full.explanations[0].render().len();
    let limits = ExplanationLimits {
        max_output_bytes: first,
        ..ExplanationLimits::MAXIMUM
    };
    let ExplanationOutcome::Truncated(report) = with_limits(&snapshot, limits) else {
        panic!("a bound below the full output must truncate explicitly");
    };
    assert_eq!(report.explanations, full.explanations[..1]);
    assert_eq!(report.omitted_explanations, 3);
    assert_eq!(report.usage.output_bytes, first);
    assert!(!report.validator_admits);
    verify_explanation_references(&snapshot, &report.binding, &report.explanations).expect("shown explanations");
    assert_eq!(
        verify_explanation_references(&snapshot, &report.binding, &full.explanations),
        Err(ReferenceIssue::LimitsExceeded {
            rule: full.explanations[1].rule,
        })
    );

    let valid = valid_input();
    let quiet = with_limits(&valid, ExplanationLimits {
        max_output_bytes: 1,
        ..ExplanationLimits::MAXIMUM
    });
    assert!(matches!(&quiet, ExplanationOutcome::Completed(report) if report.explanations.is_empty()));
    assert!(quiet.admits_release());
}

#[test]
fn snapshot_identity_binds_occurrence_order_and_multiplicity() {
    let base = valid_input();
    let base_report = completed(&base);

    let mut reordered = base.clone();
    reordered.observations.reverse();
    let reordered_report = completed(&reordered);
    assert_ne!(reordered_report.binding.snapshot_ref, base_report.binding.snapshot_ref);
    assert_eq!(
        validate_release_dependencies(&reordered).canonical_material,
        validate_release_dependencies(&base).canonical_material
    );

    let mut repeated = base.clone();
    repeated.observations.push(base.observations[BASALT_OBSERVATION].clone());
    assert_ne!(completed(&repeated).binding.snapshot_ref, base_report.binding.snapshot_ref);

    let mut limited = base_report.binding.clone();
    limited.limits.max_steps -= 1;
    assert_ne!(limited.binding_ref(), base_report.binding.binding_ref());
    assert_eq!(completed(&base).binding.binding_ref(), base_report.binding.binding_ref());
}
