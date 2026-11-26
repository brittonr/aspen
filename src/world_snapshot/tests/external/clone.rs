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

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn vm_cohort_plan_ref_hashes_the_explicit_preserves_projection() {
    let (chaos, descriptor) = fixture();
    let request = clone_request(&descriptor);
    let plan = snapshot_plan(&descriptor, &chaos, &request).expect("VM Cohort plan");
    let mechanism: vm_cohort_core::CohortPlan =
        serde_json::from_slice(&plan.mechanism_plan_json).expect("mechanism plan JSON");
    let kebab = |kind: serde_json::Value| kind.as_str().expect("kebab-case enum").to_string();
    let reservation = &mechanism.reservation;
    let clones = mechanism
        .clones
        .iter()
        .map(|clone| {
            let surfaces = clone
                .surfaces
                .iter()
                .map(|surface| {
                    format!(
                        "<surface-plan \"{}\" \"{}\">",
                        kebab(serde_json::to_value(surface.kind).expect("kind")),
                        surface.resource_ref.as_str()
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "<clone-plan \"{}\" \"{}\" \"{}\" [{surfaces}]>",
                clone.clone_ref.as_str(),
                clone.memory_base_ref.as_str(),
                clone.disk_base_ref.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let effects = mechanism
        .effects
        .iter()
        .map(|effect| {
            format!(
                "<effect-request \"{}\" \"{}\" \"{}\">",
                effect.operation_ref.as_str(),
                effect.clone_ref.as_str(),
                kebab(serde_json::to_value(effect.kind).expect("kind"))
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let projection = format!(
        "<cohort-plan \"{}\" \"{}\" \"{}\" <resource-vector {} {} {} {} {} {} {}> [{clones}] [{effects}]>",
        mechanism.cohort_ref.as_str(),
        mechanism.checkpoint_ref.as_str(),
        mechanism.policy_ref.as_str(),
        reservation.workers,
        reservation.vcpus,
        reservation.memory_bytes,
        reservation.storage_bytes,
        reservation.mappings,
        reservation.endpoints,
        reservation.in_flight,
    );
    let bytes = crate::preserves_rail::canonical_bytes(
        &crate::preserves_rail::parse_text(&projection).expect("plan projection text"),
    )
    .expect("canonical plan projection");
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-snapshot.vm-cohort-plan.v2");
    for part in [b"mechanism-plan".as_slice(), bytes.as_slice()] {
        hasher.update(&u64::try_from(part.len()).expect("bounded identity input").to_le_bytes());
        hasher.update(part);
    }
    assert!(!mechanism.effects.is_empty(), "fixture must exercise the effect projection");
    assert_eq!(plan.plan_ref, format!("blake3:{}", hasher.finalize().to_hex()));
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
