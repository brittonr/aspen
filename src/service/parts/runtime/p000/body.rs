type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const SERVICE_READINESS_ASSERTION_SCHEMA: &str = crate::preserves_rail::SERVICE_READINESS_ASSERTION_SCHEMA;
const SERVICE_REPLAY_IDENTITY_SCHEMA: &str = crate::preserves_rail::SERVICE_REPLAY_IDENTITY_SCHEMA;
const RUNTIME_REPORT_SCHEMA: &str = crate::preserves_rail::SERVICE_RUNTIME_REPORT_SCHEMA;
const RUNTIME_SUITE_SCHEMA: &str = crate::preserves_rail::SERVICE_RUNTIME_SUITE_SCHEMA;
const SERVICE_TURN_CONTEXT_SCHEMA: &str = crate::preserves_rail::SERVICE_TURN_CONTEXT_SCHEMA;

fn canonical_hash(value: &preserves::IOValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value)
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const MAX_RUNTIME_ITEMS: usize = 4096;
const MAX_RUNTIME_CHECKS: usize = 256;
const MAX_DEPENDENCY_PASSES: usize = 4096;

const _: () = assert!(MAX_RUNTIME_ITEMS <= 100_000);
const _: () = assert!(MAX_RUNTIME_CHECKS <= 10_000);
const _: () = assert!(MAX_DEPENDENCY_PASSES <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceInput {
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub scheduler_ref: Option<String>,
    pub effect_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteInput {
    pub manifests: Vec<preserves::IOValue>,
    pub demands: Vec<preserves::IOValue>,
    pub statuses: Vec<preserves::IOValue>,
    pub evidence: EvidenceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub suite_ref: String,
    pub manifests: Vec<crate::service_records::ServiceManifest>,
    pub demands: Vec<crate::service_records::ServiceDemand>,
    pub statuses: Vec<crate::service_records::ServiceStatus>,
    pub evidence: EvidenceInput,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub suite_ref: String,
    pub suite_value: preserves::IOValue,
    pub report_ref: String,
    pub lifecycle_receipts: Vec<preserves::IOValue>,
    pub statuses: Vec<preserves::IOValue>,
    pub readiness_assertions: Vec<preserves::IOValue>,
    pub replay_identities: Vec<preserves::IOValue>,
    pub turn_contexts: Vec<preserves::IOValue>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub expected_report_ref: String,
    pub actual_report_ref: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandOutcome {
    lifecycle_receipt: preserves::IOValue,
    status: Option<preserves::IOValue>,
    readiness: Option<preserves::IOValue>,
    replay_identity: Option<preserves::IOValue>,
    turn_context: Option<preserves::IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundedValues {
    label: &'static str,
    values: Vec<preserves::IOValue>,
}

impl BoundedValues {
    fn empty(label: &'static str) -> Self {
        Self {
            label,
            values: Vec::new(),
        }
    }

    fn from_values(label: &'static str, values: Vec<preserves::IOValue>) -> Self {
        Self { label, values }
    }

    fn push(&mut self, value: preserves::IOValue) -> Result<()> {
        let total = self
            .values
            .len()
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness(format!("{} count overflow", self.label)))?;
        ensure_count_at_most(total, self.label)?;
        self.values.push(value);
        Ok(())
    }

    fn as_slice(&self) -> &[preserves::IOValue] {
        &self.values
    }

    fn into_values(self) -> Vec<preserves::IOValue> {
        self.values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Artifacts {
    lifecycle_receipts: BoundedValues,
    statuses: BoundedValues,
    readiness_assertions: BoundedValues,
    replay_identities: BoundedValues,
    turn_contexts: BoundedValues,
}

impl Artifacts {
    fn new(statuses: Vec<preserves::IOValue>) -> Self {
        Self {
            lifecycle_receipts: BoundedValues::empty("service lifecycle receipts"),
            statuses: BoundedValues::from_values("service statuses", statuses),
            readiness_assertions: BoundedValues::empty("service readiness assertions"),
            replay_identities: BoundedValues::empty("service replay identities"),
            turn_contexts: BoundedValues::empty("service turn contexts"),
        }
    }

    fn push_outcome(&mut self, outcome: DemandOutcome) -> Result<()> {
        self.lifecycle_receipts.push(outcome.lifecycle_receipt)?;
        if let Some(status) = outcome.status {
            self.statuses.push(status)?;
        }
        if let Some(readiness) = outcome.readiness {
            self.readiness_assertions.push(readiness)?;
        }
        if let Some(replay_identity) = outcome.replay_identity {
            self.replay_identities.push(replay_identity)?;
        }
        if let Some(turn_context) = outcome.turn_context {
            self.turn_contexts.push(turn_context)?;
        }
        Ok(())
    }
}

struct RunCtx<'a> {
    evidence: &'a EvidenceInput,
    manifests: &'a OrderedMap<String, crate::service_records::ServiceManifest>,
    ready_statuses: OrderedMap<String, String>,
    artifacts: Artifacts,
    runtime: crate::runtime::RuntimeState,
}

struct PassResult {
    pending: Vec<crate::service_records::ServiceDemand>,
    is_progress_made: bool,
}

enum StepOutcome {
    Started,
    Finished,
    Pending(crate::service_records::ServiceDemand),
}
