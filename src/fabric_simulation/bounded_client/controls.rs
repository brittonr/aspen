use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticControlVerdict {
    Healthy,
    ReachedDetected { property: String },
    NotReached,
    ReachedSurvived,
    Invalid,
    Unsupported,
    Incomplete,
}

/// Deployment alone is not detection. The intended property must reject an
/// executed divergence in this very history; suppression invalidates the run.
// r[impl molten.chaoscontrol_client.controls]
pub fn classify_semantic_control(history: &History, check: &Check) -> SemanticControlVerdict {
    match &check.verdict {
        Verdict::Invalid { .. } => return SemanticControlVerdict::Invalid,
        Verdict::Unsupported => return SemanticControlVerdict::Unsupported,
        _ => {}
    }
    if history.deployed_control == SemanticControl::Healthy {
        return if check.verdict == Verdict::Pass {
            SemanticControlVerdict::Healthy
        } else {
            SemanticControlVerdict::Incomplete
        };
    }
    if !history.is_control_reached {
        return SemanticControlVerdict::NotReached;
    }
    match &check.verdict {
        Verdict::Fail { property, .. } if property == APPLY_ONCE_PROPERTY => SemanticControlVerdict::ReachedDetected {
            property: property.clone(),
        },
        Verdict::Pass => SemanticControlVerdict::ReachedSurvived,
        _ => SemanticControlVerdict::Incomplete,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestDependencies {
    pub vcpus_per_guest: u32,
    pub source_revision: String,
    pub source_ref: String,
    pub binary_ref: Option<String>,
    pub image_ref: Option<String>,
    pub kernel_ref: Option<String>,
    pub dependency_closure_ref: Option<String>,
    pub adapter_ref: String,
    pub oracle_ref: String,
    pub workload_schema_ref: String,
    pub property_catalog_ref: String,
    pub generator_ref: String,
}

// Two profile checks plus the ten independently required dependency identities.
const MAX_DEPENDENCY_BLOCKERS: usize = 12;

/// Preflight only. ChaosControl's existing MultiNodeGuestIntake, cohort and
/// replay validators still own execution admission and qualification.
// r[impl molten.chaoscontrol_client.guest_boundary]
pub fn guest_dependency_blockers(dependencies: &GuestDependencies) -> Vec<String> {
    let mut blockers = Vec::with_capacity(MAX_DEPENDENCY_BLOCKERS);
    if dependencies.vcpus_per_guest != 1 {
        blockers.push("unsupported guest vCPU profile".to_string());
    }
    if dependencies.source_revision.len() != 40
        || !dependencies.source_revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("immutable source revision unavailable".to_string());
    }
    for (name, reference) in [
        ("source", Some(dependencies.source_ref.as_str())),
        ("binary", dependencies.binary_ref.as_deref()),
        ("image", dependencies.image_ref.as_deref()),
        ("kernel", dependencies.kernel_ref.as_deref()),
        ("dependency closure", dependencies.dependency_closure_ref.as_deref()),
        ("adapter", Some(dependencies.adapter_ref.as_str())),
        ("oracle", Some(dependencies.oracle_ref.as_str())),
        ("workload schema", Some(dependencies.workload_schema_ref.as_str())),
        ("property catalog", Some(dependencies.property_catalog_ref.as_str())),
        ("generator", Some(dependencies.generator_ref.as_str())),
    ] {
        if reference.is_none_or(|reference| crate::preserves_rail::validate_content_ref(reference).is_err()) {
            blockers.push(format!("{name} identity unavailable"));
        }
    }
    blockers
}
