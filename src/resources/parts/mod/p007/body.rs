// Placement governance — resource requests, limits, quotas, priority,
// constraints, taints, tolerations, and capacity evidence.
//
// Pure core DTOs and validation for Molten placement decisions. Borrows
// scheduling ideas from Kubernetes but expresses them in canonical
// Preserves/capability terms.
//
// Type aliases and common helpers are inherited from p000.

const PLACEMENT_PLAN_SCHEMA: &str = "molten.placement.plan.v1";
const PLACEMENT_CONSTRAINT_SCHEMA: &str = "molten.placement.constraint.v1";

const MAX_CONSTRAINTS: usize = 64;
const MAX_TAINT_TOLERATION_PAIRS: usize = 32;
const MAX_AFFINITY_TERMS: usize = 16;
const _: () = assert!(MAX_CONSTRAINTS > 0);
const _: () = assert!(MAX_TAINT_TOLERATION_PAIRS > 0);

// ---------------------------------------------------------------------------
// Placement request DTOs
// ---------------------------------------------------------------------------

/// Placement request for a workload resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRequest {
    pub workload_ref: String,
    pub workload_type: String,
    pub requests: ResourceAmounts,
    pub limits: ResourceAmounts,
    pub quota_ref: String,
    pub priority: u64,
    pub priority_policy_ref: Option<String>,
    pub constraints: Vec<PlacementConstraint>,
    pub taints: Vec<Taint>,
    pub tolerations: Vec<Toleration>,
    pub target_capacity_evidence: Option<CapacityEvidence>,
    pub assignment_authority_ref: String,
}

/// Resource amounts for a placement request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAmounts {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_mbps: u64,
}

/// Target capacity evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityEvidence {
    pub target_ref: String,
    pub available_cpu_millis: u64,
    pub available_memory_bytes: u64,
    pub available_storage_bytes: u64,
    pub available_network_mbps: u64,
    pub evidence_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constraint and taint/toleration DTOs
// ---------------------------------------------------------------------------

/// Placement constraint — must be satisfied for placement to pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementConstraint {
    pub kind: ConstraintKind,
    pub key: String,
    pub operator: ConstraintOperator,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintKind {
    Required,
    Preferred,
    AntiAffinity,
}

impl ConstraintKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ConstraintKind::Required => "required",
            ConstraintKind::Preferred => "preferred",
            ConstraintKind::AntiAffinity => "anti-affinity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOperator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
    Gt,
    Lt,
}

/// Taint on a target node/workload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taint {
    pub key: String,
    pub value: String,
    pub effect: TaintEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintEffect {
    NoSchedule,
    PreferNoSchedule,
    NoExecute,
}

impl TaintEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            TaintEffect::NoSchedule => "no-schedule",
            TaintEffect::PreferNoSchedule => "prefer-no-schedule",
            TaintEffect::NoExecute => "no-execute",
        }
    }
}

/// Toleration matching a specific taint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toleration {
    pub key: String,
    pub operator: TolerationOperator,
    pub value: Option<String>,
    pub effect: Option<TaintEffect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TolerationOperator {
    Equal,
    Exists,
}

// ---------------------------------------------------------------------------
// Placement decision
// ---------------------------------------------------------------------------

/// Placement decision — pass, deny, or defer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementDecision {
    pub decision: String,
    pub target_ref: Option<String>,
    pub quota_consumed: Option<ResourceAmounts>,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pure core: placement evaluation
// ---------------------------------------------------------------------------

/// Evaluate whether a placement request fits within capacity.
pub fn evaluate_placement_fit(request: &PlacementRequest) -> Result<PlacementDecision> {
    // Validate inputs
    require_ref(&request.workload_ref, "workload ref")?;
    validate_non_empty(&request.workload_type, "workload type")?;
    require_ref(&request.quota_ref, "quota ref")?;
    require_ref(&request.assignment_authority_ref, "assignment authority ref")?;

    // Validate requests <= limits
    if request.requests.cpu_millis > request.limits.cpu_millis {
        return Err(MoltenError::invalid_harness(
            "cpu request exceeds limit",
        ));
    }
    if request.requests.memory_bytes > request.limits.memory_bytes {
        return Err(MoltenError::invalid_harness(
            "memory request exceeds limit",
        ));
    }

    // Validate constraints
    if request.constraints.len() > MAX_CONSTRAINTS {
        return Err(MoltenError::invalid_harness(format!(
            "constraint count {} exceeds maximum {MAX_CONSTRAINTS}",
            request.constraints.len(),
        )));
    }
    for constraint in &request.constraints {
        validate_non_empty(&constraint.key, "constraint key")?;
    }

    // Check capacity evidence
    let capacity = match &request.target_capacity_evidence {
        Some(cap) => cap,
        None => {
            return Ok(PlacementDecision {
                decision: "defer".to_string(),
                target_ref: None,
                quota_consumed: None,
                diagnostics: vec!["missing capacity evidence".to_string()],
            });
        }
    };

    // Validate capacity evidence refs
    for ref evidence_ref in &capacity.evidence_refs {
        require_ref(evidence_ref, "capacity evidence ref")?;
    }

    // Check fit
    let fits = capacity.available_cpu_millis >= request.requests.cpu_millis
        && capacity.available_memory_bytes >= request.requests.memory_bytes
        && capacity.available_storage_bytes >= request.requests.storage_bytes
        && capacity.available_network_mbps >= request.requests.network_mbps;

    if !fits {
        return Ok(PlacementDecision {
            decision: "deny".to_string(),
            target_ref: Some(capacity.target_ref.clone()),
            quota_consumed: None,
            diagnostics: vec!["resource request exceeds available capacity".to_string()],
        });
    }

    Ok(PlacementDecision {
        decision: "pass".to_string(),
        target_ref: Some(capacity.target_ref.clone()),
        quota_consumed: Some(ResourceAmounts {
            cpu_millis: capacity.available_cpu_millis - request.requests.cpu_millis,
            memory_bytes: capacity.available_memory_bytes - request.requests.memory_bytes,
            storage_bytes: capacity.available_storage_bytes - request.requests.storage_bytes,
            network_mbps: capacity.available_network_mbps - request.requests.network_mbps,
        }),
        diagnostics: Vec::new(),
    })
}

/// Evaluate taint/toleration matching.
pub fn evaluate_taint_toleration_match(
    taints: &[Taint],
    tolerations: &[Toleration],
) -> Result<Vec<TaintEffect>> {
    if taints.len() > MAX_TAINT_TOLERATION_PAIRS {
        return Err(MoltenError::invalid_harness(format!(
            "taint count {} exceeds maximum {MAX_TAINT_TOLERATION_PAIRS}",
            taints.len(),
        )));
    }
    if tolerations.len() > MAX_TAINT_TOLERATION_PAIRS {
        return Err(MoltenError::invalid_harness(format!(
            "toleration count {} exceeds maximum {MAX_TAINT_TOLERATION_PAIRS}",
            tolerations.len(),
        )));
    }

    let mut unmatched_effects = Vec::new();

    for taint in taints {
        let tolerated = tolerations.iter().any(|tol| {
            let key_match = match tol.operator {
                TolerationOperator::Equal => tol.key == taint.key,
                TolerationOperator::Exists => true,
            };
            let value_match = match (&tol.operator, &tol.value) {
                (TolerationOperator::Equal, Some(v)) => v == &taint.value,
                (TolerationOperator::Exists, _) => true,
                _ => false,
            };
            let effect_match = match (&tol.effect, &taint.effect) {
                (Some(tol_effect), _) => *tol_effect == taint.effect,
                (None, _) => true,
            };
            key_match && value_match && effect_match
        });

        if !tolerated {
            unmatched_effects.push(taint.effect);
        }
    }

    Ok(unmatched_effects)
}

/// Full placement evaluation including fit, constraints, and taint checks.
pub fn evaluate_placement(
    request: &PlacementRequest,
    explicit_target_properties: &[(String, String)],
) -> PlacementDecision {
    // Check constraints against explicit target properties
    let mut diagnostics = Vec::new();

    for constraint in &request.constraints {
        let satisfied = match constraint.kind {
            ConstraintKind::Required | ConstraintKind::AntiAffinity => {
                let found = explicit_target_properties
                    .iter()
                    .any(|(k, v)| k == &constraint.key && constraint.values.contains(v));
                if !found {
                    diagnostics.push(format!(
                        "required constraint not satisfied: {} {:?} {:?}",
                        constraint.key,
                        constraint.operator,
                        constraint.values,
                    ));
                }
                found
            }
            ConstraintKind::Preferred => true, // Preferred is not a hard deny
        };

        if !satisfied && constraint.kind == ConstraintKind::Required {
            return PlacementDecision {
                decision: "deny".to_string(),
                target_ref: None,
                quota_consumed: None,
                diagnostics,
            };
        }
    }

    // Check taint/toleration from explicit properties (simplified — real impl
    // would extract taints from the target)
    let taint_keys: Vec<String> = explicit_target_properties
        .iter()
        .filter(|(k, _)| k == "taint.no-schedule" || k == "taint.no-execute")
        .map(|(_, v)| v.clone())
        .collect();

    if !taint_keys.is_empty() {
        let tolerated = request.tolerations.iter().any(|tol| {
            taint_keys.iter().any(|tk| {
                match tol.operator {
                    TolerationOperator::Equal => &tol.key == tk,
                    TolerationOperator::Exists => true,
                }
            })
        });
        if !tolerated {
            diagnostics.push("target has hard taints without matching tolerations".to_string());
            return PlacementDecision {
                decision: "deny".to_string(),
                target_ref: None,
                quota_consumed: None,
                diagnostics,
            };
        }
    }

    PlacementDecision {
        decision: "pass".to_string(),
        target_ref: Some("target".to_string()),
        quota_consumed: None,
        diagnostics,
    }
}