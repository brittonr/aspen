use molten_core::world_state_oracle::*;

use super::*;

#[test]
fn process_oracle_rejects_ambient_relative_paths_and_invalid_source() {
    let source = source_descriptor();
    let error = DoltLiteProcessOracle::new(
        "doltlite".into(),
        "workspace".into(),
        source,
        "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
    )
    .expect_err("relative paths must fail");
    assert_eq!(error.code, "oracle-path-not-absolute");

    let error = DoltLiteProcessOracle::new(
        "/bin/doltlite".into(),
        "/tmp/molten-oracle".into(),
        source_descriptor(),
        "blake3:short".to_string(),
    )
    .expect_err("malformed adapter reference must fail");
    assert_eq!(error.code, "oracle-adapter-ref-invalid");
}

fn source_descriptor() -> OracleSourceDescriptor {
    standard_source_descriptor("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
}

#[test]
fn canonical_records_bind_source_observation_comparison_and_consumer() {
    let source = source_descriptor();
    let rows = vec![SemanticStateRow {
        key: "alpha".to_string(),
        value: "one".to_string(),
    }];
    let expected = build_oracle_observation(&source, OracleObservationInput {
        adapter_ref: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        case: OracleCaseKind::HistoryIndependentState,
        branch: Some("main".to_string()),
        rows: rows.clone(),
        outcome: OracleOutcome::EqualState,
        backend_root: Some("backend-root-a".to_string()),
        diagnostics: Vec::new(),
    })
    .expect("expected observation");
    let observed = build_oracle_observation(&source, OracleObservationInput {
        adapter_ref: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        case: OracleCaseKind::HistoryIndependentState,
        branch: Some("main".to_string()),
        rows,
        outcome: OracleOutcome::EqualState,
        backend_root: Some("backend-root-b".to_string()),
        diagnostics: Vec::new(),
    })
    .expect("observed observation");
    let comparison = compare_oracle_observations(&expected, &observed).expect("comparison");
    let projection =
        project_oracle_evidence(OracleConsumer::WorldBenchmark, &observed, &comparison).expect("benchmark projection");

    let source_record = canonical_oracle_source(&source).expect("source record");
    let observation_record = canonical_oracle_observation(&observed).expect("observation record");
    let comparison_record = canonical_oracle_comparison(&comparison).expect("comparison record");
    let projection_record = canonical_oracle_projection(&projection).expect("projection record");
    assert!(source_record.record_ref.starts_with("blake3:"));
    assert!(!observation_record.bytes.is_empty());
    assert_ne!(comparison_record.record_ref, projection_record.record_ref);

    let mut overclaim = projection;
    overclaim.correctness_proven = true;
    assert!(canonical_oracle_projection(&overclaim).is_err());
}

#[test]
fn nickel_ledger_matches_the_rust_compatibility_projection() {
    let value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../config/world-state-oracle/generated/ledger.json"
    ))
    .expect("generated ledger");
    assert_eq!(value.get("source_revision").and_then(serde_json::Value::as_str), Some(DOLTLITE_REVISION));
    let limits = value.get("limits").expect("limits");
    assert_eq!(json_usize(limits, "max_adapted"), STANDARD_COMPATIBILITY_LIMITS.max_adapted);
    assert_eq!(json_usize(limits, "max_intentional"), STANDARD_COMPATIBILITY_LIMITS.max_intentional);
    assert_eq!(json_usize(limits, "max_unsupported"), STANDARD_COMPATIBILITY_LIMITS.max_unsupported);
    assert_eq!(json_usize(limits, "max_engine_gap"), STANDARD_COMPATIBILITY_LIMITS.max_engine_gap);
    let observed = value.get("rows").and_then(serde_json::Value::as_array).expect("rows");
    let expected = standard_compatibility_rows();
    assert_eq!(observed.len(), expected.len());
    for row in &expected {
        let item = observed
            .iter()
            .find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(row.id.as_str()))
            .expect("ledger row");
        assert_eq!(item.get("status").and_then(serde_json::Value::as_str), Some(row.status.as_str()));
        assert_eq!(item.get("evidence_ref").and_then(serde_json::Value::as_str), Some(row.evidence_ref.as_str()));
        assert_eq!(item.get("fixture").and_then(serde_json::Value::as_str), Some(row.fixture.as_str()));
        assert_eq!(item.get("issue").and_then(serde_json::Value::as_str), row.issue.as_deref());
    }
}

#[test]
fn nickel_source_profile_matches_the_rust_source_projection() {
    let value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../config/world-state-oracle/generated/source.json"
    ))
    .expect("generated source profile");
    let source = source_descriptor();
    assert_eq!(value.get("source_revision").and_then(serde_json::Value::as_str), Some(source.revision.as_str()));
    assert_eq!(
        value.get("adapter_version").and_then(serde_json::Value::as_str),
        Some(source.adapter_version.as_str())
    );
    assert_eq!(
        value.get("backend_format").and_then(serde_json::Value::as_str),
        Some(source.backend_format.as_str())
    );
    assert_eq!(
        value.get("imported_scope").and_then(serde_json::Value::as_array).map(Vec::len),
        Some(source.imported_scope.len())
    );
    assert_eq!(
        value.get("build_inputs").and_then(serde_json::Value::as_array).map(Vec::len),
        Some(source.build_inputs.len())
    );
    assert_eq!(value.get("remotes_enabled").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(value.get("vec1_enabled").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(value.get("production_enabled").and_then(serde_json::Value::as_bool), Some(false));
}

fn json_usize(value: &serde_json::Value, field: &str) -> usize {
    usize::try_from(value.get(field).and_then(serde_json::Value::as_u64).expect("numeric field")).expect("usize field")
}
