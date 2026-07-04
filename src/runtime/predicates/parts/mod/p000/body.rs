type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type PendingTurn = super::PendingTurn;
type Result<T> = crate::error::Result<T>;
type RuntimeObserver = super::RuntimeObserver;
type RuntimeSnapshot = super::RuntimeSnapshot;
type RuntimeValue = super::RuntimeValue;
type TurnAction = super::TurnAction;

const PREDICATE_ENGINE: &str = "trellis-bounded-local";
const ASSERTION_VISIBILITY_PREDICATE: &str = "molten.trellis-runtime.assertion-visibility.v1";
const TURN_COMMIT_ROLLBACK_PREDICATE: &str = "molten.trellis-runtime.turn-commit-rollback.v1";
const PRESERVES_PATTERN_PREDICATE: &str = "molten.trellis-runtime.preserves-pattern.v1";
const OBSERVE_DELIVERY_PREDICATE: &str = "molten.trellis-runtime.observe-delivery.v1";
const PROMISE_STATE_PREDICATE: &str = "molten.trellis-runtime.promise-state.v1";
const PROMISE_PIPELINE_PREDICATE: &str = "molten.trellis-runtime.promise-pipeline.v1";
const PROMISE_USE_PREDICATE: &str = "molten.trellis-runtime.promise-use.v1";
const REVOCATION_CLEANUP_PREDICATE: &str = "molten.trellis-runtime.revocation-cleanup.v1";
const ACTORMAP_TRANSACTION_PREDICATE: &str = "molten.trellis-runtime.actormap-transaction.v1";
const NEAR_FAR_REFS_PREDICATE: &str = "molten.trellis-runtime.near-far-refs.v1";
const SNAPSHOT_AUTHORITY_PREDICATE: &str = "molten.trellis-runtime.snapshot-authority.v1";
const OBJECT_AUTHORITY_PREDICATE: &str = "molten.trellis-runtime.object-authority.v1";
const RIGHTS_AMPLIFICATION_PREDICATE: &str = "molten.trellis-runtime.rights-amplification.v1";
const DISTRIBUTED_REF_LIFETIME_PREDICATE: &str = "molten.trellis-runtime.distributed-ref-lifetime.v1";
const VAT_ROLLBACK_CLEANUP_PREDICATE: &str = "molten.trellis-runtime.vat-rollback-cleanup.v1";
const SERVICE_DEPENDENCIES_PREDICATE: &str = "molten.trellis-runtime.service-dependencies.v1";
const RUNTIME_PATTERN_EXACT_LABEL: &str = "runtime-pattern-exact-v1";
const RUNTIME_PATTERN_WILDCARD_LABEL: &str = "runtime-pattern-wildcard-v1";
const PATTERN_EXACT_ARITY: usize = 2;
const PATTERN_WILDCARD_ARITY: usize = 1;
const PATTERN_VALUE_INDEX: usize = 0;
const PATTERN_VALUE_REF_INDEX: usize = 1;
const PATTERN_BINDING_INDEX: usize = 0;
const MAX_PATTERN_BINDING_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateDecision {
    Pass,
    Deny,
}

impl PredicateDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    Committed,
    RolledBack,
    Denied,
    Failed,
}

impl TurnOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::RolledBack => "rolled-back",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimePattern {
    Exact(RuntimeValue),
    Wildcard { binding: String },
}

impl RuntimePattern {
    pub fn exact(value: RuntimeValue) -> Self {
        Self::Exact(value)
    }

    pub fn wildcard(binding: impl Into<String>) -> Self {
        Self::Wildcard {
            binding: binding.into(),
        }
    }

    // r[impl molten.preserves_boundary_codegen.pattern_ast]
    pub fn from_ast_value(value: &IoValue) -> Result<Self> {
        if let Some(fields) = value.collect_simple_record(RUNTIME_PATTERN_EXACT_LABEL, Some(PATTERN_EXACT_ARITY)) {
            let pattern_value = crate::preserves_rail::value_to_iovalue(&fields[PATTERN_VALUE_INDEX]);
            let runtime_value = RuntimeValue::new(pattern_value)?;
            let declared_ref = crate::preserves_rail::record_content_ref_string(
                &fields[PATTERN_VALUE_REF_INDEX],
                "value-ref",
                "pattern value ref",
            )?;
            if declared_ref != runtime_value.value_ref() {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "runtime pattern exact value-ref mismatch: expected {declared_ref}, got {}",
                    runtime_value.value_ref()
                )));
            }
            return Ok(Self::Exact(runtime_value));
        }
        if let Some(fields) = value.collect_simple_record(RUNTIME_PATTERN_WILDCARD_LABEL, Some(PATTERN_WILDCARD_ARITY)) {
            let binding = crate::preserves_rail::required_string_field(&fields[PATTERN_BINDING_INDEX], "pattern binding")?;
            validate_pattern_binding(&binding)?;
            return Ok(Self::Wildcard { binding });
        }
        Err(crate::error::MoltenError::invalid_harness(
            "unsupported Preserves routing pattern AST form",
        ))
    }

    pub fn from_observe_value(value: &RuntimeValue) -> Result<Self> {
        let source = value.as_iovalue();
        if source
            .collect_simple_record(RUNTIME_PATTERN_EXACT_LABEL, Some(PATTERN_EXACT_ARITY))
            .is_some()
            || source
                .collect_simple_record(RUNTIME_PATTERN_WILDCARD_LABEL, Some(PATTERN_WILDCARD_ARITY))
                .is_some()
        {
            Self::from_ast_value(source)
        } else {
            Ok(Self::Exact(value.clone()))
        }
    }

    pub fn to_value(&self) -> IoValue {
        match self {
            Self::Exact(value) => crate::preserves_rail::record(RUNTIME_PATTERN_EXACT_LABEL, vec![
                value.as_iovalue().clone(),
                crate::preserves_rail::record("value-ref", vec![crate::preserves_rail::string(value.value_ref())]),
            ]),
            Self::Wildcard { binding } => {
                crate::preserves_rail::record(RUNTIME_PATTERN_WILDCARD_LABEL, vec![crate::preserves_rail::string(
                    binding,
                )])
            }
        }
    }

    pub fn pattern_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Exact(_) => Ok(()),
            Self::Wildcard { binding } => validate_pattern_binding(binding),
        }
    }

    pub fn matches_value(&self, value: &RuntimeValue) -> Result<(bool, Vec<(String, String)>)> {
        match self {
            Self::Exact(expected) => Ok((expected == value, Vec::new())),
            Self::Wildcard { binding } => {
                validate_pattern_binding(binding)?;
                Ok((true, vec![(binding.clone(), value.value_ref().to_string())]))
            }
        }
    }
}

fn validate_pattern_binding(binding: &str) -> Result<()> {
    if binding.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(
            "runtime pattern binding must not be empty",
        ));
    }
    if binding.len() > MAX_PATTERN_BINDING_BYTES {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "runtime pattern binding exceeds {MAX_PATTERN_BINDING_BYTES} bytes"
        )));
    }
    if !binding
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(crate::error::MoltenError::invalid_harness(
            "runtime pattern binding must use ASCII alphanumeric, underscore, or dash bytes",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePredicateReceipt {
    pub receipt_ref: String,
    pub predicate: String,
    pub input_ref: String,
    pub decision: PredicateDecision,
    pub state_refs: Vec<String>,
    pub checks: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionVisibilityResult {
    pub is_visible: bool,
    pub visible_owner_refs: Vec<String>,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatchResult {
    pub is_match: bool,
    pub bindings: Vec<(String, String)>,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserveDeliveryResult {
    pub delivered_assertion_refs: Vec<String>,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePromiseStatus {
    Pending,
    Resolved,
    Broken,
    Cancelled,
    TimedOut,
}

impl RuntimePromiseStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Broken => "broken",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed-out",
        }
    }

    fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromiseState {
    pub promise_id: String,
    pub status: RuntimePromiseStatus,
    pub value_ref: Option<String>,
    pub reason: Option<String>,
    pub caused_by: Vec<String>,
}

impl RuntimePromiseState {
    pub fn pending(promise_id: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Pending,
            value_ref: None,
            reason: None,
            caused_by: Vec::new(),
        }
    }

    pub fn resolved(promise_id: impl Into<String>, value_ref: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Resolved,
            value_ref: Some(value_ref.into()),
            reason: None,
            caused_by: Vec::new(),
        }
    }

    pub fn broken(promise_id: impl Into<String>, reason: impl Into<String>, caused_by: Vec<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Broken,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by,
        }
    }

    pub fn cancelled(promise_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::Cancelled,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by: Vec::new(),
        }
    }

    pub fn timed_out(promise_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            promise_id: promise_id.into(),
            status: RuntimePromiseStatus::TimedOut,
            value_ref: None,
            reason: Some(reason.into()),
            caused_by: Vec::new(),
        }
    }

    pub fn promise_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-promise-state-v1", vec![
            crate::preserves_rail::string(&self.promise_id),
            crate::preserves_rail::string(self.status.as_str()),
            optional_string_value(self.value_ref.as_deref()),
            optional_string_value(self.reason.as_deref()),
            crate::preserves_rail::sequence(self.caused_by.iter().map(crate::preserves_rail::string).collect()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseStateResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromisePipelineEntry {
    pub sequence: u64,
    pub target_ref: String,
    pub operation: String,
}

impl RuntimePromisePipelineEntry {
    pub fn new(sequence: u64, target_ref: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            sequence,
            target_ref: target_ref.into(),
            operation: operation.into(),
        }
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-promise-pipeline-entry-v1", vec![
            crate::preserves_rail::u64_value(self.sequence),
            crate::preserves_rail::record("target-ref", vec![crate::preserves_rail::string(&self.target_ref)]),
            crate::preserves_rail::string(&self.operation),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromisePipelineState {
    pub source: RuntimePromiseState,
    pub max_queue: u64,
    pub entries: Vec<RuntimePromisePipelineEntry>,
}

impl RuntimePromisePipelineState {
    pub fn new(source: RuntimePromiseState, max_queue: u64, entries: Vec<RuntimePromisePipelineEntry>) -> Self {
        Self {
            source,
            max_queue,
            entries,
        }
    }

    pub fn pipeline_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-promise-pipeline-state-v1", vec![
            self.source.to_value(),
            crate::preserves_rail::u64_value(self.max_queue),
            crate::preserves_rail::sequence(self.entries.iter().map(RuntimePromisePipelineEntry::to_value).collect()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromisePipelineResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePromiseUseKind {
    ResolvedValue,
    PipelineForward,
}

impl RuntimePromiseUseKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResolvedValue => "resolved-value",
            Self::PipelineForward => "pipeline-forward",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePromiseUseState {
    pub source: RuntimePromiseState,
    pub use_kind: RuntimePromiseUseKind,
    pub dependent_call_ref: String,
    pub admitted_resolution_ref: Option<String>,
    pub admitted_pipeline_ref: Option<String>,
}

impl RuntimePromiseUseState {
    pub fn use_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-promise-use-state-v1", vec![
            self.source.to_value(),
            crate::preserves_rail::string(self.use_kind.as_str()),
            crate::preserves_rail::record("dependent-call-ref", vec![crate::preserves_rail::string(
                &self.dependent_call_ref,
            )]),
            optional_ref_record("admitted-resolution-ref", self.admitted_resolution_ref.as_deref()),
            optional_ref_record("admitted-pipeline-ref", self.admitted_pipeline_ref.as_deref()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromiseUseResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRevocationCleanupState {
    pub revoked_refs: Vec<String>,
    pub attempted_use_refs: Vec<String>,
    pub remaining_assertion_refs: Vec<String>,
    pub remaining_subscription_refs: Vec<String>,
    pub remaining_pending_call_refs: Vec<String>,
    pub remaining_child_refs: Vec<String>,
}
