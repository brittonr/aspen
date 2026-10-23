use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;

use super::*;

const SELECTION: &str = include_str!("../../../config/marble-store/generated/selection.json");

fn edited(edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut document = serde_json::from_str::<serde_json::Value>(SELECTION).expect("selection JSON");
    edit(&mut document);
    document.to_string()
}

// r[verify aspen.marble_store.spike]
#[test]
fn the_reviewed_selection_keeps_redb_as_the_default_path() {
    let selection = parse_prolly_store_selection(SELECTION).expect("reviewed selection");
    assert_eq!(selection.backend, ProllyStoreBackend::Redb);
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let roots = ProllyStoreRoots {
        storage: &storage,
        marble_heap: None,
    };
    let mut store = open_prolly_store(&roots, &selection).expect("selected store");
    assert!(matches!(store, SelectedProllyStore::Redb(_)));
    let expected = ExpectedProllyRoot {
        root_ref: None,
        generation: INITIAL_GENERATION,
    };
    publish_prolly_edit(&mut store, MAP_ID, &expected, &initial_plan()).expect("publication through the seam");
    assert_eq!(store.read_root(MAP_ID).expect("root").map(|root| root.generation), Some(FIRST_GENERATION));
}

#[test]
fn malformed_or_unbounded_selections_are_refused() {
    let cases = [
        edited(|document| document["backend"] = "sled".into()),
        edited(|document| document["schema"] = "molten.prolly-store-selection.v0".into()),
        edited(|document| document["non_claims"] = serde_json::json!([])),
        edited(|document| document["marble"]["executor_queue_depth"] = 0.into()),
        edited(|document| document["marble"]["block_ids"]["start"] = 0.into()),
        edited(|document| document["unexpected"] = true.into()),
    ];
    for case in cases {
        assert_eq!(parse_prolly_store_selection(&case).expect_err("refused").code, "prolly-store-selection");
    }
}

// r[verify aspen.marble_store.spike]
#[test]
fn selecting_marble_needs_the_feature_and_a_heap_directory() {
    let selection =
        parse_prolly_store_selection(&edited(|document| document["backend"] = "marble-art".into())).expect("selection");
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let without_heap = ProllyStoreRoots {
        storage: &storage,
        marble_heap: None,
    };
    let refused = open_prolly_store(&without_heap, &selection).err().expect("refused");
    if cfg!(feature = "marble-object-store") {
        assert_eq!(refused.code, "prolly-store-selection");
    } else {
        assert_eq!(refused.code, "prolly-store-backend-unavailable");
    }
}

#[cfg(feature = "marble-object-store")]
#[test]
fn a_marble_selection_opens_the_spike_backend() {
    let selection =
        parse_prolly_store_selection(&edited(|document| document["backend"] = "marble-art".into())).expect("selection");
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let heap = crate::test_support::process_workspace("marble_selection").expect("heap");
    let roots = ProllyStoreRoots {
        storage: &storage,
        marble_heap: Some(&heap),
    };
    let store = open_prolly_store(&roots, &selection).expect("marble store");
    assert!(matches!(store, SelectedProllyStore::MarbleArt(_)));
}

const DECISION: &str = include_str!("../../../config/marble-store/generated/decision.json");

fn decision_record() -> molten_core::marble_store::SpikeDecision {
    use molten_core::marble_store::*;
    let document = serde_json::from_str::<serde_json::Value>(DECISION).expect("decision JSON");
    let text = |value: &serde_json::Value| value.as_str().expect("string").to_owned();
    let measurements = document["measurements"]
        .as_array()
        .expect("measurements")
        .iter()
        .map(|item| SpikeMeasurement {
            workload_ref: text(&item["workload_ref"]),
            backend: match item["backend"].as_str() {
                Some("redb-current") => StoreBackend::RedbCurrent,
                Some("marble-art") => StoreBackend::MarbleArt,
                other => panic!("backend {other:?}"),
            },
            metric: match item["metric"].as_str() {
                Some("batch-write") => SpikeMetric::BatchWrite,
                Some("point-read") => SpikeMetric::PointRead,
                Some("recovery") => SpikeMetric::Recovery,
                Some("space-amplification") => SpikeMetric::SpaceAmplification,
                other => panic!("metric {other:?}"),
            },
            unit: match item["unit"].as_str() {
                Some("nanoseconds") => MeasurementUnit::Nanoseconds,
                Some("permille") => MeasurementUnit::Permille,
                other => panic!("unit {other:?}"),
            },
            value: item["value"].as_u64().expect("value"),
        })
        .collect();
    SpikeDecision {
        verdict: match document["verdict"].as_str() {
            Some("keep-current") => SpikeVerdict::KeepCurrent,
            Some("replace") => SpikeVerdict::Replace,
            Some("inconclusive") => SpikeVerdict::Inconclusive,
            other => panic!("verdict {other:?}"),
        },
        claim_verb: match document["claim_verb"].as_str() {
            Some("records") => ClaimVerb::Records,
            other => panic!("claim verb {other:?}"),
        },
        measurements,
        non_claims: document["non_claims"].as_array().expect("non-claims").iter().map(text).collect(),
    }
}

// r[verify aspen.marble_store.boundary]
// r[verify aspen.marble_store.spike]
#[test]
fn the_recorded_keep_decision_passes_the_core_claim_boundary_and_a_replace_reading_does_not() {
    use molten_core::marble_store::*;
    let decision = decision_record();
    assert_eq!(decision.verdict, SpikeVerdict::KeepCurrent);
    assert!(validate_spike_decision(&decision).is_empty());
    let mut replace = decision;
    replace.verdict = SpikeVerdict::Replace;
    assert!(
        validate_spike_decision(&replace)
            .iter()
            .any(|issue| matches!(issue, SpikeDecisionIssue::ReplaceContradicted { .. })),
        "the recorded numbers do not support replacing the current path"
    );
}
