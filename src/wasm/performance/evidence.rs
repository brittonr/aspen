use super::comparison::benchmark_comparison_ref;
use super::comparison::compare_benchmark_runs;
use super::comparison::validate_benchmark_run;
use super::model::BenchmarkComparison;
use super::model::BenchmarkRun;
use super::model::ComparisonDecision;
use super::model::PerformanceDenial;
use super::model::PerformanceEvidenceRole;
use super::model::PerformanceResult;
use super::model::sorted_unique;
use super::model::valid_content_ref;
use super::model::valid_ref_collection;
use super::profile::PERFORMANCE_NON_CLAIMS;
use super::profile::performance_profile_ref;
use super::profile::supported_performance_profile;

pub const PERFORMANCE_RECEIPT_SCHEMA: &str = "molten.wasm-component-performance-receipt.v1";
const MAX_PERFORMANCE_RECEIPT_REFS: usize = 128;
const MAX_RECEIPT_VALIDATION_BLOCKERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceReceiptInput {
    pub run: BenchmarkRun,
    pub comparison_peer_run: Option<BenchmarkRun>,
    pub comparison: Option<BenchmarkComparison>,
    pub optimization_profile_ref: String,
    pub mantle_evidence_refs: Vec<String>,
    pub valence_evidence_refs: Vec<String>,
    pub conformance_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceReceipt {
    pub input: PerformanceReceiptInput,
    pub evidence_role: PerformanceEvidenceRole,
    pub non_claims: Vec<String>,
    pub receipt_ref: String,
}

pub fn build_performance_receipt(mut input: PerformanceReceiptInput) -> PerformanceResult<PerformanceReceipt> {
    normalize_input(&mut input);
    validate_receipt_input(&input)?;
    let mut receipt = PerformanceReceipt {
        input,
        evidence_role: PerformanceEvidenceRole::RecordedOnly,
        non_claims: PERFORMANCE_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect(),
        receipt_ref: String::new(),
    };
    receipt.receipt_ref = crate::preserves_rail::canonical_hash(&performance_receipt_value(&receipt))
        .map_err(|error| PerformanceDenial::new(format!("performance receipt hashing failed: {error}")))?;
    Ok(receipt)
}

pub fn validate_performance_receipt(receipt: &PerformanceReceipt) -> PerformanceResult<()> {
    validate_receipt_input(&receipt.input)?;
    if receipt.evidence_role != PerformanceEvidenceRole::RecordedOnly {
        return Err(PerformanceDenial::new("performance receipt evidence role is not recorded-only"));
    }
    let expected_non_claims = PERFORMANCE_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        return Err(PerformanceDenial::new("performance receipt changes required non-claims"));
    }
    let expected = crate::preserves_rail::canonical_hash(&performance_receipt_value(receipt))
        .map_err(|error| PerformanceDenial::new(format!("performance receipt hashing failed: {error}")))?;
    if receipt.receipt_ref != expected {
        return Err(PerformanceDenial::new("performance receipt identity is stale or tampered"));
    }
    Ok(())
}

pub fn validate_performance_receipt_against(
    receipt: &PerformanceReceipt,
    expected_input: &PerformanceReceiptInput,
) -> PerformanceResult<()> {
    validate_performance_receipt(receipt)?;
    let expected = build_performance_receipt(expected_input.clone())?;
    if receipt != &expected {
        return Err(PerformanceDenial::new(
            "performance receipt differs from the independently derived run or comparison",
        ));
    }
    Ok(())
}

pub fn performance_receipt_value(receipt: &PerformanceReceipt) -> preserves::IOValue {
    use crate::preserves_rail::record;
    use crate::preserves_rail::sequence;
    use crate::preserves_rail::string;
    use crate::preserves_rail::u64_value;

    let run = &receipt.input.run;
    let phases = run
        .phases
        .iter()
        .map(|phase| {
            let samples = phase
                .samples
                .iter()
                .map(|sample| {
                    record("sample", vec![
                        u64_value(u64::from(sample.process)),
                        u64_value(u64::from(sample.iteration)),
                        u64_value(sample.count),
                    ])
                })
                .collect();
            record("phase", vec![string(phase.phase.as_str()), string(&phase.event), sequence(samples)])
        })
        .collect();
    record("wasm-component-performance-receipt-v1", vec![
        record("schema", vec![string(PERFORMANCE_RECEIPT_SCHEMA)]),
        record("evidence-role", vec![string(receipt.evidence_role.as_str())]),
        record("suite-ref", vec![string(&run.suite_ref)]),
        record("run-ref", vec![string(&run.run_ref)]),
        record("benchmark-ref", vec![string(&run.benchmark_ref)]),
        record("consumer", vec![string(run.consumer.as_str())]),
        record("source-component-ref", vec![string(&run.source_component_ref)]),
        record("component-ref", vec![string(&run.component_ref)]),
        record("component-profile-ref", vec![string(&run.component_profile_ref)]),
        record("performance-profile-ref", vec![string(&run.performance_profile_ref)]),
        record("engine-cohort-ref", vec![string(&run.engine_cohort_ref)]),
        record("engine-artifact-ref", vec![string(&run.engine_artifact_ref)]),
        record("runner-artifact-ref", vec![string(&run.runner_artifact_ref)]),
        record("runtime-configuration-ref", vec![string(&run.runtime_configuration_ref)]),
        record("target", vec![string(&run.target)]),
        record("host-class-ref", vec![string(&run.host_class_ref)]),
        record("measurement", vec![string(&run.measurement)]),
        record("resource-envelope-ref", vec![string(&run.resource_envelope_ref)]),
        record("recorded-effect-refs", vec![strings(&run.recorded_effect_refs)]),
        record("phases", vec![sequence(phases)]),
        record("comparison-peer-run", vec![optional_run(receipt.input.comparison_peer_run.as_ref())]),
        record("comparison-peer-run-ref", vec![optional_ref(
            receipt.input.comparison_peer_run.as_ref().map(|run| run.run_ref.as_str()),
        )]),
        record("comparison-ref", vec![optional_ref(
            receipt.input.comparison.as_ref().map(|comparison| comparison.comparison_ref.as_str()),
        )]),
        record("optimization-profile-ref", vec![string(&receipt.input.optimization_profile_ref)]),
        record("mantle-evidence-refs", vec![strings(&receipt.input.mantle_evidence_refs)]),
        record("valence-evidence-refs", vec![strings(&receipt.input.valence_evidence_refs)]),
        record("conformance-receipt-refs", vec![strings(&receipt.input.conformance_receipt_refs)]),
        record("non-claims", vec![strings(&receipt.non_claims)]),
    ])
}

pub fn performance_receipt_summary(receipt: &PerformanceReceipt) -> String {
    let comparison = receipt.input.comparison.as_ref().map_or("recorded-run", |_| "recorded-comparison");
    format!(
        "Wasm component performance {comparison} suite={} run={} receipt={} role=recorded-only (non-normative)",
        receipt.input.run.suite_ref, receipt.input.run.run_ref, receipt.receipt_ref
    )
}

fn validate_receipt_input(input: &PerformanceReceiptInput) -> PerformanceResult<()> {
    validate_benchmark_run(&input.run)?;
    if let Some(peer) = &input.comparison_peer_run {
        validate_benchmark_run(peer)?;
    }
    let profile = supported_performance_profile()?;
    let expected_profile_ref = performance_profile_ref(&profile);
    let mut blockers = Vec::with_capacity(MAX_RECEIPT_VALIDATION_BLOCKERS);
    if input.run.performance_profile_ref != expected_profile_ref
        || input
            .comparison_peer_run
            .as_ref()
            .is_some_and(|peer| peer.performance_profile_ref != expected_profile_ref)
    {
        blockers.push("performance receipt run uses an unsupported performance profile".to_string());
    }
    if !valid_content_ref(&input.optimization_profile_ref) {
        blockers.push("performance receipt optimization profile ref is malformed".to_string());
    }
    for (label, refs) in [
        ("Mantle", &input.mantle_evidence_refs),
        ("Valence", &input.valence_evidence_refs),
        ("conformance", &input.conformance_receipt_refs),
    ] {
        if refs.len() > MAX_PERFORMANCE_RECEIPT_REFS || !valid_ref_collection(refs) {
            blockers.push(format!("performance receipt {label} refs are missing, malformed, duplicate, or over bound"));
        }
    }
    match (&input.comparison, &input.comparison_peer_run) {
        (Some(comparison), Some(peer)) => {
            if comparison.comparison_ref != benchmark_comparison_ref(comparison) {
                blockers.push("performance comparison identity does not match its canonical fields".to_string());
            }
            let runs = if comparison.baseline_run_ref == input.run.run_ref
                && comparison.candidate_run_ref == peer.run_ref
            {
                Some((&input.run, peer))
            } else if comparison.baseline_run_ref == peer.run_ref && comparison.candidate_run_ref == input.run.run_ref {
                Some((peer, &input.run))
            } else {
                None
            };
            match runs {
                Some((baseline, candidate)) => match compare_benchmark_runs(&profile, baseline, candidate)? {
                    ComparisonDecision::Comparable(expected) if &expected == comparison => {}
                    ComparisonDecision::Comparable(_) => blockers
                        .push("performance comparison differs from independently recomputed samples".to_string()),
                    ComparisonDecision::Incompatible { .. } => {
                        blockers.push("performance comparison links runs that are not comparable".to_string())
                    }
                },
                None => blockers
                    .push("performance comparison does not bind the recorded run and its exact peer run".to_string()),
            }
        }
        (Some(_), None) => blockers.push("performance comparison omits its exact peer run".to_string()),
        (None, Some(_)) => blockers.push("performance receipt includes a peer run without a comparison".to_string()),
        (None, None) => {}
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

fn normalize_input(input: &mut PerformanceReceiptInput) {
    input.mantle_evidence_refs = sorted_unique(&input.mantle_evidence_refs);
    input.valence_evidence_refs = sorted_unique(&input.valence_evidence_refs);
    input.conformance_receipt_refs = sorted_unique(&input.conformance_receipt_refs);
}

fn benchmark_run_value(run: &BenchmarkRun) -> preserves::IOValue {
    use crate::preserves_rail::record;
    use crate::preserves_rail::sequence;
    use crate::preserves_rail::string;
    use crate::preserves_rail::u64_value;

    let phases = run
        .phases
        .iter()
        .map(|phase| {
            let samples = phase
                .samples
                .iter()
                .map(|sample| {
                    record("sample", vec![
                        u64_value(u64::from(sample.process)),
                        u64_value(u64::from(sample.iteration)),
                        u64_value(sample.count),
                    ])
                })
                .collect();
            record("phase", vec![string(phase.phase.as_str()), string(&phase.event), sequence(samples)])
        })
        .collect();
    record("benchmark-run-v1", vec![
        record("suite-ref", vec![string(&run.suite_ref)]),
        record("run-ref", vec![string(&run.run_ref)]),
        record("benchmark-ref", vec![string(&run.benchmark_ref)]),
        record("consumer", vec![string(run.consumer.as_str())]),
        record("source-component-ref", vec![string(&run.source_component_ref)]),
        record("component-ref", vec![string(&run.component_ref)]),
        record("component-profile-ref", vec![string(&run.component_profile_ref)]),
        record("performance-profile-ref", vec![string(&run.performance_profile_ref)]),
        record("engine-cohort-ref", vec![string(&run.engine_cohort_ref)]),
        record("engine-artifact-ref", vec![string(&run.engine_artifact_ref)]),
        record("runner-artifact-ref", vec![string(&run.runner_artifact_ref)]),
        record("runtime-configuration-ref", vec![string(&run.runtime_configuration_ref)]),
        record("target", vec![string(&run.target)]),
        record("host-class-ref", vec![string(&run.host_class_ref)]),
        record("measurement", vec![string(&run.measurement)]),
        record("resource-envelope-ref", vec![string(&run.resource_envelope_ref)]),
        record("recorded-effect-refs", vec![strings(&run.recorded_effect_refs)]),
        record("phases", vec![sequence(phases)]),
    ])
}

fn optional_run(run: Option<&BenchmarkRun>) -> preserves::IOValue {
    run.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |run| crate::preserves_rail::record("some", vec![benchmark_run_value(run)]),
    )
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn optional_ref(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}
