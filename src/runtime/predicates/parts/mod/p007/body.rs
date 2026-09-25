
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
