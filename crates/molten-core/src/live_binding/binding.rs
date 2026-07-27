use artifact_binding_core::ReachabilityLimits;
use artifact_binding_core::ReachabilitySnapshot;
use artifact_binding_core::ResolutionLimits;
use artifact_binding_core::ResolutionRequest;
use artifact_binding_core::RetirementClassification;
use artifact_binding_core::RetirementCompleteness;
use artifact_binding_core::RetirementIssue;
use artifact_binding_core::RetirementLimits;
use artifact_binding_core::RetirementRequest;
use artifact_binding_core::RootClassCompleteness;
use artifact_binding_core::TransitionError;
use artifact_binding_core::classify_retirement;
use artifact_binding_core::plan_transition;
use artifact_binding_core::resolve;

use super::ATTRIBUTION_LIMIT;
use super::BINDING_NON_CLAIMS;
use super::DIAGNOSTIC_LIMIT;
use super::DeployDiagnostic;
use super::EDGE_LIMIT;
use super::LiveBindingError;
use super::MoltenCutoverPlan;
use super::MoltenCutoverRequest;
use super::MoltenRetirementReport;
use super::PIN_PATH_NODE_LIMIT;
use super::ProductGateFacts;
use super::REACHABLE_NODE_LIMIT;
use super::REQUIRED_ROOT_CLASSES;
use super::RETIREMENT_ISSUE_LIMIT;
use super::ROOT_CLASS_LIMIT;
use super::ROOT_LIMIT;
use super::RootInventoryInput;
use super::SNAPSHOT_BINDING_LIMIT;
use super::SYSTEM_EXTENSION_LATE_BINDING_PILOT_PROFILE;
use super::SourcePinObservation;
use super::SourcePinReport;
use super::SystemExtensionCallbackInput;
use super::UnitBoundary;
use super::UnitResolution;
use super::UnitResolutionInput;
use super::root_class;

pub fn validate_source_pins(observation: &SourcePinObservation) -> SourcePinReport {
    let artifact_binding_exact = observation.artifact_binding_source == super::ARTIFACT_BINDING_SOURCE
        && observation.artifact_binding_revision == super::ARTIFACT_BINDING_REVISION;
    let kamacite_exact = observation.kamacite_source == super::KAMACITE_SEMANTIC_SOURCE
        && observation.kamacite_revision == super::KAMACITE_SEMANTIC_REVISION;
    SourcePinReport {
        artifact_binding_exact,
        kamacite_exact,
        release_eligible: artifact_binding_exact && kamacite_exact,
    }
}

fn require_product_gates(gates: &ProductGateFacts) -> Result<(), LiveBindingError> {
    let required = [
        ("target-loaded", gates.target_loaded),
        ("target-verified", gates.target_verified),
        ("product-compatible", gates.product_compatible),
        ("migration-satisfied", !gates.migration_required || gates.migration_satisfied),
        ("authority-admitted", gates.authority_admitted),
        ("policy-admitted", gates.policy_admitted),
        ("provenance-admitted", gates.provenance_admitted),
        ("resource-admitted", gates.resource_admitted),
        ("lifecycle-admitted", gates.lifecycle_admitted),
    ];
    for (gate, admitted) in required {
        if !admitted {
            return Err(LiveBindingError::ProductGateDenied { gate });
        }
    }
    Ok(())
}

// r[impl molten.artifacts.live_binding.cutover]
// r[impl molten.artifacts.live_binding.non_authority]
pub fn plan_molten_cutover(input: &MoltenCutoverRequest) -> Result<MoltenCutoverPlan, LiveBindingError> {
    require_product_gates(&input.gates)?;
    let shared_plan =
        plan_transition(&input.transition).map_err(|error| LiveBindingError::SharedTransition(format!("{error:?}")))?;
    Ok(MoltenCutoverPlan {
        shared_plan,
        publication_authorized: false,
        non_claims: BINDING_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    })
}

// r[impl molten.artifacts.live_binding.unit_resolution]
pub fn resolve_unit_once(input: &UnitResolutionInput) -> Result<UnitResolution, LiveBindingError> {
    let late_bound = matches!(input.request, ResolutionRequest::LateBound(_));
    if input.nested_lookup && late_bound && !input.nested_late_binding_declared {
        return Err(LiveBindingError::ImplicitNestedLateBinding);
    }
    let shared_resolution = resolve(&input.request, input.snapshot.as_ref(), ResolutionLimits {
        max_bindings: SNAPSHOT_BINDING_LIMIT,
    })
    .map_err(|error| LiveBindingError::SharedResolution(format!("{error:?}")))?;
    let matching = input
        .closures
        .iter()
        .filter(|closure| closure.artifact == shared_resolution.target)
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Err(LiveBindingError::MissingDependencyClosure);
    }
    if matching.len() != 1 {
        return Err(LiveBindingError::DuplicateDependencyClosure);
    }
    let mut pinned_dependencies = matching[0].dependencies.clone();
    pinned_dependencies.sort();
    pinned_dependencies.dedup();
    Ok(UnitResolution {
        boundary: input.boundary,
        shared_resolution,
        pinned_dependencies,
        nested_lookup_authorized: input.nested_lookup && input.nested_late_binding_declared,
        non_claims: BINDING_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    })
}

// r[impl molten.artifacts.live_binding.unit_resolution]
pub fn resolve_system_extension_callback(
    input: &SystemExtensionCallbackInput,
) -> Result<UnitResolution, LiveBindingError> {
    if input.profile != SYSTEM_EXTENSION_LATE_BINDING_PILOT_PROFILE
        || input.resolution.boundary != UnitBoundary::CallbackPass
    {
        return Err(LiveBindingError::UnsupportedLateBindingProfile);
    }
    resolve_unit_once(&input.resolution)
}

fn declared_root_classes() -> Result<Vec<artifact_binding_core::RootClassId>, LiveBindingError> {
    REQUIRED_ROOT_CLASSES.iter().map(|class| root_class(class)).collect()
}

fn normalized_completeness(input: &RootInventoryInput) -> Result<Vec<RootClassCompleteness>, LiveBindingError> {
    let declared = declared_root_classes()?;
    let mut observations = Vec::with_capacity(declared.len());
    for class in declared {
        let supplied = input
            .class_completeness
            .iter()
            .find(|observation| observation.class == class)
            .is_some_and(|observation| observation.complete);
        observations.push(RootClassCompleteness {
            class,
            complete: input.instrumented && supplied,
        });
    }
    Ok(observations)
}

fn validate_inventory_classes(input: &RootInventoryInput) -> Result<(), LiveBindingError> {
    let declared = declared_root_classes()?;
    for root in &input.roots {
        if !declared.contains(&root.class) {
            return Err(LiveBindingError::InvalidRootClass(root.class.as_str().to_string()));
        }
    }
    for observation in &input.class_completeness {
        if !declared.contains(&observation.class) {
            return Err(LiveBindingError::InvalidRootClass(observation.class.as_str().to_string()));
        }
    }
    Ok(())
}

fn retirement_limits() -> RetirementLimits {
    RetirementLimits {
        reachability: ReachabilityLimits {
            max_roots: ROOT_LIMIT,
            max_edges: EDGE_LIMIT,
            max_nodes: REACHABLE_NODE_LIMIT,
            max_path_nodes: PIN_PATH_NODE_LIMIT,
            max_diagnostics: DIAGNOSTIC_LIMIT,
        },
        max_attributions: ATTRIBUTION_LIMIT,
        max_root_classes: ROOT_CLASS_LIMIT,
        max_issues: RETIREMENT_ISSUE_LIMIT,
    }
}

// r[impl molten.retirement.root_inventory]
// r[impl molten.retirement.classification]
// r[impl molten.retirement.gc_boundary]
pub fn classify_generation(input: &RootInventoryInput) -> Result<MoltenRetirementReport, LiveBindingError> {
    validate_inventory_classes(input)?;
    let request = RetirementRequest {
        generation: input.generation.clone(),
        declared_root_classes: declared_root_classes()?,
        graph: ReachabilitySnapshot {
            id: input.snapshot.clone(),
            roots: input.roots.clone(),
            edges: input.edges.clone(),
        },
        attributions: input.attributions.clone(),
        completeness: RetirementCompleteness {
            snapshot: input.snapshot.clone(),
            root_classes: normalized_completeness(input)?,
            edge_inventory_complete: input.instrumented && input.edge_inventory_complete,
            attribution_inventory_complete: input.instrumented && input.attribution_inventory_complete,
        },
    };
    let decision = classify_retirement(&request, retirement_limits())
        .map_err(|error| LiveBindingError::SharedRetirement(format!("{error:?}")))?;
    Ok(MoltenRetirementReport {
        profile: input.profile.clone(),
        snapshot: input.snapshot.clone(),
        decision,
        observation_only: true,
        retention_authorized: false,
        deletion_authorized: false,
        non_claims: BINDING_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    })
}

fn transition_diagnostic(error: &TransitionError) -> DeployDiagnostic {
    match error {
        TransitionError::CompatibilityMissing
        | TransitionError::CompatibilityBindingMismatch
        | TransitionError::CompatibilityRejected => DeployDiagnostic::IncompatibleTarget,
        TransitionError::SuccessorReachabilityMissing
        | TransitionError::SuccessorReachabilityBindingMismatch
        | TransitionError::SuccessorUnreachable => DeployDiagnostic::UnreachableSuccessor,
        _ => DeployDiagnostic::StaleCompareAndSwap,
    }
}

// r[impl molten.retirement.deploy_diagnostics]
pub fn diagnose_transition(error: &TransitionError) -> Vec<DeployDiagnostic> {
    vec![transition_diagnostic(error)]
}

// r[impl molten.retirement.trace_report]
pub fn diagnose_retirement(report: &MoltenRetirementReport) -> Vec<DeployDiagnostic> {
    let mut diagnostics = Vec::new();
    for issue in &report.decision.issues {
        match issue {
            RetirementIssue::IncompleteRootClass { class } | RetirementIssue::UndeclaredRootClass { class } => {
                diagnostics.push(DeployDiagnostic::IncompleteRootInventory {
                    root_class: class.as_str().to_string(),
                });
            }
            RetirementIssue::MalformedSharedAttribution { .. } | RetirementIssue::ContradictoryAttribution { .. } => {
                diagnostics.push(DeployDiagnostic::AmbiguousAttribution);
            }
            _ => {}
        }
    }
    for path in &report.decision.pin_paths {
        diagnostics.push(DeployDiagnostic::LivePinPath {
            root_class: path.root_class.as_str().to_string(),
            root_id: path.root_id.as_str().to_string(),
            target: path.target.as_str().to_string(),
        });
    }
    diagnostics.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
    diagnostics.dedup();
    diagnostics
}

pub fn retirement_is_complete(report: &MoltenRetirementReport) -> bool {
    report.decision.classification == RetirementClassification::Retired
}
