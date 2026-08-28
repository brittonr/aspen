use std::path::PathBuf;

use molten::world_state_oracle::*;
use molten_core::world_state_oracle::*;

const BINARY_ENV: &str = "MOLTEN_DOLTLITE_ORACLE_BIN";
const WORKSPACE_ENV: &str = "MOLTEN_DOLTLITE_ORACLE_WORKSPACE";
const BUILD_REF_ENV: &str = "MOLTEN_DOLTLITE_BUILD_REF";
const ADAPTER_REF_ENV: &str = "MOLTEN_DOLTLITE_ADAPTER_REF";

#[test]
#[ignore = "requires the Nix-built remotes-disabled DoltLite cohort"]
fn live_doltlite_oracle_covers_normalized_positive_and_negative_cases() {
    let executable = PathBuf::from(std::env::var(BINARY_ENV).expect("DoltLite binary"));
    let workspace = PathBuf::from(std::env::var(WORKSPACE_ENV).expect("oracle workspace"));
    let build_ref = std::env::var(BUILD_REF_ENV).expect("build ref");
    let adapter_ref = std::env::var(ADAPTER_REF_ENV).expect("adapter ref");
    let source = standard_source_descriptor(build_ref);
    let mut oracle = DoltLiteProcessOracle::new(executable, workspace, source, adapter_ref).expect("oracle");

    let forward = oracle
        .execute_case(&request(OracleCaseKind::HistoryIndependentState, "history-forward", vec![0, 1]))
        .expect("forward history");
    let reverse = oracle
        .execute_case(&request(OracleCaseKind::HistoryIndependentState, "history-reverse", vec![1, 0]))
        .expect("reverse history");
    assert_eq!(forward.rows, reverse.rows);
    assert_eq!(forward.backend_root, reverse.backend_root);
    assert_eq!(
        compare_oracle_observations(&forward, &reverse).expect("comparison").decision,
        ComparisonDecision::Agreement
    );

    for (case, id, expected) in [
        (OracleCaseKind::BranchIsolation, "branch", OracleOutcome::Applied),
        (OracleCaseKind::ReaderSafeGarbageCollection, "gc", OracleOutcome::Applied),
        (OracleCaseKind::ExactFormatReopen, "reopen", OracleOutcome::Applied),
        (OracleCaseKind::DetachedRead, "detached", OracleOutcome::ReadOnly),
        (OracleCaseKind::RemoteDisabled, "remote", OracleOutcome::Rejected),
        (OracleCaseKind::RowIdRejected, "rowid", OracleOutcome::Rejected),
        (OracleCaseKind::CustomCollationRejected, "collation", OracleOutcome::Rejected),
        (OracleCaseKind::MultiFileWriteUnsupported, "multi-file", OracleOutcome::Rejected),
    ] {
        let observation = oracle
            .execute_case(&request(case, id, vec![0, 1]))
            .unwrap_or_else(|error| panic!("{}: {error:?}", case.as_str()));
        assert_eq!(observation.outcome, expected, "{}", case.as_str());
        assert!(!observation.backend_root_is_global_identity);
        assert!(validate_observation(&observation, OracleBounds::standard(), true).is_empty());
    }

    for case in [
        OracleCaseKind::CompareAndAdvance,
        OracleCaseKind::SerializationRoundTrip,
        OracleCaseKind::StaleWriterDenied,
        OracleCaseKind::CompetingWriterClassified,
        OracleCaseKind::TamperedStorage,
        OracleCaseKind::WrongFormatRejected,
        OracleCaseKind::MalformedSerialization,
        OracleCaseKind::IdentityOverclaimRejected,
    ] {
        let observation = oracle
            .execute_case(&request(case, case.as_str(), vec![0, 1]))
            .unwrap_or_else(|error| panic!("{}: {error:?}", case.as_str()));
        assert_eq!(observation.outcome, OracleOutcome::Unsupported, "{}", case.as_str());
        assert_eq!(observation.diagnostics, ["case-requires-reviewed-external-evidence"]);
    }
}

fn request(case: OracleCaseKind, database_id: &str, mutation_order: Vec<usize>) -> OracleCaseRequest {
    OracleCaseRequest {
        case,
        database_id: database_id.to_string(),
        branch: Some("main".to_string()),
        rows: vec![
            SemanticStateRow {
                key: "alpha".to_string(),
                value: "one".to_string(),
            },
            SemanticStateRow {
                key: "beta".to_string(),
                value: "two".to_string(),
            },
        ],
        mutation_order,
    }
}
