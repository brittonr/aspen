use std::collections::BTreeMap;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::node_state::NodeStateFileObservation;
use crate::node_state::NodeStateNamespace;
use crate::node_state::NodeStatePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanShellControl {
    pub max_items: usize,
    pub max_item_bytes: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableScanBinding {
    pub item_ref: String,
    pub path: NodeStatePath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOnlyContentObservation {
    pub status: ScanItemStatus,
    pub bytes: Option<Vec<u8>>,
    pub evidence_refs: Vec<String>,
}

pub trait ReadOnlyContentSource {
    fn observe_bounded(&self, item_ref: &str, max_bytes: u64) -> ReadOnlyContentObservation;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanExecution {
    pub observations: Vec<CanonicalArtifact<ScanObservation>>,
    pub result: CanonicalArtifact<IntegrityResult>,
}

// r[impl molten.fabric_observability.integrity_readonly]
// r[impl molten.fabric_observability.adapter_contract]
pub fn scan_durable_namespace(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
    namespace: &NodeStateNamespace,
    bindings: &[DurableScanBinding],
    control: &ScanShellControl,
) -> Result<ScanExecution> {
    canonical_integrity_plan(profile, plan)?;
    validate_scan_control(profile, plan, control)?;
    let binding_index = index_bindings(plan, bindings)?;
    let mut observations = Vec::new();
    if !control.cancelled {
        for target in plan.targets.iter().take(control.max_items) {
            let observation = match binding_index.get(&target.item_ref) {
                Some(binding) => observe_node_state_target(plan, target, namespace, binding, control.max_item_bytes),
                None => missing_observation(plan, target),
            };
            observations.push(observation);
        }
    }
    finish_scan(profile, plan, observations, control.cancelled)
}

// r[impl molten.fabric_observability.integrity_readonly]
pub fn scan_content_source(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
    source: &dyn ReadOnlyContentSource,
    control: &ScanShellControl,
) -> Result<ScanExecution> {
    canonical_integrity_plan(profile, plan)?;
    validate_scan_control(profile, plan, control)?;
    let mut observations = Vec::new();
    if !control.cancelled {
        for target in plan.targets.iter().take(control.max_items) {
            let source_observation = source.observe_bounded(&target.item_ref, control.max_item_bytes);
            observations.push(observation_from_source(plan, target, source_observation));
        }
    }
    finish_scan(profile, plan, observations, control.cancelled)
}

fn validate_scan_control(profile: &ObservationProfile, plan: &IntegrityPlan, control: &ScanShellControl) -> Result<()> {
    if control.max_items == 0 || control.max_items > plan.max_items || control.max_items > profile.bounds.max_scan_items
    {
        return Err(MoltenError::invalid_harness("integrity scan item bound exceeds the admitted plan or profile"));
    }
    if control.max_item_bytes == 0 {
        return Err(MoltenError::invalid_harness("integrity scan byte bound must be positive"));
    }
    Ok(())
}

fn index_bindings<'a>(
    plan: &IntegrityPlan,
    bindings: &'a [DurableScanBinding],
) -> Result<BTreeMap<String, &'a DurableScanBinding>> {
    if bindings.len() > plan.max_items {
        return Err(MoltenError::invalid_harness("durable scan binding count exceeds the admitted plan"));
    }
    let declared = plan
        .targets
        .iter()
        .map(|target| target.item_ref.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut index = BTreeMap::new();
    for binding in bindings {
        if !declared.contains(binding.item_ref.as_str()) {
            return Err(MoltenError::invalid_harness("durable scan binding is outside the admitted integrity plan"));
        }
        if index.insert(binding.item_ref.clone(), binding).is_some() {
            return Err(MoltenError::invalid_harness("duplicate durable scan binding item ref"));
        }
    }
    Ok(index)
}

fn observe_node_state_target(
    plan: &IntegrityPlan,
    target: &IntegrityTarget,
    namespace: &NodeStateNamespace,
    binding: &DurableScanBinding,
    max_bytes: u64,
) -> ScanObservation {
    let source = match namespace.observe_file(&binding.path) {
        Ok(NodeStateFileObservation::Missing) => ReadOnlyContentObservation {
            status: ScanItemStatus::Missing,
            bytes: None,
            evidence_refs: Vec::new(),
        },
        Ok(NodeStateFileObservation::NonRegular(_)) => ReadOnlyContentObservation {
            status: ScanItemStatus::Unsupported,
            bytes: None,
            evidence_refs: Vec::new(),
        },
        Ok(NodeStateFileObservation::Regular(file)) if file.size() > max_bytes => ReadOnlyContentObservation {
            status: ScanItemStatus::OverBound,
            bytes: None,
            evidence_refs: Vec::new(),
        },
        Ok(NodeStateFileObservation::Regular(file)) => match file.read_bounded(max_bytes) {
            Ok(bytes) => ReadOnlyContentObservation {
                status: ScanItemStatus::Present,
                bytes: Some(bytes),
                evidence_refs: Vec::new(),
            },
            Err(_) => ReadOnlyContentObservation {
                status: ScanItemStatus::PermissionDenied,
                bytes: None,
                evidence_refs: Vec::new(),
            },
        },
        Err(_) => ReadOnlyContentObservation {
            status: ScanItemStatus::PermissionDenied,
            bytes: None,
            evidence_refs: Vec::new(),
        },
    };
    observation_from_source(plan, target, source)
}

fn observation_from_source(
    plan: &IntegrityPlan,
    target: &IntegrityTarget,
    source: ReadOnlyContentObservation,
) -> ScanObservation {
    let observed_content_ref = source.bytes.as_deref().map(crate::preserves_rail::content_ref_from_bytes);
    let observed_length = source.bytes.as_ref().and_then(|bytes| u64::try_from(bytes.len()).ok());
    let observation_ref = scan_observation_ref(
        &plan.plan_ref,
        &target.item_ref,
        source.status,
        observed_content_ref.as_deref(),
        observed_length,
    );
    ScanObservation {
        schema: SCAN_OBSERVATION_SCHEMA.to_string(),
        observation_ref,
        plan_ref: plan.plan_ref.clone(),
        item_ref: target.item_ref.clone(),
        kind: target.kind,
        status: source.status,
        observed_content_ref,
        observed_length,
        evidence_refs: source.evidence_refs,
    }
}

fn missing_observation(plan: &IntegrityPlan, target: &IntegrityTarget) -> ScanObservation {
    observation_from_source(plan, target, ReadOnlyContentObservation {
        status: ScanItemStatus::Missing,
        bytes: None,
        evidence_refs: Vec::new(),
    })
}

fn finish_scan(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
    observations: Vec<ScanObservation>,
    cancelled: bool,
) -> Result<ScanExecution> {
    let completion = ScanCompletion {
        scanned_items: observations.len(),
        declared_items: plan.targets.len(),
        exhausted: !cancelled && observations.len() == plan.targets.len(),
        cancelled,
        unavailable: false,
    };
    let result = evaluate_integrity_plan(profile, plan, &observations, &completion);
    let canonical_observations = observations
        .iter()
        .map(|observation| canonical_scan_observation(plan, observation))
        .collect::<Result<Vec<_>>>()?;
    Ok(ScanExecution {
        observations: canonical_observations,
        result: canonical_integrity_result(profile, &result)?,
    })
}

fn scan_observation_ref(
    plan_ref: &str,
    item_ref: &str,
    status: ScanItemStatus,
    content_ref: Option<&str>,
    observed_length: Option<u64>,
) -> String {
    let identity = format!(
        "{plan_ref}\0{item_ref}\0{}\0{}\0{}",
        status.as_str(),
        content_ref.unwrap_or("none"),
        observed_length.map_or_else(|| "none".to_string(), |length| length.to_string())
    );
    crate::preserves_rail::content_ref_from_bytes(identity.as_bytes())
}
