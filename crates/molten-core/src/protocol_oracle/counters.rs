use std::collections::BTreeMap;

use super::*;

/// One observation of a named work counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterObservation {
    pub name: String,
    pub generation: u64,
    pub value: u64,
}

/// The final value of every declared counter in one run.
pub type CounterTotals = BTreeMap<String, u64>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CounterIssue {
    UndeclaredCounter(String),
    /// A monotonic counter decreased without a reset its schema allows.
    Regression {
        name: String,
        previous: u64,
        value: u64,
    },
    /// The generation moved backwards.
    GenerationRegression(String),
}

/// Admit a run's counter observations in order.
///
/// Each counter is monotonic. It may restart only at a higher generation, and
/// only when its schema allows a generation reset.
// r[impl molten.fabric_simulation.protocol_cost]
pub fn admit_counters(
    schema: &[CounterSchema],
    observations: &[CounterObservation],
) -> Result<CounterTotals, Vec<CounterIssue>> {
    let mut issues = Vec::with_capacity(observations.len());
    let mut last = BTreeMap::<String, (u64, u64)>::new();
    for observation in observations {
        let Some(declared) = schema.iter().find(|counter| counter.name == observation.name) else {
            issues.push(CounterIssue::UndeclaredCounter(observation.name.clone()));
            continue;
        };
        if let Some((generation, value)) = last.get(&observation.name).copied() {
            if observation.generation < generation {
                issues.push(CounterIssue::GenerationRegression(observation.name.clone()));
                continue;
            }
            let is_reset_allowed = declared.resets_on_generation && observation.generation > generation;
            if observation.value < value && !is_reset_allowed {
                issues.push(CounterIssue::Regression {
                    name: observation.name.clone(),
                    previous: value,
                    value: observation.value,
                });
                continue;
            }
        }
        last.insert(observation.name.clone(), (observation.generation, observation.value));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(last.into_iter().map(|(name, (_, value))| (name, value)).collect())
}

/// The identities a counter comparison must share exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterCohort {
    pub world_ref: String,
    pub runtime_ref: String,
    pub scheduler_ref: String,
    pub workload_ref: String,
    pub fault_ref: String,
    pub counter_schema_ref: String,
    pub bounds_ref: String,
}

/// The exact difference of one counter between two runs of one cohort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterDelta {
    pub name: String,
    pub baseline: u64,
    pub candidate: u64,
    pub delta: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterComparisonIssue {
    /// The two runs differ in an identity the comparison binds.
    CohortMismatch(&'static str),
    /// A counter appears in one run only.
    CounterSetMismatch(String),
}

/// Compare the counters of two runs of one deterministic cohort.
///
/// The result is exact algorithmic work differences. It carries no latency,
/// throughput, capacity, or environment-independent performance claim.
// r[impl molten.fabric_simulation.protocol_cost]
pub fn compare_counters(
    baseline: (&CounterCohort, &CounterTotals),
    candidate: (&CounterCohort, &CounterTotals),
) -> Result<Vec<CounterDelta>, CounterComparisonIssue> {
    let (left, right) = (baseline.0, candidate.0);
    for (field, is_equal) in [
        ("world", left.world_ref == right.world_ref),
        ("runtime", left.runtime_ref == right.runtime_ref),
        ("scheduler", left.scheduler_ref == right.scheduler_ref),
        ("workload", left.workload_ref == right.workload_ref),
        ("faults", left.fault_ref == right.fault_ref),
        ("counter-schema", left.counter_schema_ref == right.counter_schema_ref),
        ("bounds", left.bounds_ref == right.bounds_ref),
    ] {
        if !is_equal {
            return Err(CounterComparisonIssue::CohortMismatch(field));
        }
    }
    if let Some(name) = baseline.1.keys().find(|name| !candidate.1.contains_key(*name)) {
        return Err(CounterComparisonIssue::CounterSetMismatch(name.clone()));
    }
    if let Some(name) = candidate.1.keys().find(|name| !baseline.1.contains_key(*name)) {
        return Err(CounterComparisonIssue::CounterSetMismatch(name.clone()));
    }
    Ok(baseline
        .1
        .iter()
        .zip(candidate.1.values())
        .map(|((name, value), other)| CounterDelta {
            name: name.clone(),
            baseline: *value,
            candidate: *other,
            delta: i128::from(*other) - i128::from(*value),
        })
        .collect())
}
