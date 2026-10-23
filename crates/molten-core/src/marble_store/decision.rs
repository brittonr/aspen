use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// The non-claims every spike decision record carries.
pub const SPIKE_NON_CLAIMS: &[&str] = &[
    "the spike changes no dataspace, vat, networking, or replication semantics",
    "the spike does not change the default storage path",
    "marble object identifiers are physical handles, not content identity",
    "measurements record one workload on one host and do not prove runtime superiority",
    "durability covers only batches marble atomically recovered",
];

/// The storage paths a spike compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StoreBackend {
    /// The current Redb-backed Prolly block store.
    RedbCurrent,
    MarbleArt,
}

/// What one measurement records. Lower is better for every metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpikeMetric {
    BatchWrite,
    PointRead,
    Recovery,
    SpaceAmplification,
}

impl SpikeMetric {
    pub const ALL: [Self; 4] = [
        Self::BatchWrite,
        Self::PointRead,
        Self::Recovery,
        Self::SpaceAmplification,
    ];

    pub const fn unit(self) -> MeasurementUnit {
        match self {
            Self::BatchWrite | Self::PointRead | Self::Recovery => MeasurementUnit::Nanoseconds,
            Self::SpaceAmplification => MeasurementUnit::Permille,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeasurementUnit {
    Nanoseconds,
    /// Bytes on disk per thousand live payload bytes.
    Permille,
}

/// One recorded number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpikeMeasurement {
    pub workload_ref: String,
    pub backend: StoreBackend,
    pub metric: SpikeMetric,
    pub unit: MeasurementUnit,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikeVerdict {
    KeepCurrent,
    Replace,
    /// No storage-path decision; allowed without measurements.
    Inconclusive,
}

/// The verb the record uses for its numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimVerb {
    Records,
    Proves,
}

/// The keep-or-replace decision record of the spike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpikeDecision {
    pub verdict: SpikeVerdict,
    pub claim_verb: ClaimVerb,
    pub measurements: Vec<SpikeMeasurement>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpikeDecisionIssue {
    /// The record claims proof from measurements.
    Overclaim,
    MissingMeasurement {
        workload_ref: String,
        backend: StoreBackend,
        metric: SpikeMetric,
    },
    WrongUnit(SpikeMetric),
    DuplicateMeasurement {
        workload_ref: String,
        backend: StoreBackend,
        metric: SpikeMetric,
    },
    /// A replace verdict whose own numbers show the current path is better.
    ReplaceContradicted {
        workload_ref: String,
        metric: SpikeMetric,
    },
    MissingNonClaim(String),
}

type MeasurementKey = (String, StoreBackend, SpikeMetric);

/// Validate the claim boundary of a spike decision record.
///
/// Keep and replace verdicts need every metric for both paths on every
/// recorded workload. A replace verdict is refused when its own numbers favor
/// the current path. The record states what it measured, never that it proves
/// superiority.
// r[impl aspen.marble_store.boundary]
// r[impl aspen.marble_store.spike]
pub fn validate_spike_decision(decision: &SpikeDecision) -> Vec<SpikeDecisionIssue> {
    let mut issues = BTreeSet::new();
    if decision.claim_verb == ClaimVerb::Proves {
        issues.insert(SpikeDecisionIssue::Overclaim);
    }
    let values = index_measurements(&decision.measurements, &mut issues);
    if decision.verdict != SpikeVerdict::Inconclusive {
        check_completeness(&values, &mut issues);
    }
    if decision.verdict == SpikeVerdict::Replace {
        check_replacement(&values, &mut issues);
    }
    for non_claim in SPIKE_NON_CLAIMS {
        if !decision.non_claims.iter().any(|stated| stated == non_claim) {
            issues.insert(SpikeDecisionIssue::MissingNonClaim((*non_claim).to_owned()));
        }
    }
    issues.into_iter().collect()
}

fn index_measurements(
    measurements: &[SpikeMeasurement],
    issues: &mut BTreeSet<SpikeDecisionIssue>,
) -> BTreeMap<MeasurementKey, u64> {
    let mut values = BTreeMap::new();
    for measurement in measurements {
        if measurement.unit != measurement.metric.unit() {
            issues.insert(SpikeDecisionIssue::WrongUnit(measurement.metric));
        }
        let key = (measurement.workload_ref.clone(), measurement.backend, measurement.metric);
        if values.insert(key, measurement.value).is_some() {
            issues.insert(SpikeDecisionIssue::DuplicateMeasurement {
                workload_ref: measurement.workload_ref.clone(),
                backend: measurement.backend,
                metric: measurement.metric,
            });
        }
    }
    values
}

fn workloads(values: &BTreeMap<MeasurementKey, u64>) -> BTreeSet<String> {
    values.keys().map(|(workload_ref, _, _)| workload_ref.clone()).collect()
}

fn check_completeness(values: &BTreeMap<MeasurementKey, u64>, issues: &mut BTreeSet<SpikeDecisionIssue>) {
    let mut recorded = workloads(values);
    if recorded.is_empty() {
        recorded.insert(String::new());
    }
    for workload_ref in recorded {
        for backend in [StoreBackend::RedbCurrent, StoreBackend::MarbleArt] {
            for metric in SpikeMetric::ALL {
                if !values.contains_key(&(workload_ref.clone(), backend, metric)) {
                    issues.insert(SpikeDecisionIssue::MissingMeasurement {
                        workload_ref: workload_ref.clone(),
                        backend,
                        metric,
                    });
                }
            }
        }
    }
}

fn check_replacement(values: &BTreeMap<MeasurementKey, u64>, issues: &mut BTreeSet<SpikeDecisionIssue>) {
    for workload_ref in workloads(values) {
        for metric in SpikeMetric::ALL {
            let current = values.get(&(workload_ref.clone(), StoreBackend::RedbCurrent, metric));
            let candidate = values.get(&(workload_ref.clone(), StoreBackend::MarbleArt, metric));
            if let (Some(current), Some(candidate)) = (current, candidate)
                && candidate > current
            {
                issues.insert(SpikeDecisionIssue::ReplaceContradicted {
                    workload_ref: workload_ref.clone(),
                    metric,
                });
            }
        }
    }
}
