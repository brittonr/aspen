use std::collections::BTreeSet;

use super::*;
use crate::fabric::FabricPortClass;
use crate::fabric::ReferenceSystemKind;

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_BYTES: usize = 64;
const MINIMUM_POSITIVE_BOUND: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldIssue {
    SchemaMismatch,
    MalformedRef {
        field: &'static str,
        value: String,
    },
    MalformedIdentifier {
        field: &'static str,
        value: String,
    },
    EmptyCollection(&'static str),
    TooManyItems {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateNode(String),
    DuplicatePortClass(FabricPortClass),
    DuplicateWorkloadSequence(u64),
    DuplicateFault(String),
    DuplicateInvariant,
    ZeroGeneration(String),
    SameCoreMismatch {
        node_id: String,
        field: &'static str,
    },
    MissingPortClass(FabricPortClass),
    NonDeterministicPort(FabricPortClass),
    NodeRequiresMissingPort {
        node_id: String,
        class: FabricPortClass,
    },
    WorkloadNodeUnknown(String),
    WorkloadServiceMismatch {
        node_id: String,
        expected: ReferenceSystemKind,
        actual: ReferenceSystemKind,
    },
    WorkloadSequenceGap {
        expected: u64,
        actual: u64,
    },
    FaultTargetUnknown(String),
    FaultBoundaryMissing(FabricPortClass),
    FaultNotDeclared {
        class: FabricPortClass,
        kind: SimulationFaultKind,
    },
    DirectExtensionStateMutation(String),
    ZeroFaultResourceCost(String),
    MissingUniversalInvariant(UniversalInvariantKind),
    MissingServiceInvariant(ReferenceSystemKind),
    Unbounded(&'static str),
    StructuralBoundExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    ClaimOverreach(SimulationClaimProfile),
    MissingNonClaim(SimulationNonClaim),
    DuplicateNonClaim,
    AmbientInput(String),
}

pub fn admit_simulated_world(manifest: &SimulatedWorldManifest) -> Result<AdmittedSimulatedWorld, Vec<WorldIssue>> {
    let mut issues = Vec::new();
    validate_world_identity(manifest, &mut issues);
    validate_bounds(manifest, &mut issues);
    validate_nodes(manifest, &mut issues);
    validate_ports(manifest, &mut issues);
    validate_workload(manifest, &mut issues);
    validate_faults(manifest, &mut issues);
    validate_invariants(manifest, &mut issues);
    validate_claims(manifest, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut normalized = manifest.clone();
    normalized.nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    normalized.port_profiles.sort_by_key(|profile| profile.class);
    normalized.workload.sort_by_key(|step| step.sequence);
    normalized.faults.sort_by(|left, right| left.fault_id.cmp(&right.fault_id));
    normalized.invariants.sort_by_key(invariant_order);
    normalized.non_claims.sort();
    Ok(AdmittedSimulatedWorld { manifest: normalized })
}

fn validate_world_identity(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.schema != FABRIC_SIMULATION_WORLD_SCHEMA {
        issues.push(WorldIssue::SchemaMismatch);
    }
    for (field, value) in [
        ("runtime-ref", manifest.runtime_ref.as_str()),
        ("scheduler-input-ref", manifest.scheduler_input_ref.as_str()),
        ("entropy-input-ref", manifest.entropy_input_ref.as_str()),
        ("authority-ref", manifest.authority_ref.as_str()),
        ("policy-ref", manifest.policy_ref.as_str()),
        ("initial-durable-state-ref", manifest.initial_durable_state_ref.as_str()),
        ("resource-profile-ref", manifest.resource_profile_ref.as_str()),
        ("workload-ref", manifest.workload_ref.as_str()),
        ("fault-plan-ref", manifest.fault_plan_ref.as_str()),
        ("invariant-set-ref", manifest.invariant_set_ref.as_str()),
    ] {
        validate_ref(field, value, issues);
    }
    if !manifest.ambient_inputs.is_empty() {
        for ambient in &manifest.ambient_inputs {
            issues.push(WorldIssue::AmbientInput(ambient.clone()));
        }
    }
}

fn validate_bounds(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    for (field, value) in [
        ("max-choices", manifest.bounds.max_choices),
        ("max-events", manifest.bounds.max_events),
        ("max-virtual-ticks", manifest.bounds.max_virtual_ticks),
        ("max-trace-bytes", manifest.bounds.max_trace_bytes),
        ("max-resource-units", manifest.bounds.max_resource_units),
        ("max-shrink-attempts", manifest.bounds.max_shrink_attempts),
    ] {
        if value < MINIMUM_POSITIVE_BOUND {
            issues.push(WorldIssue::Unbounded(field));
        }
    }
    if manifest.bounds.max_shrink_attempts > MAX_SHRINK_ATTEMPTS {
        issues.push(WorldIssue::StructuralBoundExceeded {
            field: "max-shrink-attempts",
            actual: manifest.bounds.max_shrink_attempts,
            maximum: MAX_SHRINK_ATTEMPTS,
        });
    }
    check_count("nodes", manifest.nodes.len(), MAX_WORLD_NODES, issues);
    check_count("port-profiles", manifest.port_profiles.len(), MAX_WORLD_PORT_PROFILES, issues);
    check_count("workload", manifest.workload.len(), MAX_WORLD_WORKLOAD_STEPS, issues);
    check_count("faults", manifest.faults.len(), MAX_WORLD_FAULTS, issues);
    check_count("invariants", manifest.invariants.len(), MAX_WORLD_INVARIANTS, issues);
    check_count("non-claims", manifest.non_claims.len(), MAX_WORLD_NON_CLAIMS, issues);
    match u64::try_from(manifest.workload.len()) {
        Ok(actual) if actual > manifest.bounds.max_events => issues.push(WorldIssue::StructuralBoundExceeded {
            field: "workload-events",
            actual,
            maximum: manifest.bounds.max_events,
        }),
        Ok(_) => {}
        Err(_) => issues.push(WorldIssue::StructuralBoundExceeded {
            field: "workload-events",
            actual: u64::MAX,
            maximum: manifest.bounds.max_events,
        }),
    }
    match u64::try_from(manifest.faults.len()) {
        Ok(actual) if actual > manifest.bounds.max_choices => issues.push(WorldIssue::StructuralBoundExceeded {
            field: "fault-choice-count",
            actual,
            maximum: manifest.bounds.max_choices,
        }),
        Ok(_) => {}
        Err(_) => issues.push(WorldIssue::StructuralBoundExceeded {
            field: "fault-choice-count",
            actual: u64::MAX,
            maximum: manifest.bounds.max_choices,
        }),
    }
}

fn validate_nodes(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.nodes.is_empty() {
        issues.push(WorldIssue::EmptyCollection("nodes"));
    }
    let mut node_ids = BTreeSet::new();
    for node in &manifest.nodes {
        for (field, value) in [
            ("node-id", node.node_id.as_str()),
            ("extension-id", node.extension_id.as_str()),
            ("service-id", node.service_id.as_str()),
        ] {
            validate_identifier(field, value, issues);
        }
        for (field, value) in [
            ("initial-state-ref", node.initial_state_ref.as_str()),
            ("membership-view-ref", node.membership_view_ref.as_str()),
            ("placement-ref", node.placement_ref.as_str()),
            ("consistency-profile-ref", node.consistency_profile_ref.as_str()),
        ] {
            validate_ref(field, value, issues);
        }
        validate_core_identity(&node.same_core.simulation, issues);
        validate_core_identity(&node.same_core.live, issues);
        validate_same_core(node, issues);
        if node.generation == 0 {
            issues.push(WorldIssue::ZeroGeneration(node.node_id.clone()));
        }
        if !node_ids.insert(node.node_id.clone()) {
            issues.push(WorldIssue::DuplicateNode(node.node_id.clone()));
        }
        if node.required_port_classes.is_empty() {
            issues.push(WorldIssue::EmptyCollection("node-required-port-classes"));
        }
        let mut classes = node.required_port_classes.clone();
        classes.sort();
        if classes.windows(2).any(|window| window[0] == window[1]) {
            issues.push(WorldIssue::DuplicatePortClass(classes[0]));
        }
    }
}

fn validate_ports(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.port_profiles.is_empty() {
        issues.push(WorldIssue::EmptyCollection("port-profiles"));
    }
    let mut classes = BTreeSet::new();
    for profile in &manifest.port_profiles {
        for (field, value) in [
            ("port-id", profile.port_id.as_str()),
            ("port-version", profile.version.as_str()),
            ("implementation-profile", profile.implementation_profile.as_str()),
            ("command-schema-ref", profile.command_schema_ref.as_str()),
            ("event-schema-ref", profile.event_schema_ref.as_str()),
        ] {
            validate_identifier(field, value, issues);
        }
        validate_ref("port-descriptor-ref", &profile.descriptor_ref, issues);
        if !classes.insert(profile.class) {
            issues.push(WorldIssue::DuplicatePortClass(profile.class));
        }
        if !profile.deterministic {
            issues.push(WorldIssue::NonDeterministicPort(profile.class));
        }
    }
    for required in REQUIRED_SIMULATION_PORT_CLASSES {
        if !classes.contains(&required) {
            issues.push(WorldIssue::MissingPortClass(required));
        }
    }
    for node in &manifest.nodes {
        for required in &node.required_port_classes {
            if !classes.contains(required) {
                issues.push(WorldIssue::NodeRequiresMissingPort {
                    node_id: node.node_id.clone(),
                    class: *required,
                });
            }
        }
    }
}

fn validate_workload(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.workload.is_empty() {
        issues.push(WorldIssue::EmptyCollection("workload"));
    }
    let mut sequences = BTreeSet::new();
    let mut ordered = manifest.workload.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|step| step.sequence);
    let mut expected = FIRST_WORKLOAD_SEQUENCE;
    for step in ordered {
        if !sequences.insert(step.sequence) {
            issues.push(WorldIssue::DuplicateWorkloadSequence(step.sequence));
        }
        if step.sequence != expected {
            issues.push(WorldIssue::WorkloadSequenceGap {
                expected,
                actual: step.sequence,
            });
        }
        match expected.checked_add(1) {
            Some(next) => expected = next,
            None => issues.push(WorldIssue::StructuralBoundExceeded {
                field: "workload-sequence",
                actual: expected,
                maximum: u64::MAX,
            }),
        }
        validate_ref("workload-request-ref", &step.request_ref, issues);
        validate_identifier("workload-node-id", &step.node_id, issues);
        let matching = manifest.nodes.iter().find(|node| node.node_id == step.node_id);
        let Some(node) = matching else {
            issues.push(WorldIssue::WorkloadNodeUnknown(step.node_id.clone()));
            continue;
        };
        let expected_service = service_kind_from_id(&node.service_id);
        if let Some(expected_service) = expected_service
            && expected_service != step.service
        {
            issues.push(WorldIssue::WorkloadServiceMismatch {
                node_id: node.node_id.clone(),
                expected: expected_service,
                actual: step.service,
            });
        }
    }
}

fn validate_faults(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    let mut fault_ids = BTreeSet::new();
    for fault in &manifest.faults {
        validate_identifier("fault-id", &fault.fault_id, issues);
        validate_identifier("fault-target", &fault.target, issues);
        validate_identifier("fault-observation", &fault.expected_observation, issues);
        if !fault_ids.insert(fault.fault_id.clone()) {
            issues.push(WorldIssue::DuplicateFault(fault.fault_id.clone()));
        }
        if fault.direct_extension_state_mutation {
            issues.push(WorldIssue::DirectExtensionStateMutation(fault.fault_id.clone()));
        }
        if fault.resource_cost == 0 {
            issues.push(WorldIssue::ZeroFaultResourceCost(fault.fault_id.clone()));
        }
        let target_known = manifest.nodes.iter().any(|node| node.node_id == fault.target)
            || manifest.port_profiles.iter().any(|profile| profile.port_id == fault.target);
        if !target_known {
            issues.push(WorldIssue::FaultTargetUnknown(fault.target.clone()));
        }
        let Some(profile) = manifest.port_profiles.iter().find(|profile| profile.class == fault.boundary) else {
            issues.push(WorldIssue::FaultBoundaryMissing(fault.boundary));
            continue;
        };
        if !profile.declared_faults.contains(&fault.kind) {
            issues.push(WorldIssue::FaultNotDeclared {
                class: fault.boundary,
                kind: fault.kind,
            });
        }
    }
}

fn validate_invariants(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.invariants.is_empty() {
        issues.push(WorldIssue::EmptyCollection("invariants"));
    }
    let mut seen = BTreeSet::new();
    for invariant in &manifest.invariants {
        if !seen.insert(invariant_order(invariant)) {
            issues.push(WorldIssue::DuplicateInvariant);
        }
        if let SimulationInvariant::ExtensionSemantic { invariant_id, .. } = invariant {
            validate_identifier("extension-invariant-id", invariant_id, issues);
        }
    }
    for required in REQUIRED_UNIVERSAL_INVARIANTS {
        if !manifest.invariants.contains(&SimulationInvariant::Universal(required)) {
            issues.push(WorldIssue::MissingUniversalInvariant(required));
        }
    }
    for service in [
        ReferenceSystemKind::TransactionalKeyValue,
        ReferenceSystemKind::ReplicatedLog,
        ReferenceSystemKind::DistributedScheduler,
    ] {
        if !manifest
            .invariants
            .iter()
            .any(|invariant| matches!(invariant, SimulationInvariant::ExtensionSemantic { service: actual, .. } if *actual == service))
        {
            issues.push(WorldIssue::MissingServiceInvariant(service));
        }
    }
}

fn validate_claims(manifest: &SimulatedWorldManifest, issues: &mut Vec<WorldIssue>) {
    if manifest.claim_profile > SimulationClaimProfile::DeterministicWholeSystem {
        issues.push(WorldIssue::ClaimOverreach(manifest.claim_profile));
    }
    let mut non_claims = manifest.non_claims.clone();
    non_claims.sort();
    if non_claims.windows(2).any(|window| window[0] == window[1]) {
        issues.push(WorldIssue::DuplicateNonClaim);
    }
    for required in REQUIRED_SIMULATION_NON_CLAIMS {
        if !manifest.non_claims.contains(&required) {
            issues.push(WorldIssue::MissingNonClaim(required));
        }
    }
}

pub fn evaluate_invariants(
    invariants: &[SimulationInvariant],
    observations: &[SimulationObservation],
) -> Vec<InvariantResult> {
    invariants
        .iter()
        .map(|invariant| {
            let failure = observations.iter().find(|observation| !observation_satisfies(invariant, observation));
            InvariantResult {
                invariant: invariant.clone(),
                passed: failure.is_none(),
                first_failure_sequence: failure.map(|observation| observation.sequence),
            }
        })
        .collect()
}

// r[impl molten.fabric_simulation.claim_ladder]
pub fn evaluate_claim_promotion(
    current: SimulationClaimProfile,
    target: SimulationClaimProfile,
    evidence: &ClaimEvidence,
) -> ClaimPromotionDecision {
    let mut missing = Vec::new();
    if target < current || evidence.profile != target {
        missing.push("profile-specific-evidence");
    }
    if !valid_ref(&evidence.implementation_ref) {
        missing.push("implementation-identity");
    }
    if evidence.adapter_refs.is_empty() || evidence.adapter_refs.iter().any(|reference| !valid_ref(reference)) {
        missing.push("adapter-evidence");
    }
    if target >= SimulationClaimProfile::MultiProcessLive {
        require_optional_ref(&evidence.environment_ref, "environment-evidence", &mut missing);
        require_optional_ref(&evidence.lifecycle_ref, "lifecycle-evidence", &mut missing);
        require_optional_ref(&evidence.operator_ref, "operator-evidence", &mut missing);
    }
    if target >= SimulationClaimProfile::HostChaos {
        require_optional_ref(&evidence.fault_ref, "fault-evidence", &mut missing);
    }
    ClaimPromotionDecision {
        admitted: missing.is_empty(),
        target,
        missing_evidence: missing,
    }
}

fn observation_satisfies(invariant: &SimulationInvariant, observation: &SimulationObservation) -> bool {
    match invariant {
        SimulationInvariant::Universal(UniversalInvariantKind::NoAmbientEffect) => !observation.ambient_effect,
        SimulationInvariant::Universal(UniversalInvariantKind::NoStaleGenerationMutation) => {
            !observation.stale_generation_mutation
        }
        SimulationInvariant::Universal(UniversalInvariantKind::NoResourceBoundBypass) => {
            !observation.resource_bound_bypass
        }
        SimulationInvariant::Universal(UniversalInvariantKind::NoPortStateMachineViolation) => {
            !observation.port_state_machine_violation
        }
        SimulationInvariant::Universal(UniversalInvariantKind::ValidCanonicalRefs) => {
            valid_ref(&observation.state_ref)
                && valid_ref(&observation.history_ref)
                && valid_ref(&observation.port_event_ref)
        }
        SimulationInvariant::Universal(UniversalInvariantKind::CompleteTerminalCleanup) => {
            observation.terminal_cleanup_complete
        }
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => {
            observation.service != Some(*service) || observation.semantic_invariants_passed.contains(invariant_id)
        }
    }
}

fn validate_same_core(node: &SimulatedNode, issues: &mut Vec<WorldIssue>) {
    let simulation = &node.same_core.simulation;
    let live = &node.same_core.live;
    for (field, matches) in [
        ("implementation-ref", simulation.implementation_ref == live.implementation_ref),
        ("manifest-ref", simulation.manifest_ref == live.manifest_ref),
        ("callback-dispatcher-ref", simulation.callback_dispatcher_ref == live.callback_dispatcher_ref),
        ("protocol-core-ref", simulation.protocol_core_ref == live.protocol_core_ref),
        ("state-machine-ref", simulation.state_machine_ref == live.state_machine_ref),
        ("schema-set-ref", simulation.schema_set_ref == live.schema_set_ref),
        ("port-contract-set-ref", simulation.port_contract_set_ref == live.port_contract_set_ref),
    ] {
        if !matches {
            issues.push(WorldIssue::SameCoreMismatch {
                node_id: node.node_id.clone(),
                field,
            });
        }
    }
}

fn validate_core_identity(identity: &ExtensionCoreIdentity, issues: &mut Vec<WorldIssue>) {
    for (field, value) in [
        ("implementation-ref", identity.implementation_ref.as_str()),
        ("manifest-ref", identity.manifest_ref.as_str()),
        ("callback-dispatcher-ref", identity.callback_dispatcher_ref.as_str()),
        ("protocol-core-ref", identity.protocol_core_ref.as_str()),
        ("state-machine-ref", identity.state_machine_ref.as_str()),
        ("schema-set-ref", identity.schema_set_ref.as_str()),
        ("port-contract-set-ref", identity.port_contract_set_ref.as_str()),
    ] {
        validate_ref(field, value, issues);
    }
}

fn check_count(field: &'static str, actual: usize, maximum: usize, issues: &mut Vec<WorldIssue>) {
    if actual > maximum {
        issues.push(WorldIssue::TooManyItems { field, actual, maximum });
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<WorldIssue>) {
    if !valid_ref(value) {
        issues.push(WorldIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}

pub(crate) fn valid_ref(value: &str) -> bool {
    value
        .strip_prefix(BLAKE3_PREFIX)
        .is_some_and(|hex| hex.len() == BLAKE3_HEX_BYTES && hex.chars().all(|character| character.is_ascii_hexdigit()))
}

fn validate_identifier(field: &'static str, value: &str, issues: &mut Vec<WorldIssue>) {
    let valid = !value.is_empty()
        && value.len() <= MAX_WORLD_IDENTIFIER_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '-' | '_' | '/'));
    if !valid {
        issues.push(WorldIssue::MalformedIdentifier {
            field,
            value: value.to_string(),
        });
    }
}

fn invariant_order(invariant: &SimulationInvariant) -> (u8, String) {
    match invariant {
        SimulationInvariant::Universal(kind) => (0, kind.as_str().to_string()),
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => {
            (1, format!("{}:{invariant_id}", service.as_str()))
        }
    }
}

fn service_kind_from_id(service_id: &str) -> Option<ReferenceSystemKind> {
    if service_id.contains("transactional-key-value") {
        Some(ReferenceSystemKind::TransactionalKeyValue)
    } else if service_id.contains("replicated-log") {
        Some(ReferenceSystemKind::ReplicatedLog)
    } else if service_id.contains("distributed-scheduler") {
        Some(ReferenceSystemKind::DistributedScheduler)
    } else {
        None
    }
}

fn require_optional_ref(value: &Option<String>, label: &'static str, missing: &mut Vec<&'static str>) {
    if value.as_deref().is_none_or(|reference| !valid_ref(reference)) {
        missing.push(label);
    }
}
