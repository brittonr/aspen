use std::path::Path;

use super::CHILD_OBJECT_BYTES;
use super::workload;
use crate::error::Result;
use crate::marble_store::MeasureInput;
use crate::marble_store::compare_paths;
use crate::marble_store::decision_value;
use crate::marble_store::keep_or_replace;
use crate::marble_store::measurement_value;

const BATCHES: u32 = 3;
const OBJECTS_PER_BATCH: u32 = 4;

// r[verify aspen.marble_store.spike]
// r[verify aspen.marble_store.verification]
#[test]
fn comparison_records_write_read_recovery_and_space_numbers() -> Result<()> {
    let heap_path = super::heap_root("measure-heap");
    let chunk_root = super::heap_root("measure-chunks");
    let report = compare_paths(&MeasureInput {
        heap_path: Path::new(&heap_path),
        chunk_root: Path::new(&chunk_root),
        batches: BATCHES,
        objects_per_batch: OBJECTS_PER_BATCH,
        object_bytes: 1024,
    })?;
    let expected_objects = (BATCHES * OBJECTS_PER_BATCH) as usize;
    assert_eq!(report.spike_round_trips, expected_objects);
    assert_eq!(report.current_round_trips, expected_objects);
    assert!(report.spike.batch_write_micros > 0 || report.current.batch_write_micros > 0);
    assert_eq!(report.recovered_mappings, expected_objects);
    assert!(report.spike.physical_bytes > 0);
    assert!(report.current.physical_bytes > 0);
    Ok(())
}

// r[verify aspen.marble_store.verification]
#[test]
fn decision_record_binds_all_four_measurement_families() -> Result<()> {
    let heap_path = super::heap_root("decision-heap");
    let chunk_root = super::heap_root("decision-chunks");
    let report = compare_paths(&MeasureInput {
        heap_path: Path::new(&heap_path),
        chunk_root: Path::new(&chunk_root),
        batches: BATCHES,
        objects_per_batch: OBJECTS_PER_BATCH,
        object_bytes: 1024,
    })?;
    let decision = keep_or_replace(&report);
    assert!(matches!(decision.decision, "keep-current-path" | "replace-candidate"));
    assert!(decision.round_trips_held);
    let measurement_text = crate::preserves_rail::to_text(&measurement_value(&report))?;
    assert!(measurement_text.contains("marble-store-measurement"));
    let decision_text = crate::preserves_rail::to_text(&decision_value(&report, &decision))?;
    assert!(decision_text.contains("marble-store-decision"));
    assert!(decision_text.contains("molten.marble-store.decision.v1"));
    for anchor in ["batch-write", "point-read", "recovery", "space-amplification"] {
        assert!(
            decision_text.contains(anchor) || measurement_text.contains(anchor),
            "the decision record family {anchor} must stay bound"
        );
    }
    Ok(())
}

#[test]
fn measurement_inputs_out_of_bounds_fail_admission() {
    let heap_path = super::heap_root("measure-bounds");
    let chunk_root = super::heap_root("measure-bounds-chunks");
    let denied = compare_paths(&MeasureInput {
        heap_path: Path::new(&heap_path),
        chunk_root: Path::new(&chunk_root),
        batches: 0,
        objects_per_batch: OBJECTS_PER_BATCH,
        object_bytes: 1024,
    });
    assert!(denied.is_err());
}

#[test]
fn workload_bytes_are_deterministic_and_bounded() {
    let first = workload(1, 1, CHILD_OBJECT_BYTES);
    let second = workload(1, 1, CHILD_OBJECT_BYTES);
    assert_eq!(first, second);
    assert_eq!(first.len(), CHILD_OBJECT_BYTES);
    assert_ne!(first, workload(1, 2, CHILD_OBJECT_BYTES));
}
