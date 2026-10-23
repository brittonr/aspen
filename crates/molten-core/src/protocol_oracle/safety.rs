use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

/// The extension-owned meaning of a protocol's projections.
///
/// The extension names its local rules and checks one participant's step from
/// its previous projection to the next. The check reads projection values
/// only; it never calls the transition under test.
pub trait ProtocolSemantics {
    /// The local rules of a service, in a stable order.
    fn local_rules(&self, service: &str) -> &'static [&'static str];

    /// The rules the step from `previous` to `next` violates.
    fn check_local(&self, previous: Option<&ProtocolProjection>, next: &ProtocolProjection) -> Vec<&'static str>;
}

/// The first observed failure of a rule, kept even when later projections agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureWitness {
    pub run_sequence: u64,
    pub participant: String,
    pub scheduler_choice_ref: String,
    pub projection_ref: String,
    /// The other side of a pairwise, cohort, or durability disagreement.
    pub other_projection_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleVerdict {
    Pass,
    Fail(FailureWitness),
    /// Required observations are missing; neither pass nor fail.
    Incomplete,
    /// The facts the rule needs do not exist in this cohort.
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyResult {
    pub level: SafetyLevel,
    pub service: String,
    pub rule: String,
    pub verdict: OracleVerdict,
}

/// Evaluate every safety level a profile selects over one cohort.
///
/// Projections are visited in run order and the earliest failure of each rule
/// is kept. An incomplete cohort cannot pass: a rule with no failure reports
/// incomplete instead.
// r[impl molten.fabric_simulation.protocol_safety]
// r[impl molten.fabric_simulation.oracle_independence]
pub fn evaluate_safety(
    profile: &ProtocolOracleProfile,
    cohort: &ProtocolCohort,
    semantics: &dyn ProtocolSemantics,
) -> Vec<SafetyResult> {
    let mut results = Vec::with_capacity(profile.services.len() * 4);
    for service in &profile.services {
        for level in &service.safety {
            match level {
                SafetyLevel::Local => {
                    for rule in semantics.local_rules(&service.service) {
                        results.push(result(service, *level, rule, local_failure(cohort, service, rule, semantics)));
                    }
                }
                SafetyLevel::Pairwise | SafetyLevel::Cohort => {
                    results.push(result(
                        service,
                        *level,
                        level_rule(*level),
                        agreement(profile, cohort, service, *level),
                    ));
                }
                SafetyLevel::Durability => {
                    results.push(result(service, *level, level_rule(*level), durability(cohort, service)));
                }
            }
        }
    }
    if !cohort.is_complete() {
        for item in &mut results {
            if item.verdict == OracleVerdict::Pass {
                item.verdict = OracleVerdict::Incomplete;
            }
        }
    }
    results
}

const fn level_rule(level: SafetyLevel) -> &'static str {
    match level {
        SafetyLevel::Local => "local",
        SafetyLevel::Pairwise => "pairwise-agreement",
        SafetyLevel::Cohort => "cohort-agreement",
        SafetyLevel::Durability => "recovered-state-matches-durable-state",
    }
}

fn result(service: &ServiceProfile, level: SafetyLevel, rule: &str, verdict: OracleVerdict) -> SafetyResult {
    SafetyResult {
        level,
        service: service.service.clone(),
        rule: rule.to_owned(),
        verdict,
    }
}

fn witness(admitted: &AdmittedProjection, other: Option<&AdmittedProjection>) -> FailureWitness {
    FailureWitness {
        run_sequence: admitted.projection.run_sequence,
        participant: admitted.projection.participant.clone(),
        scheduler_choice_ref: admitted.projection.scheduler_choice_ref.clone(),
        projection_ref: admitted.projection_ref.clone(),
        other_projection_ref: other.map(|other| other.projection_ref.clone()),
    }
}

fn local_failure(
    cohort: &ProtocolCohort,
    service: &ServiceProfile,
    rule: &str,
    semantics: &dyn ProtocolSemantics,
) -> OracleVerdict {
    let mut previous = BTreeMap::<&str, &ProtocolProjection>::new();
    let mut is_evaluated = false;
    for admitted in cohort.projections.iter().filter(|admitted| admitted.projection.service == service.service) {
        let participant = admitted.projection.participant.as_str();
        is_evaluated = true;
        if semantics.check_local(previous.get(participant).copied(), &admitted.projection).contains(&rule) {
            return OracleVerdict::Fail(witness(admitted, None));
        }
        previous.insert(participant, &admitted.projection);
    }
    if is_evaluated {
        OracleVerdict::Pass
    } else {
        OracleVerdict::NotEvaluated
    }
}

/// Participants of one service must agree at every shared logical position:
/// each participant's first projection at a position must match the first
/// participant's.
fn agreement(
    profile: &ProtocolOracleProfile,
    cohort: &ProtocolCohort,
    service: &ServiceProfile,
    level: SafetyLevel,
) -> OracleVerdict {
    let mut first_at_position = BTreeMap::<u64, &AdmittedProjection>::new();
    let mut seen = BTreeSet::<(u64, &str)>::new();
    let mut is_compared = false;
    for admitted in cohort.projections.iter().filter(|admitted| admitted.projection.service == service.service) {
        let position = admitted.projection.logical_position;
        if !seen.insert((position, admitted.projection.participant.as_str())) {
            continue;
        }
        let Some(first) = first_at_position.get(&position) else {
            first_at_position.insert(position, admitted);
            continue;
        };
        is_compared = true;
        let is_physical = service.physical_layout == PhysicalLayout::CanonicalPhysical;
        let is_disagreement = first.projection.state_ref != admitted.projection.state_ref
            || (is_physical && first.projection.physical_ref != admitted.projection.physical_ref);
        if is_disagreement {
            return OracleVerdict::Fail(witness(admitted, Some(first)));
        }
    }
    if level == SafetyLevel::Cohort {
        let required = profile
            .participants
            .iter()
            .filter(|participant| participant.service == service.service && participant.is_required)
            .count();
        let is_partial = first_at_position
            .keys()
            .any(|position| seen.iter().filter(|(seen_position, _)| seen_position == position).count() < required);
        if is_partial {
            return OracleVerdict::Incomplete;
        }
    }
    if is_compared {
        OracleVerdict::Pass
    } else {
        OracleVerdict::NotEvaluated
    }
}

/// A recovered projection must restore a state the participant actually held
/// at the same logical position before the recovery.
fn durability(cohort: &ProtocolCohort, service: &ServiceProfile) -> OracleVerdict {
    let mut held = BTreeMap::<(&str, u64), (BTreeSet<&str>, &AdmittedProjection)>::new();
    let mut is_compared = false;
    let mut is_missing_reference = false;
    for admitted in cohort.projections.iter().filter(|admitted| admitted.projection.service == service.service) {
        let key = (admitted.projection.participant.as_str(), admitted.projection.logical_position);
        if admitted.projection.durability == DurabilityFact::Recovered {
            match held.get(&key) {
                Some((states, latest)) => {
                    is_compared = true;
                    if !states.contains(admitted.projection.state_ref.as_str()) {
                        return OracleVerdict::Fail(witness(admitted, Some(latest)));
                    }
                }
                None => is_missing_reference = true,
            }
        }
        let entry = held.entry(key).or_insert_with(|| (BTreeSet::new(), admitted));
        entry.0.insert(admitted.projection.state_ref.as_str());
        entry.1 = admitted;
    }
    if is_missing_reference {
        OracleVerdict::Incomplete
    } else if is_compared {
        OracleVerdict::Pass
    } else {
        OracleVerdict::NotEvaluated
    }
}
