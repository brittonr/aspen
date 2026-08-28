use super::*;
use crate::world_benchmark::*;
use crate::world_state_oracle::*;

const SOURCE_REVISION: &str = "1111111111111111111111111111111111111111";

// r[verify molten.prolly_map.differential]
#[test]
fn normalized_oracle_agreement_never_requires_cross_format_roots() {
    let profile = profile();
    let build = build_map(&profile, &text_entries()).expect("text map");
    let oracle = oracle_observation(vec![
        SemanticStateRow {
            key: "alpha".to_string(),
            value: "one".to_string(),
        },
        SemanticStateRow {
            key: "beta".to_string(),
            value: "two".to_string(),
        },
    ]);
    let agreement = compare_with_doltlite(&profile, &build.snapshot, &oracle).expect("agreement");
    assert_eq!(agreement.decision, DifferentialDecision::Agreement);
    assert!(!agreement.cross_format_root_equality_required);
    assert!(!agreement.proves_correctness);

    let changed = oracle_observation(vec![SemanticStateRow {
        key: "alpha".to_string(),
        value: "changed".to_string(),
    }]);
    let divergence = compare_with_doltlite(&profile, &build.snapshot, &changed).expect("divergence");
    assert_eq!(divergence.decision, DifferentialDecision::Divergence);
    assert!(divergence.first_divergence.is_some());
}

// r[verify molten.prolly_map.extraction]
#[test]
fn one_local_consumer_cannot_trigger_shared_component_extraction() {
    let decision = classify_local_extraction(benchmark_receipt()).expect("extraction decision");
    assert_eq!(decision.disposition, WorldBenchmarkExtractionDisposition::RetainCurrent);
    assert!(!decision.creates_repository);
    assert!(!decision.approves_dependency);
    assert_eq!(decision.credible_consumers, ["molten-prolly-pilot"]);
}

fn oracle_observation(rows: Vec<SemanticStateRow>) -> OracleObservation {
    let source = standard_source_descriptor(reference("doltlite-build"));
    build_oracle_observation(&source, OracleObservationInput {
        adapter_ref: reference("doltlite-adapter"),
        case: OracleCaseKind::HistoryIndependentState,
        branch: Some("main".to_string()),
        rows,
        outcome: OracleOutcome::EqualState,
        backend_root: Some("backend-local-root".to_string()),
        diagnostics: Vec::new(),
    })
    .expect("oracle observation")
}

fn benchmark_receipt() -> WorldBenchmarkReceipt {
    WorldBenchmarkReceipt {
        schema: WORLD_BENCHMARK_RECEIPT_SCHEMA.to_string(),
        receipt_ref: reference("benchmark-receipt"),
        plan_ref: reference("benchmark-plan"),
        consumer_id: "molten-prolly-pilot".to_string(),
        profile_ref: profile().profile_ref.as_str().to_string(),
        source_revision: SOURCE_REVISION.to_string(),
        dataset_ref: reference("dataset"),
        preparation: WorldBenchmarkPreparation::Cold,
        class: WorldBenchmarkClass::Logical,
        adapters: vec!["molten-prolly-map".to_string()],
        hardware_cohort: "logical-no-hardware".to_string(),
        bounds: WorldBenchmarkBounds {
            max_operations: 1,
            max_repetitions: 1,
            max_logical_bytes: 1,
            max_physical_bytes: 1,
            max_objects: 1,
            max_pages: 1,
            max_references: 1,
            max_keys: 1,
            max_conflicts: 1,
            max_duration_nanoseconds: 1,
            max_peak_memory_bytes: 1,
        },
        results: Vec::new(),
        threshold_results: Vec::new(),
        unsupported_rows: Vec::new(),
        accepted: true,
        non_claims: world_benchmark_non_claims(),
    }
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
