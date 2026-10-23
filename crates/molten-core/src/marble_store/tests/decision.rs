use super::*;

const WORKLOAD: &str = "prolly-shell-64x8";

fn complete(marble_better: bool) -> Vec<SpikeMeasurement> {
    let mut measurements = Vec::with_capacity(8);
    for metric in SpikeMetric::ALL {
        for (backend, value) in [
            (StoreBackend::RedbCurrent, 100),
            (StoreBackend::MarbleArt, if marble_better { 50 } else { 200 }),
        ] {
            measurements.push(SpikeMeasurement {
                workload_ref: WORKLOAD.to_owned(),
                backend,
                metric,
                unit: metric.unit(),
                value,
            });
        }
    }
    measurements
}

fn decision(verdict: SpikeVerdict, measurements: Vec<SpikeMeasurement>) -> SpikeDecision {
    SpikeDecision {
        verdict,
        claim_verb: ClaimVerb::Records,
        measurements,
        non_claims: SPIKE_NON_CLAIMS.iter().map(|claim| (*claim).to_owned()).collect(),
    }
}

// r[verify aspen.marble_store.boundary]
// r[verify aspen.marble_store.spike]
#[test]
fn a_measured_keep_decision_that_records_its_numbers_validates() {
    assert!(validate_spike_decision(&decision(SpikeVerdict::KeepCurrent, complete(false))).is_empty());
    assert!(validate_spike_decision(&decision(SpikeVerdict::Replace, complete(true))).is_empty());
    assert!(validate_spike_decision(&decision(SpikeVerdict::Inconclusive, Vec::new())).is_empty());
}

// r[verify aspen.marble_store.boundary]
#[test]
fn a_superiority_claim_without_recorded_numbers_fails_verification() {
    let unmeasured = validate_spike_decision(&decision(SpikeVerdict::Replace, Vec::new()));
    assert!(unmeasured.iter().any(|issue| matches!(issue, SpikeDecisionIssue::MissingMeasurement { .. })));

    let mut partial = complete(true);
    partial.retain(|measurement| measurement.metric != SpikeMetric::Recovery);
    let issues = validate_spike_decision(&decision(SpikeVerdict::Replace, partial));
    assert!(issues.contains(&SpikeDecisionIssue::MissingMeasurement {
        workload_ref: WORKLOAD.to_owned(),
        backend: StoreBackend::MarbleArt,
        metric: SpikeMetric::Recovery,
    }));

    let mut proves = decision(SpikeVerdict::KeepCurrent, complete(false));
    proves.claim_verb = ClaimVerb::Proves;
    assert_eq!(validate_spike_decision(&proves), vec![SpikeDecisionIssue::Overclaim]);

    let contradicted = validate_spike_decision(&decision(SpikeVerdict::Replace, complete(false)));
    assert!(contradicted.contains(&SpikeDecisionIssue::ReplaceContradicted {
        workload_ref: WORKLOAD.to_owned(),
        metric: SpikeMetric::PointRead,
    }));
}

#[test]
fn units_duplicates_and_non_claims_are_checked() {
    let mut measurements = complete(false);
    measurements[0].unit = MeasurementUnit::Permille;
    let duplicate = measurements[1].clone();
    measurements.push(duplicate);
    let mut record = decision(SpikeVerdict::KeepCurrent, measurements);
    record.non_claims.pop();
    let issues = validate_spike_decision(&record);
    assert!(issues.contains(&SpikeDecisionIssue::WrongUnit(SpikeMetric::BatchWrite)));
    assert!(issues.iter().any(|issue| matches!(issue, SpikeDecisionIssue::DuplicateMeasurement { .. })));
    assert!(issues.iter().any(|issue| matches!(issue, SpikeDecisionIssue::MissingNonClaim(_))));
}
