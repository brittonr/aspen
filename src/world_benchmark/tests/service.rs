use chaoscontrol_snapshot_descriptor::SnapshotDescriptor;
use molten_core::world_benchmark::*;

use super::super::*;
use super::support::*;

const EXPECTED_RESULTS: usize = 16;
const ROOT_REFERENCE_BYTES: u64 = 64;
const SNAPSHOT_PAGE_BYTES: u64 = 4_096;
const SNAPSHOT_COPIED_PAGES: u64 = 64;
const SNAPSHOT_MAPPED_PAGES: u64 = 32_704;
const SNAPSHOT_PHYSICAL_BYTES: u64 = SNAPSHOT_PAGE_BYTES * SNAPSHOT_COPIED_PAGES;

// r[verify molten.world_bench.profile]
#[test]
fn nickel_projection_is_revalidated_by_rust_before_execution() {
    let (profile, dataset) = decode_world_benchmark_input(
        include_bytes!("../../../config/world-benchmark/generated/logical-synthetic.json"),
        SOURCE_REVISION,
    )
    .expect("checked logical projection");
    assert_eq!(profile.dataset_ref, dataset.dataset_ref);
    assert_eq!(profile.preparation, dataset.preparation);

    let (opaque, _) = decode_world_benchmark_input(
        include_bytes!("../../../config/world-benchmark/generated/opaque-chaoscontrol.json"),
        SOURCE_REVISION,
    )
    .expect("checked opaque projection");
    assert_eq!(opaque.class, WorldBenchmarkClass::OpaqueExactSnapshot);
}

// r[verify molten.world_bench.verification]
#[test]
fn deterministic_fixture_repeats_exact_counts_and_publishes_receipt_last() {
    let profile = profile();
    let first = run(&profile).expect("first benchmark run");
    let second = run(&profile).expect("repeated benchmark run");
    assert_eq!(first.plan.plan_ref, second.plan.plan_ref);
    assert_eq!(first.receipt.receipt_ref, second.receipt.receipt_ref);
    assert_eq!(first.receipt.results.len(), EXPECTED_RESULTS);
    assert_eq!(first.published_receipt_ref, first.receipt_record.record_ref);
    let branch = first
        .receipt
        .results
        .iter()
        .find(|result| result.operation == WorldBenchmarkOperation::RootBranch)
        .expect("root branch result");
    assert_eq!(branch.metric(WorldBenchmarkMetricKind::ReusedObjects), Some(OBJECT_COUNT));
    assert_eq!(branch.metric(WorldBenchmarkMetricKind::PhysicalBytesWritten), Some(ROOT_REFERENCE_BYTES));
}

// r[verify molten.world_bench.retention]
#[test]
fn hidden_prepopulation_and_protected_deletion_candidates_fail_closed() {
    let profile = profile();
    let mut dataset_port = fixture(&profile);
    dataset_port.preparation.prior_objects_available = true;
    let mut operation_port = fixture(&profile);
    let mut resource_port = fixture(&profile);
    let mut snapshot_port = fixture(&profile);
    let mut receipt_port = fixture(&profile);
    let error = run_world_benchmark(&profile, SOURCE_REVISION, "molten".to_string(), WorldBenchmarkPorts {
        datasets: &mut dataset_port,
        operations: &mut operation_port,
        resources: &mut resource_port,
        snapshots: &mut snapshot_port,
        receipts: &mut receipt_port,
    })
    .expect_err("hidden prepopulation denied");
    assert!(error.to_string().contains("HiddenPrepopulation"));

    let facts = WorldBenchmarkOperationFacts {
        adapter_ref: ADAPTER.to_string(),
        logical_bytes: 0,
        physical_bytes_written: 0,
        new_objects: 0,
        reused_objects: 0,
        copied_pages: 0,
        mapped_pages: 0,
        traversed_references: OBJECT_COUNT,
        compared_keys: 0,
        emitted_conflicts: 0,
        transferred_bytes: 0,
        retained_objects: OBJECT_COUNT,
        planned_deletions: 1,
        protected_deletion_candidates: 1,
        physical_measurement_independent: true,
    };
    assert!(instrument_world_benchmark_facts(WorldBenchmarkOperation::RetentionPlan, &facts).is_err());
}

// r[verify molten.world_bench.snapshot_profiles]
#[test]
fn published_chaoscontrol_descriptor_binds_one_exact_opaque_cohort() {
    let descriptor: SnapshotDescriptor = serde_json::from_str(include_str!(
        "../../../tests/fixtures/world-benchmark/chaoscontrol-snapshot-descriptor.valid.json"
    ))
    .expect("published descriptor fixture");
    let observed =
        instrument_chaoscontrol_snapshot(reference('d'), &descriptor, &ChaosControlSnapshotSharingObservation {
            observation_ref: reference('e'),
            adapter_ref: "chaoscontrol-exact-snapshot-descriptor-v1".to_string(),
            page_size_bytes: SNAPSHOT_PAGE_BYTES,
            copied_pages: SNAPSHOT_COPIED_PAGES,
            mapped_pages: SNAPSHOT_MAPPED_PAGES,
            physical_bytes_written: SNAPSHOT_PHYSICAL_BYTES,
        })
        .expect("exact snapshot sharing observation");
    assert_eq!(observed.binding.source_revision, CHAOSCONTROL_SNAPSHOT_REVISION);
    assert_eq!(observed.binding.completeness_profile, CHAOSCONTROL_SNAPSHOT_PROFILE);
    assert_eq!(observed.binding.memory_bytes, descriptor.topology.memory_bytes);
    assert_eq!(
        observed
            .operation
            .metrics
            .iter()
            .find(|metric| metric.kind == WorldBenchmarkMetricKind::CopiedPages),
        Some(&WorldBenchmarkMetric {
            kind: WorldBenchmarkMetricKind::CopiedPages,
            value: SNAPSHOT_COPIED_PAGES,
        })
    );
}

fn run(profile: &WorldBenchmarkProfile) -> crate::error::Result<WorldBenchmarkRunOutcome> {
    let mut dataset_port = fixture(profile);
    let mut operation_port = fixture(profile);
    let mut resource_port = fixture(profile);
    let mut snapshot_port = fixture(profile);
    let mut receipt_port = fixture(profile);
    run_world_benchmark(profile, SOURCE_REVISION, "molten".to_string(), WorldBenchmarkPorts {
        datasets: &mut dataset_port,
        operations: &mut operation_port,
        resources: &mut resource_port,
        snapshots: &mut snapshot_port,
        receipts: &mut receipt_port,
    })
}
