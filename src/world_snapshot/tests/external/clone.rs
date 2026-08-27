use super::super::super::*;
use super::support::EFFECTIVE_DISK_BYTES;
use super::support::clone_request;
use super::support::digest;
use super::support::fixture;
use super::support::limits;
use crate::error::Result;

// r[verify molten.world_snapshot.cow]
#[test]
fn vm_cohort_plan_binds_parent_children_and_private_surfaces() {
    let (chaos, descriptor) = fixture();
    let request = clone_request(&descriptor);
    let plan = snapshot_plan(&descriptor, &chaos, &request).expect("VM Cohort plan");

    assert_eq!(plan.worker_count, 2);
    assert_eq!(plan.clones.len(), request.children.len());
    assert!(!plan.fault_authority_granted);
    assert!(!plan.replay_authority_granted);
    assert!(!plan.release_authority_granted);
    assert!(plan.clones.iter().all(|clone| !clone.vm_surface_refs.is_empty()));
    let mechanism: vm_cohort_core::CohortPlan =
        serde_json::from_slice(&plan.mechanism_plan_json).expect("mechanism plan JSON");
    assert_eq!(mechanism.clones.len(), plan.clones.len());
}

// r[verify molten.world_snapshot.cow]
#[test]
fn complete_realization_is_admitted_without_product_authority() {
    let (chaos, descriptor) = fixture();
    let request = clone_request(&descriptor);
    let plan = snapshot_plan(&descriptor, &chaos, &request).expect("VM Cohort plan");
    let mut port = CompleteRealization;
    let observation = realize_vm_cohort_clones(&plan, &mut port).expect("complete VM Cohort realization");
    assert_eq!(observation.active_clones, plan.worker_count);
    assert!(!observation.cleanup_uncertain);
    assert!(!observation.fault_authority_granted);
    assert!(!observation.replay_authority_granted);
    assert!(!observation.release_authority_granted);
}

// r[verify molten.world_snapshot.verification]
#[test]
fn partial_uncertain_and_overclaiming_realizations_fail_closed() {
    let (chaos, descriptor) = fixture();
    let request = clone_request(&descriptor);
    let plan = snapshot_plan(&descriptor, &chaos, &request).expect("VM Cohort plan");

    let mut partial = PartialRealization;
    assert!(realize_vm_cohort_clones(&plan, &mut partial).is_err());

    let mut overclaim = AuthorityRealization;
    assert!(realize_vm_cohort_clones(&plan, &mut overclaim).is_err());
}

// r[verify molten.world_snapshot.verification]
#[test]
fn vm_cohort_planning_rejects_parent_overlay_and_disk_drift() {
    let (chaos, descriptor) = fixture();
    let mut request = clone_request(&descriptor);
    request.children[0].parent_ref =
        molten_core::world_commit::WorldCommitRef::new(digest('f')).expect("drifted parent");
    assert!(snapshot_plan(&descriptor, &chaos, &request).is_err());

    let mut request = clone_request(&descriptor);
    request.children[1].memory_overlay = request.children[0].memory_overlay.clone();
    assert!(snapshot_plan(&descriptor, &chaos, &request).is_err());

    let request = clone_request(&descriptor);
    assert!(plan_vm_cohort_clones(&descriptor, &chaos, &request, 0, &limits()).is_err());
}

struct CompleteRealization;

impl VmCohortRealizationPort for CompleteRealization {
    fn realize(&mut self, plan: &VmCohortPlanProjection) -> Result<VmCohortRealizationObservation> {
        Ok(observation(plan, plan.worker_count, false, false))
    }
}

struct PartialRealization;

impl VmCohortRealizationPort for PartialRealization {
    fn realize(&mut self, plan: &VmCohortPlanProjection) -> Result<VmCohortRealizationObservation> {
        Ok(observation(plan, plan.worker_count.saturating_sub(1), true, false))
    }
}

struct AuthorityRealization;

impl VmCohortRealizationPort for AuthorityRealization {
    fn realize(&mut self, plan: &VmCohortPlanProjection) -> Result<VmCohortRealizationObservation> {
        Ok(observation(plan, plan.worker_count, false, true))
    }
}

fn snapshot_plan(
    descriptor: &molten_core::world_snapshot::SnapshotDescriptor,
    chaos: &chaoscontrol_snapshot_descriptor::SnapshotDescriptorEnvelope,
    request: &molten_core::world_snapshot::ClonePlanRequest,
) -> Result<VmCohortPlanProjection> {
    plan_vm_cohort_clones(descriptor, chaos, request, EFFECTIVE_DISK_BYTES, &limits())
}

fn observation(
    plan: &VmCohortPlanProjection,
    active_clones: u32,
    cleanup_uncertain: bool,
    release_authority_granted: bool,
) -> VmCohortRealizationObservation {
    VmCohortRealizationObservation {
        plan_ref: plan.plan_ref.clone(),
        cohort_ref: plan.cohort_ref.clone(),
        mechanism_receipt_ref: digest('e'),
        active_clones,
        cleanup_uncertain,
        fault_authority_granted: false,
        replay_authority_granted: false,
        release_authority_granted,
    }
}
