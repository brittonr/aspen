use artifact_binding_core::ArtifactAttribution;
use artifact_binding_core::ArtifactId;
use artifact_binding_core::BindingKey;
use artifact_binding_core::BindingRecord;
use artifact_binding_core::BindingRevision;
use artifact_binding_core::BindingSnapshot;
use artifact_binding_core::CompatibilityObservation;
use artifact_binding_core::ContractId;
use artifact_binding_core::ExpectedBinding;
use artifact_binding_core::GenerationAttribution;
use artifact_binding_core::GenerationId;
use artifact_binding_core::GraphEdge;
use artifact_binding_core::GraphRoot;
use artifact_binding_core::LateBoundRequest;
use artifact_binding_core::ObservationId;
use artifact_binding_core::ProposedBinding;
use artifact_binding_core::ResolutionRequest;
use artifact_binding_core::RetirementClassification;
use artifact_binding_core::RootClassCompleteness;
use artifact_binding_core::RootId;
use artifact_binding_core::SnapshotId;
use artifact_binding_core::TargetReachabilityObservation;
use artifact_binding_core::TransitionError;
use artifact_binding_core::TransitionPolicy;
use artifact_binding_core::TransitionRequest;
use artifact_binding_core::plan_transition;

use super::super::*;

const CURRENT_REVISION: u64 = 7;
const NEXT_REVISION: u64 = CURRENT_REVISION + 1;
const STALE_REVISION: u64 = CURRENT_REVISION - 1;

fn artifact(value: &str) -> ArtifactId {
    ArtifactId::try_new(value, IDENTIFIER_MAXIMUM_BYTES).expect("valid artifact fixture")
}

fn key() -> BindingKey {
    BindingKey::try_new("service:search", IDENTIFIER_MAXIMUM_BYTES).expect("valid binding key fixture")
}

fn contract() -> ContractId {
    ContractId::try_new("contract:search-v1", IDENTIFIER_MAXIMUM_BYTES).expect("valid contract fixture")
}

fn snapshot(value: &str) -> SnapshotId {
    SnapshotId::try_new(value, IDENTIFIER_MAXIMUM_BYTES).expect("valid snapshot fixture")
}

fn generation(value: &str) -> GenerationId {
    GenerationId::try_new(value, IDENTIFIER_MAXIMUM_BYTES).expect("valid generation fixture")
}

fn observation(value: &str) -> ObservationId {
    ObservationId::try_new(value, IDENTIFIER_MAXIMUM_BYTES).expect("valid observation fixture")
}

fn current_binding(snapshot: &SnapshotId, target: &str) -> BindingRecord {
    BindingRecord {
        key: key(),
        revision: BindingRevision::new(CURRENT_REVISION),
        target: artifact(target),
        contract: contract(),
        snapshot: snapshot.clone(),
        active: true,
    }
}

fn transition() -> TransitionRequest {
    let snapshot = snapshot("snapshot:cutover");
    let current = current_binding(&snapshot, "artifact:old");
    let successor = ProposedBinding {
        key: key(),
        revision: BindingRevision::new(NEXT_REVISION),
        target: artifact("artifact:new"),
        contract: contract(),
        basis_snapshot: snapshot.clone(),
    };
    TransitionRequest {
        expected: ExpectedBinding {
            key: key(),
            revision: BindingRevision::new(CURRENT_REVISION),
            target: artifact("artifact:old"),
            snapshot: snapshot.clone(),
        },
        observed_current: current,
        proposed_successor: successor,
        compatibility: Some(CompatibilityObservation {
            observation_id: observation("observation:compatibility"),
            snapshot: snapshot.clone(),
            prior_target: artifact("artifact:old"),
            successor_target: artifact("artifact:new"),
            prior_contract: contract(),
            successor_contract: contract(),
            compatible: true,
        }),
        successor_reachability: Some(TargetReachabilityObservation {
            observation_id: observation("observation:reachability"),
            snapshot,
            target: artifact("artifact:new"),
            reachable: true,
        }),
        rollback_target: None,
        policy: TransitionPolicy {
            compatibility_required: true,
            successor_reachability_required: true,
        },
    }
}

fn admitted_gates() -> ProductGateFacts {
    ProductGateFacts {
        target_loaded: true,
        target_verified: true,
        product_compatible: true,
        migration_required: true,
        migration_satisfied: true,
        authority_admitted: true,
        policy_admitted: true,
        provenance_admitted: true,
        resource_admitted: true,
        lifecycle_admitted: true,
    }
}

fn all_complete() -> Vec<RootClassCompleteness> {
    REQUIRED_ROOT_CLASSES
        .iter()
        .map(|class| RootClassCompleteness {
            class: root_class(class).expect("valid root class fixture"),
            complete: true,
        })
        .collect()
}

fn root(class: &str, id: &str, target: &str) -> GraphRoot {
    GraphRoot {
        class: root_class(class).expect("valid root class fixture"),
        id: RootId::try_new(id, IDENTIFIER_MAXIMUM_BYTES).expect("valid root id fixture"),
        target: artifact(target),
        generation_scope: None,
    }
}

fn inventory(snapshot: &SnapshotId, old: &GenerationId) -> RootInventoryInput {
    RootInventoryInput {
        profile: "sandboxed-component".to_string(),
        snapshot: snapshot.clone(),
        generation: old.clone(),
        instrumented: true,
        roots: Vec::new(),
        edges: Vec::new(),
        class_completeness: all_complete(),
        edge_inventory_complete: true,
        attribution_inventory_complete: true,
        attributions: Vec::new(),
    }
}

#[test]
fn exact_sources_and_all_product_gates_produce_non_publishing_plan() {
    let pins = validate_source_pins(&SourcePinObservation {
        artifact_binding_source: ARTIFACT_BINDING_SOURCE.to_string(),
        artifact_binding_revision: ARTIFACT_BINDING_REVISION.to_string(),
        kamacite_source: KAMACITE_SEMANTIC_SOURCE.to_string(),
        kamacite_revision: KAMACITE_SEMANTIC_REVISION.to_string(),
    });
    assert!(pins.release_eligible);

    let plan = plan_molten_cutover(&MoltenCutoverRequest {
        transition: transition(),
        gates: admitted_gates(),
    })
    .expect("admitted cutover plan");
    assert_eq!(plan.shared_plan.successor.target.as_str(), "artifact:new");
    assert!(!plan.publication_authorized);
    assert!(plan.non_claims.iter().any(|claim| claim.contains("not publication")));
}

#[test]
fn source_drift_stale_cas_and_missing_product_gate_fail_closed() {
    let pins = validate_source_pins(&SourcePinObservation {
        artifact_binding_source: ARTIFACT_BINDING_SOURCE.to_string(),
        artifact_binding_revision: "wrong-revision".to_string(),
        kamacite_source: KAMACITE_SEMANTIC_SOURCE.to_string(),
        kamacite_revision: KAMACITE_SEMANTIC_REVISION.to_string(),
    });
    assert!(!pins.release_eligible);

    let mut stale = transition();
    stale.expected.revision = BindingRevision::new(STALE_REVISION);
    let error = plan_transition(&stale).expect_err("stale compare-and-swap denied");
    assert!(matches!(error, TransitionError::StaleRevision { .. }));
    assert_eq!(diagnose_transition(&error), vec![DeployDiagnostic::StaleCompareAndSwap]);

    let mut gates = admitted_gates();
    gates.authority_admitted = false;
    assert_eq!(
        plan_molten_cutover(&MoltenCutoverRequest {
            transition: transition(),
            gates,
        }),
        Err(LiveBindingError::ProductGateDenied {
            gate: "authority-admitted"
        })
    );
}

#[test]
fn each_unit_resolves_once_and_old_work_stays_pinned() {
    let old_snapshot = snapshot("snapshot:old");
    let new_snapshot = snapshot("snapshot:new");
    let old = resolve_system_extension_callback(&SystemExtensionCallbackInput {
        profile: SYSTEM_EXTENSION_LATE_BINDING_PILOT_PROFILE.to_string(),
        resolution: UnitResolutionInput {
            boundary: UnitBoundary::CallbackPass,
            request: ResolutionRequest::LateBound(LateBoundRequest {
                key: key(),
                expected_snapshot: old_snapshot.clone(),
                expected_revision: Some(BindingRevision::new(CURRENT_REVISION)),
            }),
            snapshot: Some(BindingSnapshot {
                id: old_snapshot.clone(),
                bindings: vec![current_binding(&old_snapshot, "artifact:old")],
            }),
            closures: vec![ArtifactDependencyClosure {
                artifact: artifact("artifact:old"),
                dependencies: vec![artifact("artifact:shared"), artifact("artifact:old-dependency")],
            }],
            nested_lookup: false,
            nested_late_binding_declared: false,
        },
    })
    .expect("old system-extension callback resolution");

    let mut new_binding = current_binding(&new_snapshot, "artifact:new");
    new_binding.revision = BindingRevision::new(NEXT_REVISION);
    let new = resolve_unit_once(&UnitResolutionInput {
        boundary: UnitBoundary::CallbackPass,
        request: ResolutionRequest::LateBound(LateBoundRequest {
            key: key(),
            expected_snapshot: new_snapshot.clone(),
            expected_revision: Some(BindingRevision::new(NEXT_REVISION)),
        }),
        snapshot: Some(BindingSnapshot {
            id: new_snapshot.clone(),
            bindings: vec![new_binding],
        }),
        closures: vec![ArtifactDependencyClosure {
            artifact: artifact("artifact:new"),
            dependencies: vec![artifact("artifact:new-dependency"), artifact("artifact:shared")],
        }],
        nested_lookup: false,
        nested_late_binding_declared: false,
    })
    .expect("new unit resolution");

    assert_eq!(old.shared_resolution.target.as_str(), "artifact:old");
    assert_eq!(new.shared_resolution.target.as_str(), "artifact:new");
    assert_eq!(old.shared_resolution.snapshot, old_snapshot);
    assert_eq!(new.shared_resolution.snapshot, new_snapshot);
}

#[test]
fn implicit_nested_lookup_and_missing_closure_are_denied() {
    let snapshot = snapshot("snapshot:nested");
    let base = UnitResolutionInput {
        boundary: UnitBoundary::Request,
        request: ResolutionRequest::LateBound(LateBoundRequest {
            key: key(),
            expected_snapshot: snapshot.clone(),
            expected_revision: None,
        }),
        snapshot: Some(BindingSnapshot {
            id: snapshot.clone(),
            bindings: vec![current_binding(&snapshot, "artifact:old")],
        }),
        closures: vec![ArtifactDependencyClosure {
            artifact: artifact("artifact:old"),
            dependencies: Vec::new(),
        }],
        nested_lookup: true,
        nested_late_binding_declared: false,
    };
    assert_eq!(resolve_unit_once(&base), Err(LiveBindingError::ImplicitNestedLateBinding));
    assert_eq!(
        resolve_system_extension_callback(&SystemExtensionCallbackInput {
            profile: "native-uninstrumented".to_string(),
            resolution: base.clone(),
        }),
        Err(LiveBindingError::UnsupportedLateBindingProfile)
    );

    let mut missing = base;
    missing.nested_lookup = false;
    missing.closures.clear();
    assert_eq!(resolve_unit_once(&missing), Err(LiveBindingError::MissingDependencyClosure));
}

#[test]
fn complete_empty_inventory_retires_but_never_authorizes_deletion() {
    let snapshot = snapshot("snapshot:retired");
    let old = generation("generation:old");
    let report = classify_generation(&inventory(&snapshot, &old)).expect("complete retirement report");
    assert!(retirement_is_complete(&report));
    assert_eq!(report.decision.classification, RetirementClassification::Retired);
    assert!(report.observation_only);
    assert!(!report.retention_authorized);
    assert!(!report.deletion_authorized);
}

#[test]
fn incomplete_and_uninstrumented_inventories_never_retire() {
    let snapshot = snapshot("snapshot:incomplete");
    let old = generation("generation:old");
    let mut incomplete = inventory(&snapshot, &old);
    incomplete.class_completeness[0].complete = false;
    let report = classify_generation(&incomplete).expect("incomplete report");
    assert_eq!(report.decision.classification, RetirementClassification::Incomplete);
    assert!(
        diagnose_retirement(&report)
            .iter()
            .any(|diagnostic| { matches!(diagnostic, DeployDiagnostic::IncompleteRootInventory { .. }) })
    );

    let mut uninstrumented = inventory(&snapshot, &old);
    uninstrumented.instrumented = false;
    let report = classify_generation(&uninstrumented).expect("uninstrumented report");
    assert_eq!(report.decision.classification, RetirementClassification::Incomplete);
}

#[test]
fn cycles_duplicates_and_exclusive_attribution_produce_stable_live_pin_paths() {
    let snapshot = snapshot("snapshot:live");
    let old = generation("generation:old");
    let new = generation("generation:new");
    let mut input = inventory(&snapshot, &old);
    input.roots = vec![
        root(ROOT_CLASS_SESSION, "session:one", "artifact:a"),
        root(ROOT_CLASS_SESSION, "session:one", "artifact:a"),
    ];
    input.edges = vec![
        GraphEdge {
            from: artifact("artifact:a"),
            to: artifact("artifact:b"),
        },
        GraphEdge {
            from: artifact("artifact:a"),
            to: artifact("artifact:b"),
        },
        GraphEdge {
            from: artifact("artifact:b"),
            to: artifact("artifact:a"),
        },
    ];
    input.attributions = vec![
        ArtifactAttribution {
            snapshot: snapshot.clone(),
            artifact: artifact("artifact:a"),
            ownership: GenerationAttribution::Shared(vec![old.clone(), new]),
        },
        ArtifactAttribution {
            snapshot,
            artifact: artifact("artifact:b"),
            ownership: GenerationAttribution::Exclusive(old),
        },
    ];
    let report = classify_generation(&input).expect("live report");
    assert_eq!(report.decision.classification, RetirementClassification::Live);
    assert_eq!(report.decision.pin_paths.len(), 1);
    assert_eq!(report.decision.pin_paths[0].artifacts.iter().map(ArtifactId::as_str).collect::<Vec<_>>(), vec![
        "artifact:a",
        "artifact:b"
    ]);
    assert!(diagnose_retirement(&report).iter().any(|diagnostic| {
        matches!(diagnostic, DeployDiagnostic::LivePinPath { target, .. } if target == "artifact:b")
    }));
}

#[test]
fn shared_artifacts_can_retire_and_malformed_sharing_is_unknown() {
    let snapshot = snapshot("snapshot:shared");
    let old = generation("generation:old");
    let new = generation("generation:new");
    let mut shared = inventory(&snapshot, &old);
    shared.roots = vec![root(
        ROOT_CLASS_ROLLBACK_RETENTION,
        "retention:shared",
        "artifact:shared",
    )];
    shared.attributions = vec![ArtifactAttribution {
        snapshot: snapshot.clone(),
        artifact: artifact("artifact:shared"),
        ownership: GenerationAttribution::Shared(vec![old.clone(), new]),
    }];
    let report = classify_generation(&shared).expect("shared report");
    assert_eq!(report.decision.classification, RetirementClassification::Retired);

    shared.attributions[0].ownership = GenerationAttribution::Shared(vec![old.clone(), old]);
    let report = classify_generation(&shared).expect("malformed sharing report");
    assert_eq!(report.decision.classification, RetirementClassification::Unknown);
    assert!(diagnose_retirement(&report).contains(&DeployDiagnostic::AmbiguousAttribution));
}
