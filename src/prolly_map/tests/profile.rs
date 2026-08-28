use std::collections::BTreeMap;

use molten_core::prolly_map::*;

use super::*;

#[test]
fn nickel_profile_and_benchmark_match_the_rust_projection() {
    let profile_json =
        serde_json::from_str::<serde_json::Value>(include_str!("../../../config/prolly-map/generated/profile.json"))
            .expect("generated profile");
    let profile = profile();
    assert_eq!(
        profile_json.get("profile_ref").and_then(serde_json::Value::as_str),
        Some(profile.profile_ref.as_str())
    );
    assert_eq!(
        profile_json.get("max_node_bytes").and_then(serde_json::Value::as_u64),
        Some(u64::from(profile.max_node_bytes))
    );
    assert_eq!(profile_json.get("extraction_approved").and_then(serde_json::Value::as_bool), Some(false));

    let benchmark =
        serde_json::from_str::<serde_json::Value>(include_str!("../../../config/prolly-map/generated/benchmark.json"))
            .expect("generated benchmark");
    assert_eq!(
        benchmark.get("adversarial_node_bytes_maximum").and_then(serde_json::Value::as_u64),
        Some(u64::from(profile.max_node_bytes))
    );
    assert_eq!(benchmark.get("extraction_disposition").and_then(serde_json::Value::as_str), Some("retain-current"));
    assert_eq!(benchmark.get("creates_repository").and_then(serde_json::Value::as_bool), Some(false));

    let proof = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../config/prolly-map/generated/proof-obligations.json"
    ))
    .expect("generated proof obligations");
    assert_eq!(
        proof.get("trellis_revision").and_then(serde_json::Value::as_str),
        Some(TRELLIS_PROOF_REFERENCE_REVISION)
    );
    assert_eq!(
        proof.get("obligations").and_then(serde_json::Value::as_array).map(Vec::len),
        Some(standard_proof_obligations().len())
    );
    assert_eq!(proof.get("formal_refinement_complete").and_then(serde_json::Value::as_bool), Some(false));
}

#[test]
fn named_benchmark_thresholds_bind_measured_structural_facts() {
    let profile = profile();
    let prior = build_map(&profile, &entries()).expect("prior map");
    let edit = update_plan(&prior.snapshot);
    let diff = diff_maps(&profile, &prior.snapshot, &edit.next.snapshot).expect("diff");
    let facts = merged_facts(&profile, &[&prior.snapshot, &edit.next.snapshot]);
    let all_nodes = prior
        .snapshot
        .blocks
        .iter()
        .chain(&edit.next.snapshot.blocks)
        .map(|block| block.node_ref.clone())
        .collect::<Vec<_>>();
    let gc = plan_gc(&profile, &all_nodes, core::slice::from_ref(&edit.next.snapshot.root.top_node_ref), &[], &facts)
        .expect("gc plan");
    let measured = benchmark_map(&edit.next, &edit, &diff, &gc, true).expect("measurement");
    let declared =
        serde_json::from_str::<serde_json::Value>(include_str!("../../../config/prolly-map/generated/benchmark.json"))
            .expect("benchmark profile");
    assert_eq!(u64::from(measured.entry_count), json_u64(&declared, "entry_count"));
    assert_eq!(measured.logical_bytes, json_u64(&declared, "observed_logical_bytes"));
    assert_eq!(u64::from(measured.block_count), json_u64(&declared, "observed_block_count"));
    assert_eq!(measured.block_bytes, json_u64(&declared, "observed_block_bytes"));
    assert_eq!(u64::from(measured.reused_blocks), json_u64(&declared, "observed_reused_blocks"));
    assert_eq!(u64::from(measured.diff_records), json_u64(&declared, "observed_diff_records"));
    assert_eq!(u64::from(measured.skipped_equal_nodes), json_u64(&declared, "observed_skipped_equal_nodes"));
    assert_eq!(u64::from(measured.gc_candidates), json_u64(&declared, "observed_gc_candidates"));
    assert_eq!(
        declared.get("observed_restart_verified").and_then(serde_json::Value::as_bool),
        Some(measured.restart_verified)
    );
    assert!(u64::from(measured.reused_blocks) >= json_u64(&declared, "sharing_min_reused_blocks"));
    assert!(u64::from(measured.diff_records) <= json_u64(&declared, "diff_max_records"));
    assert!(measured.block_bytes <= json_u64(&declared, "retained_bytes_maximum"));
    assert!(u64::from(measured.gc_candidates) <= json_u64(&declared, "gc_candidates_maximum"));
    assert!(edit.next.snapshot.blocks.iter().all(|block| {
        u64::try_from(block.bytes.len())
            .is_ok_and(|length| length <= json_u64(&declared, "adversarial_node_bytes_maximum"))
    }));
    assert!(!measured.timing_proves_correctness);
}

fn merged_facts(profile: &ProllyProfile, snapshots: &[&MapSnapshot]) -> Vec<GraphFact> {
    let mut facts = BTreeMap::new();
    for snapshot in snapshots {
        for fact in facts_from_snapshot(profile, snapshot).expect("facts") {
            facts.insert(fact.node_ref.as_str().to_string(), fact);
        }
    }
    facts.into_values().collect()
}

fn json_u64(value: &serde_json::Value, field: &str) -> u64 {
    value.get(field).and_then(serde_json::Value::as_u64).expect("numeric benchmark field")
}
