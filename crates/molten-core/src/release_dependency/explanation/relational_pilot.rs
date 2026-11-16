//! Minimal Datalog-style alternative measured against the structured Rust path (issue #11).
//!
//! Test-only: this is the comparison subject, not a production rule engine.
//! It projects one snapshot into set-valued relations whose first column is the
//! row occurrence (so set semantics cannot erase duplicate rows), then runs
//! stratified, non-recursive rules by naive bottom-up evaluation with
//! nested-loop joins until each stratum reaches a fixpoint. Derived heads carry
//! their supporting occurrences and values, which become the same
//! [`Explanation`] shape the Rust path produces.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::CollectionDeclaration;
use super::CollectionState;
use super::ExplainedField;
use super::Explanation;
use super::ExplanationRule;
use super::FactRelation;
use super::SupportRef;
use crate::release_dependency::ReleaseDependencyInput;

const DEPENDENCY_EXPECTATION: &str = "dep_exp";
const DEPENDENCY_OBSERVATION: &str = "dep_obs";
const DEPENDENCY_SUPERSEDED: &str = "dep_superseded";
const DEPENDENCY_EFFECTIVE: &str = "dep_eff";
const REGISTRY_EXPECTATION: &str = "reg_exp";
const REGISTRY_OBSERVATION: &str = "reg_obs";
const REGISTRY_SUPERSEDED: &str = "reg_superseded";
const REGISTRY_EFFECTIVE: &str = "reg_eff";
const ARCHIVE_EXPECTATION: &str = "arc_exp";
const ARCHIVE_OBSERVATION: &str = "arc_obs";
const ARCHIVE_SUPERSEDED: &str = "arc_superseded";
const ARCHIVE_EFFECTIVE: &str = "arc_eff";
const ARCHIVE_SEEN: &str = "arc_seen";
const COMPLETED: &str = "completed";
const GIT_LOCK_SOURCE: &str = "git_lock_source";
const GIT_LOCK_REVISION: &str = "git_lock_revision";
const REGISTRY_LOCK_VERSION: &str = "registry_lock_version";
const REGISTRY_LOCK_SOURCE: &str = "registry_lock_source";
const REGISTRY_LOCK_CHECKSUM: &str = "registry_lock_checksum";
const ARCHIVE_ABSENT: &str = "archive_absent";
const ARCHIVE_REPORTED_ABSENT: &str = "archive_reported_absent";

type FieldSlot = (FactRelation, ExplainedField);

/// Drift heads: `(K, EO, OO, E, X)` with the rule and the expected/observed field slots.
const DRIFT_HEADS: [(&str, ExplanationRule, FieldSlot, FieldSlot); 5] = [
    (
        GIT_LOCK_SOURCE,
        ExplanationRule::GitLockSourceCoordinate,
        (FactRelation::DependencyExpectation, ExplainedField::SourceCoordinate),
        (FactRelation::DependencyObservation, ExplainedField::LockSourceCoordinate),
    ),
    (
        GIT_LOCK_REVISION,
        ExplanationRule::GitLockRevision,
        (FactRelation::DependencyExpectation, ExplainedField::ImmutableRevision),
        (FactRelation::DependencyObservation, ExplainedField::LockRevision),
    ),
    (
        REGISTRY_LOCK_VERSION,
        ExplanationRule::RegistryLockPackageVersion,
        (FactRelation::RegistryExpectation, ExplainedField::PackageVersion),
        (FactRelation::RegistryObservation, ExplainedField::LockPackageVersion),
    ),
    (
        REGISTRY_LOCK_SOURCE,
        ExplanationRule::RegistryLockSourceCoordinate,
        (FactRelation::RegistryExpectation, ExplainedField::SourceCoordinate),
        (FactRelation::RegistryObservation, ExplainedField::LockSourceCoordinate),
    ),
    (
        REGISTRY_LOCK_CHECKSUM,
        ExplanationRule::RegistryLockChecksum,
        (FactRelation::RegistryExpectation, ExplainedField::Checksum),
        (FactRelation::RegistryObservation, ExplainedField::LockChecksum),
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Value {
    Occurrence(usize),
    Text(String),
    Missing,
    Flag(bool),
}

impl Value {
    fn optional(value: Option<&String>) -> Self {
        value.map_or(Self::Missing, |text| Self::Text(text.clone()))
    }

    fn text_bytes(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Occurrence(_) | Self::Missing | Self::Flag(_) => 0,
        }
    }
}

type Tuple = Vec<Value>;
type Binding = Vec<Option<Value>>;
type Database = BTreeMap<&'static str, BTreeSet<Tuple>>;

#[derive(Debug, Clone)]
enum Term {
    Var(usize),
    Const(Value),
    Any,
}

#[derive(Debug, Clone)]
struct Atom {
    relation: &'static str,
    terms: Vec<Term>,
}

#[derive(Debug, Clone, Copy)]
enum Constraint {
    NotEqual(usize, usize),
    Greater(usize, usize),
}

#[derive(Debug, Clone)]
struct Rule {
    head: Atom,
    body: Vec<Atom>,
    negated: Vec<Atom>,
    constraints: Vec<Constraint>,
    variables: usize,
}

/// Work and memory one pilot evaluation used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PilotMetrics {
    pub(super) base_facts: usize,
    pub(super) derived_facts: usize,
    pub(super) materialized_text_bytes: usize,
    pub(super) steps: usize,
    pub(super) iterations: usize,
}

pub(super) struct PilotResult {
    pub(super) explanations: Vec<Explanation>,
    pub(super) metrics: PilotMetrics,
}

pub(super) struct PilotProgram {
    strata: Vec<Vec<Rule>>,
}

impl PilotProgram {
    pub(super) fn new() -> Self {
        Self {
            strata: vec![superseded_rules(), effective_rules(), diagnostic_rules()],
        }
    }

    pub(super) fn explain(&self, snapshot: &ReleaseDependencyInput, collections: CollectionDeclaration) -> PilotResult {
        let mut database = project(snapshot, collections);
        let mut metrics = PilotMetrics {
            base_facts: database.values().map(BTreeSet::len).sum(),
            ..PilotMetrics::default()
        };
        for stratum in &self.strata {
            loop {
                metrics.iterations += 1;
                let mut changed = false;
                for rule in stratum {
                    for tuple in evaluate_rule(&database, rule, &mut metrics.steps) {
                        if database.entry(rule.head.relation).or_default().insert(tuple) {
                            metrics.derived_facts += 1;
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
        }
        metrics.materialized_text_bytes =
            database.values().flatten().flat_map(|tuple| tuple.iter()).map(Value::text_bytes).sum();
        let mut explanations = derived_explanations(&database);
        explanations.sort();
        PilotResult { explanations, metrics }
    }
}

fn project(snapshot: &ReleaseDependencyInput, collections: CollectionDeclaration) -> Database {
    let mut database = Database::new();
    for (occurrence, row) in snapshot.dependencies.iter().enumerate() {
        insert(&mut database, DEPENDENCY_EXPECTATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.manifest_dependency.clone()),
            Value::Text(row.source_coordinate.clone()),
            Value::Text(row.immutable_revision.clone()),
        ]);
    }
    for (occurrence, row) in snapshot.observations.iter().enumerate() {
        insert(&mut database, DEPENDENCY_OBSERVATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.manifest_dependency.clone()),
            Value::optional(row.lock_source_coordinate.as_ref()),
            Value::optional(row.lock_revision.as_ref()),
        ]);
    }
    for (occurrence, row) in snapshot.registry_dependencies.iter().enumerate() {
        insert(&mut database, REGISTRY_EXPECTATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.manifest_dependency.clone()),
            Value::Text(row.package_version.clone()),
            Value::Text(row.source_coordinate.clone()),
            Value::Text(row.checksum.clone()),
        ]);
    }
    for (occurrence, row) in snapshot.registry_observations.iter().enumerate() {
        insert(&mut database, REGISTRY_OBSERVATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.manifest_dependency.clone()),
            Value::optional(row.lock_package_version.as_ref()),
            Value::optional(row.lock_source_coordinate.as_ref()),
            Value::optional(row.lock_checksum.as_ref()),
        ]);
    }
    for (occurrence, row) in snapshot.archive_receipts.iter().enumerate() {
        insert(&mut database, ARCHIVE_EXPECTATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.id.clone()),
            Value::Text(row.archive_path.clone()),
        ]);
    }
    for (occurrence, row) in snapshot.archive_observations.iter().enumerate() {
        insert(&mut database, ARCHIVE_OBSERVATION, vec![
            Value::Occurrence(occurrence),
            Value::Text(row.id.clone()),
            Value::Flag(row.archive_present),
        ]);
    }
    for (relation, state) in collections.states() {
        if state == CollectionState::Completed {
            insert(&mut database, COMPLETED, vec![Value::Text(relation.label().to_owned())]);
        }
    }
    database
}

fn insert(database: &mut Database, relation: &'static str, tuple: Tuple) {
    database.entry(relation).or_default().insert(tuple);
}

fn var(index: usize) -> Term {
    Term::Var(index)
}

fn atom(relation: &'static str, terms: Vec<Term>) -> Atom {
    Atom { relation, terms }
}

fn rule(head: Atom, body: Vec<Atom>, negated: Vec<Atom>, constraints: Vec<Constraint>, variables: usize) -> Rule {
    Rule {
        head,
        body,
        negated,
        constraints,
        variables,
    }
}

/// `superseded(O) :- obs(O, K, ..), obs(O2, K, ..), O2 > O.` and `arc_seen(K) :- arc_obs(_, K, _).`
fn superseded_rules() -> Vec<Rule> {
    let superseded = |head: &'static str, observation: &'static str, wildcards: usize| {
        let row = |occurrence: usize| {
            let mut terms = vec![var(occurrence), var(1)];
            terms.extend(std::iter::repeat_n(Term::Any, wildcards));
            atom(observation, terms)
        };
        rule(atom(head, vec![var(0)]), vec![row(0), row(2)], Vec::new(), vec![Constraint::Greater(2, 0)], 3)
    };
    vec![
        superseded(DEPENDENCY_SUPERSEDED, DEPENDENCY_OBSERVATION, 2),
        superseded(REGISTRY_SUPERSEDED, REGISTRY_OBSERVATION, 3),
        superseded(ARCHIVE_SUPERSEDED, ARCHIVE_OBSERVATION, 1),
        rule(
            atom(ARCHIVE_SEEN, vec![var(0)]),
            vec![atom(ARCHIVE_OBSERVATION, vec![Term::Any, var(0), Term::Any])],
            Vec::new(),
            Vec::new(),
            1,
        ),
    ]
}

/// `eff(O, ..) :- obs(O, ..), not superseded(O).` — the last occurrence wins, as in the validator.
fn effective_rules() -> Vec<Rule> {
    let effective = |head: &'static str, observation: &'static str, superseded: &'static str, columns: usize| {
        let terms: Vec<Term> = (0..columns).map(var).collect();
        rule(
            atom(head, terms.clone()),
            vec![atom(observation, terms)],
            vec![atom(superseded, vec![var(0)])],
            Vec::new(),
            columns,
        )
    };
    vec![
        effective(DEPENDENCY_EFFECTIVE, DEPENDENCY_OBSERVATION, DEPENDENCY_SUPERSEDED, 4),
        effective(REGISTRY_EFFECTIVE, REGISTRY_OBSERVATION, REGISTRY_SUPERSEDED, 5),
        effective(ARCHIVE_EFFECTIVE, ARCHIVE_OBSERVATION, ARCHIVE_SUPERSEDED, 3),
    ]
}

/// Variables: K=0, EO=1, OO=2, E=3, X=4.
fn drift_rule(
    head: &'static str,
    expectation: &'static str,
    observation: &'static str,
    width: usize,
    column: usize,
) -> Rule {
    let row = |occurrence: usize, value: usize| {
        let mut terms = vec![var(occurrence), var(0)];
        terms.extend((2..width).map(|index| if index == column { var(value) } else { Term::Any }));
        terms
    };
    rule(
        atom(head, (0..5).map(var).collect()),
        vec![atom(expectation, row(1, 3)), atom(observation, row(2, 4))],
        Vec::new(),
        vec![Constraint::NotEqual(3, 4)],
        5,
    )
}

fn diagnostic_rules() -> Vec<Rule> {
    vec![
        drift_rule(GIT_LOCK_SOURCE, DEPENDENCY_EXPECTATION, DEPENDENCY_EFFECTIVE, 4, 2),
        drift_rule(GIT_LOCK_REVISION, DEPENDENCY_EXPECTATION, DEPENDENCY_EFFECTIVE, 4, 3),
        drift_rule(REGISTRY_LOCK_VERSION, REGISTRY_EXPECTATION, REGISTRY_EFFECTIVE, 5, 2),
        drift_rule(REGISTRY_LOCK_SOURCE, REGISTRY_EXPECTATION, REGISTRY_EFFECTIVE, 5, 3),
        drift_rule(REGISTRY_LOCK_CHECKSUM, REGISTRY_EXPECTATION, REGISTRY_EFFECTIVE, 5, 4),
        // archive_absent(K, EO, P) :-
        //     arc_exp(EO, K, P), completed("archive-observation"), not arc_seen(K).
        rule(
            atom(ARCHIVE_ABSENT, vec![var(0), var(1), var(2)]),
            vec![
                atom(ARCHIVE_EXPECTATION, vec![var(1), var(0), var(2)]),
                atom(COMPLETED, vec![Term::Const(Value::Text(
                    FactRelation::ArchiveObservation.label().to_owned(),
                ))]),
            ],
            vec![atom(ARCHIVE_SEEN, vec![var(0)])],
            Vec::new(),
            3,
        ),
        // archive_reported_absent(K, EO, OO, P) :- arc_exp(EO, K, P), arc_eff(OO, K, false).
        rule(
            atom(ARCHIVE_REPORTED_ABSENT, vec![var(0), var(1), var(2), var(3)]),
            vec![
                atom(ARCHIVE_EXPECTATION, vec![var(1), var(0), var(3)]),
                atom(ARCHIVE_EFFECTIVE, vec![var(2), var(0), Term::Const(Value::Flag(false))]),
            ],
            Vec::new(),
            Vec::new(),
            4,
        ),
    ]
}

fn evaluate_rule(database: &Database, rule: &Rule, steps: &mut usize) -> Vec<Tuple> {
    let mut bindings: Vec<Binding> = vec![vec![None; rule.variables]];
    for body in &rule.body {
        let mut next = Vec::new();
        for binding in &bindings {
            for tuple in database.get(body.relation).into_iter().flatten() {
                *steps += 1;
                if let Some(extended) = unify(body, tuple, binding) {
                    next.push(extended);
                }
            }
        }
        bindings = next;
    }
    let mut heads = Vec::new();
    for binding in bindings {
        if !rule.constraints.iter().all(|constraint| holds(*constraint, &binding)) {
            continue;
        }
        if rule.negated.iter().any(|negated| matches_any(database, negated, &binding, steps)) {
            continue;
        }
        if let Some(head) = instantiate(&rule.head, &binding) {
            heads.push(head);
        }
    }
    heads
}

fn unify(atom: &Atom, tuple: &Tuple, binding: &Binding) -> Option<Binding> {
    if atom.terms.len() != tuple.len() {
        return None;
    }
    let mut extended = binding.clone();
    for (term, value) in atom.terms.iter().zip(tuple) {
        match term {
            Term::Any => {}
            Term::Const(constant) if constant == value => {}
            Term::Const(_) => return None,
            Term::Var(index) => {
                if extended[*index].as_ref().is_some_and(|bound| bound != value) {
                    return None;
                }
                extended[*index].get_or_insert_with(|| value.clone());
            }
        }
    }
    Some(extended)
}

fn holds(constraint: Constraint, binding: &Binding) -> bool {
    match constraint {
        Constraint::NotEqual(left, right) => binding[left] != binding[right],
        Constraint::Greater(left, right) => binding[left] > binding[right],
    }
}

fn matches_any(database: &Database, atom: &Atom, binding: &Binding, steps: &mut usize) -> bool {
    for tuple in database.get(atom.relation).into_iter().flatten() {
        *steps += 1;
        if unify(atom, tuple, binding).is_some() {
            return true;
        }
    }
    false
}

fn instantiate(head: &Atom, binding: &Binding) -> Option<Tuple> {
    head.terms
        .iter()
        .map(|term| match term {
            Term::Var(index) => binding[*index].clone(),
            Term::Const(value) => Some(value.clone()),
            Term::Any => None,
        })
        .collect()
}

fn derived_explanations(database: &Database) -> Vec<Explanation> {
    let mut explanations = Vec::new();
    for (head, rule, expected_slot, observed_slot) in DRIFT_HEADS {
        for tuple in database.get(head).into_iter().flatten() {
            if let [
                Value::Text(subject),
                Value::Occurrence(eo),
                Value::Occurrence(oo),
                expected,
                observed_value,
            ] = tuple.as_slice()
            {
                explanations.push(Explanation {
                    rule,
                    subject: subject.clone(),
                    support: vec![
                        field(expected_slot, *eo, expected),
                        field(observed_slot, *oo, observed_value),
                    ],
                });
            }
        }
    }
    for tuple in database.get(ARCHIVE_ABSENT).into_iter().flatten() {
        if let [Value::Text(subject), Value::Occurrence(eo), path] = tuple.as_slice() {
            explanations.push(Explanation {
                rule: ExplanationRule::ArchiveObservationAbsent,
                subject: subject.clone(),
                support: vec![
                    field((FactRelation::ArchiveExpectation, ExplainedField::ArchivePath), *eo, path),
                    SupportRef::Absent {
                        relation: FactRelation::ArchiveObservation,
                        key: subject.clone(),
                    },
                ],
            });
        }
    }
    for tuple in database.get(ARCHIVE_REPORTED_ABSENT).into_iter().flatten() {
        if let [Value::Text(subject), Value::Occurrence(eo), Value::Occurrence(oo), path] = tuple.as_slice() {
            explanations.push(Explanation {
                rule: ExplanationRule::ArchiveReportedAbsent,
                subject: subject.clone(),
                support: vec![
                    field((FactRelation::ArchiveExpectation, ExplainedField::ArchivePath), *eo, path),
                    field((FactRelation::ArchiveObservation, ExplainedField::ArchivePresent), *oo, &Value::Flag(false)),
                ],
            });
        }
    }
    explanations
}

fn field(slot: FieldSlot, occurrence: usize, value: &Value) -> SupportRef {
    SupportRef::Field {
        relation: slot.0,
        occurrence,
        field: slot.1,
        value: match value {
            Value::Text(text) => Some(text.clone()),
            Value::Flag(flag) => Some(flag.to_string()),
            Value::Occurrence(_) | Value::Missing => None,
        },
    }
}
