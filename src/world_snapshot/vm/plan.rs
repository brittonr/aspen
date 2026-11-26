//! Explicit Preserves identity projection of a VM Cohort plan.

use preserves::IOValue;
use vm_cohort_core as vm;

use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

/// The identity projection of a VM Cohort plan. Every struct is destructured exhaustively (no `..`)
/// and every enum is matched without a wildcard, so an upstream field or variant change fails to
/// compile here instead of silently entering or escaping the plan identity. Clones and effects keep
/// their planned order.
pub(super) fn cohort_plan_value(plan: &vm::CohortPlan) -> IOValue {
    let vm::CohortPlan {
        cohort_ref,
        checkpoint_ref,
        policy_ref,
        reservation,
        clones,
        effects,
    } = plan;
    let reservation = resource_vector_value(reservation);
    let clones = clones.iter().map(clone_plan_value).collect();
    let effects = effects.iter().map(effect_request_value).collect();
    record("cohort-plan", vec![
        string(cohort_ref.as_str()),
        string(checkpoint_ref.as_str()),
        string(policy_ref.as_str()),
        reservation,
        sequence(clones),
        sequence(effects),
    ])
}

fn resource_vector_value(reservation: &vm::ResourceVector) -> IOValue {
    let vm::ResourceVector {
        workers,
        vcpus,
        memory_bytes,
        storage_bytes,
        mappings,
        endpoints,
        in_flight,
    } = reservation;
    record("resource-vector", vec![
        u64_value(u64::from(*workers)),
        u64_value(u64::from(*vcpus)),
        u64_value(*memory_bytes),
        u64_value(*storage_bytes),
        u64_value(u64::from(*mappings)),
        u64_value(u64::from(*endpoints)),
        u64_value(u64::from(*in_flight)),
    ])
}

fn clone_plan_value(clone: &vm::ClonePlan) -> IOValue {
    let vm::ClonePlan {
        clone_ref,
        memory_base_ref,
        disk_base_ref,
        surfaces,
    } = clone;
    let surfaces = surfaces.iter().map(surface_plan_value).collect();
    record("clone-plan", vec![
        string(clone_ref.as_str()),
        string(memory_base_ref.as_str()),
        string(disk_base_ref.as_str()),
        sequence(surfaces),
    ])
}

fn surface_plan_value(surface: &vm::SurfacePlan) -> IOValue {
    let vm::SurfacePlan { kind, resource_ref } = surface;
    let kind = match kind {
        vm::MutableSurfaceKind::Memory => "memory",
        vm::MutableSurfaceKind::Disk => "disk",
        vm::MutableSurfaceKind::Device => "device",
        vm::MutableSurfaceKind::Endpoint => "endpoint",
        vm::MutableSurfaceKind::Output => "output",
        vm::MutableSurfaceKind::Lifecycle => "lifecycle",
    };
    record("surface-plan", vec![string(kind), string(resource_ref.as_str())])
}

fn effect_request_value(effect: &vm::EffectRequest) -> IOValue {
    let vm::EffectRequest {
        operation_ref,
        clone_ref,
        kind,
    } = effect;
    let kind = match kind {
        vm::EffectKind::PrepareMemory => "prepare-memory",
        vm::EffectKind::PrepareDisk => "prepare-disk",
        vm::EffectKind::RestoreVcpu => "restore-vcpu",
        vm::EffectKind::RestoreDevice => "restore-device",
        vm::EffectKind::BindEndpoint => "bind-endpoint",
        vm::EffectKind::Activate => "activate",
    };
    record("effect-request", vec![string(operation_ref.as_str()), string(clone_ref.as_str()), string(kind)])
}
