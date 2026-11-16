use alloc::collections::BTreeSet;

use super::super::*;
use super::support::*;

const EXPECTED_SURFACES_PER_CLONE: usize = 6;
const EXPECTED_EFFECTS_PER_CLONE: usize = 6;

// r[verify vm_cohort.checkpoint_admission]
// r[verify vm_cohort.checkpoint_retention]
// r[verify vm_cohort.clone_isolation]
#[test]
fn complete_checkpoint_repeats_cohorts_without_consuming_immutable_bases() {
    let admitted = admit_checkpoint(&candidate()).expect("admitted checkpoint");
    let retained = record_retained_use(&admitted).expect("retained use");
    assert_eq!(retained.checkpoint_ref, admitted.checkpoint_ref);
    assert_eq!(
        retained.effective_memory_base_ref,
        admitted.effective_memory_base_ref
    );
    assert_eq!(
        retained.effective_disk_base_ref,
        admitted.effective_disk_base_ref
    );

    let first = plan_cohort(&request()).expect("first cohort plan");
    let second = plan_cohort(&request()).expect("stable cohort plan");
    assert_eq!(first, second);
    assert_eq!(
        first.clones.len(),
        usize::try_from(WORKERS).expect("workers")
    );
    assert_eq!(
        first.effects.len(),
        first.clones.len() * EXPECTED_EFFECTS_PER_CLONE
    );
    assert!(validate_clone_isolation(&first).is_empty());
    let surfaces = first
        .clones
        .iter()
        .flat_map(|clone| {
            clone
                .surfaces
                .iter()
                .map(|surface| surface.resource_ref.clone())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        surfaces.len(),
        first.clones.len() * EXPECTED_SURFACES_PER_CLONE
    );
}

// r[verify vm_cohort.conformance.negative]
#[test]
fn incomplete_incompatible_mutable_and_overbudget_inputs_fail_before_effects() {
    let mut invalid = candidate();
    invalid.complete = false;
    invalid.host_handles_present = true;
    invalid.bases_mutable = true;
    let issues = admit_checkpoint(&invalid).expect_err("invalid checkpoint denied");
    assert!(issues.contains(&CheckpointIssue::Incomplete));
    assert!(issues.contains(&CheckpointIssue::HostHandlePresent));
    assert!(issues.contains(&CheckpointIssue::MutableBase));

    let mut incompatible = request();
    incompatible.expected_compatibility.runtime_ref = resource('8');
    assert!(
        plan_cohort(&incompatible)
            .expect_err("compatibility drift denied")
            .contains(&CohortIssue::Compatibility)
    );

    let mut overbudget = request();
    overbudget.limits.maximum.memory_bytes = MEMORY_BYTES;
    assert!(
        plan_cohort(&overbudget)
            .expect_err("full reservation denied")
            .contains(&CohortIssue::ResourceLimit)
    );

    let mut overflow = request();
    overflow.checkpoint.memory_bytes = u64::MAX;
    assert!(
        plan_cohort(&overflow)
            .expect_err("overflow denied")
            .contains(&CohortIssue::ArithmeticOverflow)
    );
}
