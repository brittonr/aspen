//! Head-to-head comparison of the structured Rust explanation path with the
//! minimal relational pilot over the fixture corpus (issue #11).
//!
//! Step, fact and byte counts are deterministic and printed by
//! `relational_pilot_agrees_with_structured_explanations_over_corpus`. Wall
//! time is measured outside `molten-core` by the test runner's per-test
//! duration of the two `explanation_cost_*` tests, which run the same corpus
//! the same number of times and include snapshot identity, validation and
//! explanation rendering on both paths.

use alloc::collections::BTreeSet;
use core::hint::black_box;

use super::CollectionDeclaration;
use super::CollectionState;
use super::ExplanationLimits;
use super::ExplanationOutcome;
use super::ExplanationReport;
use super::ExplanationRequest;
use super::corpus::CorpusCase;
use super::corpus::OCTET_ARCHIVE;
use super::corpus::absent_archive_observation;
use super::corpus::corpus;
use super::corpus::selected_validator_rows;
use super::explain_release_dependencies;
use super::identity::snapshot_identity;
use super::relational_pilot::PilotProgram;
use super::verify_explanation_references;
use crate::release_dependency::DiagnosticCode;
use crate::release_dependency::ReleaseDependencyInput;
use crate::release_dependency::validate_release_dependencies;

const COST_ITERATIONS: usize = 2_000;
const COMPARISON_COLUMNS: [&str; 11] = [
    "case",
    "selected_rows",
    "textual_ambiguous",
    "rust_facts",
    "rust_steps",
    "rust_output_bytes",
    "pilot_base_facts",
    "pilot_derived_facts",
    "pilot_steps",
    "pilot_iterations",
    "pilot_text_bytes",
];

fn completed_report(snapshot: &ReleaseDependencyInput) -> ExplanationReport {
    let outcome = explain_release_dependencies(&ExplanationRequest {
        snapshot,
        collections: CollectionDeclaration::COMPLETED,
        limits: ExplanationLimits::MAXIMUM,
    });
    match outcome {
        ExplanationOutcome::Completed(report) => report,
        other => panic!("corpus snapshot must explain completely: {other:?}"),
    }
}

/// Selected rows today's textual diagnostics cannot pin to one row: the subject keys several rows.
fn textual_ambiguous_rows(snapshot: &ReleaseDependencyInput) -> usize {
    selected_validator_rows(snapshot)
        .iter()
        .filter(|(code, subject)| {
            let many = |count: usize| count > 1;
            match code {
                DiagnosticCode::LockDrift => {
                    many(snapshot.dependencies.iter().filter(|row| row.manifest_dependency == *subject).count())
                        || many(snapshot.observations.iter().filter(|row| row.manifest_dependency == *subject).count())
                        || many(
                            snapshot
                                .registry_dependencies
                                .iter()
                                .filter(|row| row.manifest_dependency == *subject)
                                .count(),
                        )
                        || many(
                            snapshot
                                .registry_observations
                                .iter()
                                .filter(|row| row.manifest_dependency == *subject)
                                .count(),
                        )
                }
                _ => {
                    many(snapshot.archive_receipts.iter().filter(|row| row.id == *subject).count())
                        || many(snapshot.archive_observations.iter().filter(|row| row.id == *subject).count())
                }
            }
        })
        .count()
}

fn assert_rule_subjects(case: &CorpusCase, report: &ExplanationReport) {
    let mut actual: Vec<_> = report
        .explanations
        .iter()
        .map(|explanation| (explanation.rule, explanation.subject.clone()))
        .collect();
    actual.sort();
    let mut expected: Vec<_> = case.expected.iter().map(|(rule, subject)| (*rule, (*subject).to_owned())).collect();
    expected.sort();
    assert_eq!(actual, expected, "{}: explained rows", case.name);
}

// r[verify molten.project.reproducible_dependencies.explanation_comparison]
#[test]
fn relational_pilot_agrees_with_structured_explanations_over_corpus() {
    let program = PilotProgram::new();
    let mut table = vec![COMPARISON_COLUMNS.join("\t")];
    let mut totals = [0_usize; 10];
    for case in corpus() {
        let report = completed_report(&case.snapshot);
        assert_rule_subjects(&case, &report);
        let explained: BTreeSet<_> = report.explanations.iter().map(|row| (row.code(), row.subject.clone())).collect();
        let selected = selected_validator_rows(&case.snapshot);
        assert_eq!(selected, explained, "{}: validator rows", case.name);
        let pilot = program.explain(&case.snapshot, CollectionDeclaration::COMPLETED);
        assert_eq!(pilot.explanations, report.explanations, "{}: pilot explanations", case.name);
        verify_explanation_references(&case.snapshot, &report.binding, &pilot.explanations)
            .unwrap_or_else(|issue| panic!("{}: pilot references: {issue:?}", case.name));

        let row = [
            selected.len(),
            textual_ambiguous_rows(&case.snapshot),
            report.usage.facts,
            report.usage.steps,
            report.usage.output_bytes,
            pilot.metrics.base_facts,
            pilot.metrics.derived_facts,
            pilot.metrics.steps,
            pilot.metrics.iterations,
            pilot.metrics.materialized_text_bytes,
        ];
        for (total, value) in totals.iter_mut().zip(row) {
            *total += value;
        }
        table.push(format!("{}\t{}", case.name, row.map(|value| value.to_string()).join("\t")));
    }
    table.push(format!("total\t{}", totals.map(|value| value.to_string()).join("\t")));
    println!("release-dependency explanation comparison\n{}", table.join("\n"));
}

// r[verify molten.project.reproducible_dependencies.explanation_comparison]
#[test]
fn pilot_absence_rule_silently_underderives_without_a_completed_relation() {
    let snapshot = absent_archive_observation();
    let collections = CollectionDeclaration {
        archive_observations: CollectionState::Incomplete,
        ..CollectionDeclaration::COMPLETED
    };
    let outcome = explain_release_dependencies(&ExplanationRequest {
        snapshot: &snapshot,
        collections,
        limits: ExplanationLimits::MAXIMUM,
    });
    assert!(matches!(outcome, ExplanationOutcome::Incomplete { .. }));
    assert!(!outcome.admits_release());

    let pilot = PilotProgram::new().explain(&snapshot, collections);
    assert!(
        pilot.explanations.iter().all(|row| row.subject != OCTET_ARCHIVE),
        "the guarded absence rule derives nothing and reports no refusal on its own"
    );
}

#[test]
fn explanation_cost_structured_rust_path() {
    let cases = corpus();
    let expected: usize = cases.iter().map(|case| case.expected.len()).sum();
    let mut explained = 0;
    for _ in 0..COST_ITERATIONS {
        for case in &cases {
            let outcome = explain_release_dependencies(&ExplanationRequest {
                snapshot: &case.snapshot,
                collections: CollectionDeclaration::COMPLETED,
                limits: ExplanationLimits::MAXIMUM,
            });
            explained += outcome.report().map_or(0, |report| report.explanations.len());
        }
    }
    assert_eq!(explained, expected * COST_ITERATIONS);
}

#[test]
fn explanation_cost_relational_pilot() {
    let cases = corpus();
    let expected: usize = cases.iter().map(|case| case.expected.len()).sum();
    let program = PilotProgram::new();
    let mut explained = 0;
    for _ in 0..COST_ITERATIONS {
        for case in &cases {
            // Same surrounding work as the Rust path: identity, validation, rendering.
            black_box(snapshot_identity(&case.snapshot));
            black_box(validate_release_dependencies(&case.snapshot).is_valid());
            let result = program.explain(&case.snapshot, CollectionDeclaration::COMPLETED);
            black_box(result.explanations.iter().map(|row| row.render().len()).sum::<usize>());
            explained += result.explanations.len();
        }
    }
    assert_eq!(explained, expected * COST_ITERATIONS);
}
