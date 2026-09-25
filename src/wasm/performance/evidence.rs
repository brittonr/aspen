pub const PERFORMANCE_RECEIPT_SCHEMA: &str = "molten.wasm-component-performance-receipt.v1";
const MAX_PERFORMANCE_RECEIPT_REFS: usize = 128;
const MAX_RECEIPT_VALIDATION_BLOCKERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceReceiptInput {
    pub run: super::model::BenchmarkRun,
    pub comparison_peer_run: Option<super::model::BenchmarkRun>,
    pub comparison: Option<super::model::BenchmarkComparison>,
    pub optimization_profile_ref: String,
    pub mantle_evidence_refs: Vec<String>,
    pub valence_evidence_refs: Vec<String>,
    pub conformance_receipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceReceipt {
    pub input: PerformanceReceiptInput,
    pub evidence_role: super::model::PerformanceEvidenceRole,
    pub non_claims: Vec<String>,
    pub receipt_ref: String,
}

pub fn build_performance_receipt(
    mut input: PerformanceReceiptInput,
) -> super::model::PerformanceResult<PerformanceReceipt> {
    normalize_input(&mut input);
    validate_receipt_input(&input)?;
    let mut receipt = PerformanceReceipt {
        input,
        evidence_role: super::model::PerformanceEvidenceRole::RecordedOnly,
        non_claims: super::profile::PERFORMANCE_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect(),
        receipt_ref: String::new(),
    };
    receipt.receipt_ref =
        crate::preserves_rail::canonical_hash(&performance_receipt_value(&receipt)).map_err(|error| {
            super::model::PerformanceDenial::new(format!("performance receipt hashing failed: {error}"))
        })?;
    Ok(receipt)
}

pub fn validate_performance_receipt(receipt: &PerformanceReceipt) -> super::model::PerformanceResult<()> {
    validate_receipt_input(&receipt.input)?;
    if receipt.evidence_role != super::model::PerformanceEvidenceRole::RecordedOnly {
        return Err(super::model::PerformanceDenial::new("performance receipt evidence role is not recorded-only"));
    }
    let expected_non_claims =
        super::profile::PERFORMANCE_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        return Err(super::model::PerformanceDenial::new("performance receipt changes required non-claims"));
    }
    let expected = crate::preserves_rail::canonical_hash(&performance_receipt_value(receipt)).map_err(|error| {
        super::model::PerformanceDenial::new(format!("performance receipt hashing failed: {error}"))
    })?;
    if receipt.receipt_ref != expected {
        return Err(super::model::PerformanceDenial::new("performance receipt identity is stale or tampered"));
    }
    Ok(())
}

pub fn validate_performance_receipt_against(
    receipt: &PerformanceReceipt,
    expected_input: &PerformanceReceiptInput,
) -> super::model::PerformanceResult<()> {
    validate_performance_receipt(receipt)?;
    let expected = build_performance_receipt(expected_input.clone())?;
    if receipt != &expected {
        return Err(super::model::PerformanceDenial::new(
            "performance receipt differs from the independently derived run or comparison",
        ));
    }
    Ok(())
}

pub fn performance_receipt_value(receipt: &PerformanceReceipt) -> preserves::IOValue {
    let run = &receipt.input.run;
    let phases = phase_values(run);
    crate::preserves_rail::record("wasm-component-performance-receipt-v1", vec![
        crate::preserves_rail::record("schema", vec![crate::preserves_rail::string(PERFORMANCE_RECEIPT_SCHEMA)]),
        crate::preserves_rail::record("evidence-role", vec![crate::preserves_rail::string(
            receipt.evidence_role.as_str(),
        )]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&run.suite_ref)]),
        crate::preserves_rail::record("run-ref", vec![crate::preserves_rail::string(&run.run_ref)]),
        crate::preserves_rail::record("benchmark-ref", vec![crate::preserves_rail::string(&run.benchmark_ref)]),
        crate::preserves_rail::record("consumer", vec![crate::preserves_rail::string(run.consumer.as_str())]),
        crate::preserves_rail::record("source-component-ref", vec![crate::preserves_rail::string(
            &run.source_component_ref,
        )]),
        crate::preserves_rail::record("component-ref", vec![crate::preserves_rail::string(&run.component_ref)]),
        crate::preserves_rail::record("component-profile-ref", vec![crate::preserves_rail::string(
            &run.component_profile_ref,
        )]),
        crate::preserves_rail::record("performance-profile-ref", vec![crate::preserves_rail::string(
            &run.performance_profile_ref,
        )]),
        crate::preserves_rail::record("engine-cohort-ref", vec![crate::preserves_rail::string(&run.engine_cohort_ref)]),
        crate::preserves_rail::record("engine-artifact-ref", vec![crate::preserves_rail::string(
            &run.engine_artifact_ref,
        )]),
        crate::preserves_rail::record("runner-artifact-ref", vec![crate::preserves_rail::string(
            &run.runner_artifact_ref,
        )]),
        crate::preserves_rail::record("runtime-configuration-ref", vec![crate::preserves_rail::string(
            &run.runtime_configuration_ref,
        )]),
        crate::preserves_rail::record("target", vec![crate::preserves_rail::string(&run.target)]),
        crate::preserves_rail::record("host-class-ref", vec![crate::preserves_rail::string(&run.host_class_ref)]),
        crate::preserves_rail::record("measurement", vec![crate::preserves_rail::string(&run.measurement)]),
        crate::preserves_rail::record("resource-envelope-ref", vec![crate::preserves_rail::string(
            &run.resource_envelope_ref,
        )]),
        crate::preserves_rail::record("recorded-effect-refs", vec![strings(&run.recorded_effect_refs)]),
        crate::preserves_rail::record("phases", vec![crate::preserves_rail::sequence(phases)]),
        crate::preserves_rail::record("comparison-peer-run", vec![optional_run(
            receipt.input.comparison_peer_run.as_ref(),
        )]),
        crate::preserves_rail::record("comparison-peer-run-ref", vec![optional_ref(
            receipt.input.comparison_peer_run.as_ref().map(|run| run.run_ref.as_str()),
        )]),
        crate::preserves_rail::record("comparison-ref", vec![optional_ref(
            receipt.input.comparison.as_ref().map(|comparison| comparison.comparison_ref.as_str()),
        )]),
        crate::preserves_rail::record("optimization-profile-ref", vec![crate::preserves_rail::string(
            &receipt.input.optimization_profile_ref,
        )]),
        crate::preserves_rail::record("mantle-evidence-refs", vec![strings(&receipt.input.mantle_evidence_refs)]),
        crate::preserves_rail::record("valence-evidence-refs", vec![strings(&receipt.input.valence_evidence_refs)]),
        crate::preserves_rail::record("conformance-receipt-refs", vec![strings(
            &receipt.input.conformance_receipt_refs,
        )]),
        crate::preserves_rail::record("non-claims", vec![strings(&receipt.non_claims)]),
    ])
}

fn phase_values(run: &super::model::BenchmarkRun) -> Vec<preserves::IOValue> {
    run.phases
        .iter()
        .map(|phase| {
            let samples = phase
                .samples
                .iter()
                .map(|sample| {
                    crate::preserves_rail::record("sample", vec![
                        crate::preserves_rail::u64_value(u64::from(sample.process)),
                        crate::preserves_rail::u64_value(u64::from(sample.iteration)),
                        crate::preserves_rail::u64_value(sample.count),
                    ])
                })
                .collect();
            crate::preserves_rail::record("phase", vec![
                crate::preserves_rail::string(phase.phase.as_str()),
                crate::preserves_rail::string(&phase.event),
                crate::preserves_rail::sequence(samples),
            ])
        })
        .collect()
}

pub fn performance_receipt_summary(receipt: &PerformanceReceipt) -> String {
    let comparison = receipt.input.comparison.as_ref().map_or("recorded-run", |_| "recorded-comparison");
    format!(
        "Wasm component performance {comparison} suite={} run={} receipt={} role=recorded-only (non-normative)",
        receipt.input.run.suite_ref, receipt.input.run.run_ref, receipt.receipt_ref
    )
}

fn validate_receipt_input(input: &PerformanceReceiptInput) -> super::model::PerformanceResult<()> {
    super::comparison::validate_benchmark_run(&input.run)?;
    if let Some(peer) = &input.comparison_peer_run {
        super::comparison::validate_benchmark_run(peer)?;
    }
    let profile = super::profile::supported_performance_profile()?;
    let expected_profile_ref = super::profile::performance_profile_ref(&profile);
    let mut blockers = Vec::with_capacity(MAX_RECEIPT_VALIDATION_BLOCKERS);
    if input.run.performance_profile_ref != expected_profile_ref
        || input
            .comparison_peer_run
            .as_ref()
            .is_some_and(|peer| peer.performance_profile_ref != expected_profile_ref)
    {
        blockers.push("performance receipt run uses an unsupported performance profile".to_string());
    }
    if !super::model::valid_content_ref(&input.optimization_profile_ref) {
        blockers.push("performance receipt optimization profile ref is malformed".to_string());
    }
    for (label, refs) in [
        ("Mantle", &input.mantle_evidence_refs),
        ("Valence", &input.valence_evidence_refs),
        ("conformance", &input.conformance_receipt_refs),
    ] {
        if refs.len() > MAX_PERFORMANCE_RECEIPT_REFS || !super::model::valid_ref_collection(refs) {
            blockers.push(format!("performance receipt {label} refs are missing, malformed, duplicate, or over bound"));
        }
    }
    match (&input.comparison, &input.comparison_peer_run) {
        (Some(comparison), Some(peer)) => {
            if comparison.comparison_ref != super::comparison::benchmark_comparison_ref(comparison) {
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
                Some((baseline, candidate)) => {
                    match super::comparison::compare_benchmark_runs(&profile, baseline, candidate)? {
                        super::model::ComparisonDecision::Comparable(expected) if &expected == comparison => {}
                        super::model::ComparisonDecision::Comparable(_) => blockers
                            .push("performance comparison differs from independently recomputed samples".to_string()),
                        super::model::ComparisonDecision::Incompatible { .. } => {
                            blockers.push("performance comparison links runs that are not comparable".to_string())
                        }
                    }
                }
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
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

fn normalize_input(input: &mut PerformanceReceiptInput) {
    input.mantle_evidence_refs = super::model::sorted_unique(&input.mantle_evidence_refs);
    input.valence_evidence_refs = super::model::sorted_unique(&input.valence_evidence_refs);
    input.conformance_receipt_refs = super::model::sorted_unique(&input.conformance_receipt_refs);
}

fn benchmark_run_value(run: &super::model::BenchmarkRun) -> preserves::IOValue {
    let phases = run
        .phases
        .iter()
        .map(|phase| {
            let samples = phase
                .samples
                .iter()
                .map(|sample| {
                    crate::preserves_rail::record("sample", vec![
                        crate::preserves_rail::u64_value(u64::from(sample.process)),
                        crate::preserves_rail::u64_value(u64::from(sample.iteration)),
                        crate::preserves_rail::u64_value(sample.count),
                    ])
                })
                .collect();
            crate::preserves_rail::record("phase", vec![
                crate::preserves_rail::string(phase.phase.as_str()),
                crate::preserves_rail::string(&phase.event),
                crate::preserves_rail::sequence(samples),
            ])
        })
        .collect();
    crate::preserves_rail::record("benchmark-run-v1", vec![
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&run.suite_ref)]),
        crate::preserves_rail::record("run-ref", vec![crate::preserves_rail::string(&run.run_ref)]),
        crate::preserves_rail::record("benchmark-ref", vec![crate::preserves_rail::string(&run.benchmark_ref)]),
        crate::preserves_rail::record("consumer", vec![crate::preserves_rail::string(run.consumer.as_str())]),
        crate::preserves_rail::record("source-component-ref", vec![crate::preserves_rail::string(
            &run.source_component_ref,
        )]),
        crate::preserves_rail::record("component-ref", vec![crate::preserves_rail::string(&run.component_ref)]),
        crate::preserves_rail::record("component-profile-ref", vec![crate::preserves_rail::string(
            &run.component_profile_ref,
        )]),
        crate::preserves_rail::record("performance-profile-ref", vec![crate::preserves_rail::string(
            &run.performance_profile_ref,
        )]),
        crate::preserves_rail::record("engine-cohort-ref", vec![crate::preserves_rail::string(&run.engine_cohort_ref)]),
        crate::preserves_rail::record("engine-artifact-ref", vec![crate::preserves_rail::string(
            &run.engine_artifact_ref,
        )]),
        crate::preserves_rail::record("runner-artifact-ref", vec![crate::preserves_rail::string(
            &run.runner_artifact_ref,
        )]),
        crate::preserves_rail::record("runtime-configuration-ref", vec![crate::preserves_rail::string(
            &run.runtime_configuration_ref,
        )]),
        crate::preserves_rail::record("target", vec![crate::preserves_rail::string(&run.target)]),
        crate::preserves_rail::record("host-class-ref", vec![crate::preserves_rail::string(&run.host_class_ref)]),
        crate::preserves_rail::record("measurement", vec![crate::preserves_rail::string(&run.measurement)]),
        crate::preserves_rail::record("resource-envelope-ref", vec![crate::preserves_rail::string(
            &run.resource_envelope_ref,
        )]),
        crate::preserves_rail::record("recorded-effect-refs", vec![strings(&run.recorded_effect_refs)]),
        crate::preserves_rail::record("phases", vec![crate::preserves_rail::sequence(phases)]),
    ])
}

fn optional_run(run: Option<&super::model::BenchmarkRun>) -> preserves::IOValue {
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
