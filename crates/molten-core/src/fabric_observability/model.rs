pub const OBSERVATION_PROFILE_SCHEMA: &str = "molten.fabric-observability.profile.v1";
pub const METRIC_DESCRIPTOR_SCHEMA: &str = "molten.fabric-observability.metric-descriptor.v1";
pub const METRIC_SAMPLE_SCHEMA: &str = "molten.fabric-observability.metric-sample.v1";
pub const OBSERVATION_EVENT_SCHEMA: &str = "molten.fabric-observability.event.v1";
pub const HEALTH_INPUT_SCHEMA: &str = "molten.fabric-observability.health-input.v1";
pub const HEALTH_DECISION_SCHEMA: &str = "molten.fabric-observability.health-decision.v1";
pub const READINESS_POLICY_SCHEMA: &str = "molten.fabric-observability.readiness-policy.v1";
pub const INTEGRITY_PLAN_SCHEMA: &str = "molten.fabric-observability.integrity-plan.v1";
pub const SCAN_OBSERVATION_SCHEMA: &str = "molten.fabric-observability.scan-observation.v1";
pub const INTEGRITY_FINDING_SCHEMA: &str = "molten.fabric-observability.integrity-finding.v1";
pub const INTEGRITY_RESULT_SCHEMA: &str = "molten.fabric-observability.integrity-result.v1";
pub const OBSERVATION_ADAPTER_PROFILE_SCHEMA: &str = "molten.fabric-observability.adapter-profile.v1";
pub const OBSERVATION_ADAPTER_OUTCOME_SCHEMA: &str = "molten.fabric-observability.adapter-outcome.v1";
pub const OBSERVATION_ADAPTER_STATUS_SCHEMA: &str = "molten.fabric-observability.adapter-status.v1";
pub const OBSERVATION_SNAPSHOT_SCHEMA: &str = "molten.fabric-observability.snapshot.v1";
pub const INTEGRITY_MUTATION_AUTHORITY_SCHEMA: &str = "molten.fabric-observability.mutation-authority.v1";
pub const MAX_OBSERVATION_TEXT_BYTES: usize = 256;
pub const MAX_OBSERVATION_REFS: usize = 256;
pub(crate) const ADJACENT_PAIR_WIDTH: usize = 2;

const REQUIRED_NON_CLAIM_COUNT: usize = 7;
const HEALTHY_SEVERITY: u8 = 0;
const DEGRADED_SEVERITY: u8 = 1;
const UNAVAILABLE_SEVERITY: u8 = 2;
const FAILED_SEVERITY: u8 = 3;

pub const REQUIRED_OBSERVABILITY_NON_CLAIMS: [ObservabilityNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    ObservabilityNonClaim::NoCapabilityAuthority,
    ObservabilityNonClaim::NoRepairAuthority,
    ObservabilityNonClaim::NoServiceCorrectness,
    ObservabilityNonClaim::NoClusterTruth,
    ObservabilityNonClaim::NoReleaseReadiness,
    ObservabilityNonClaim::NoConfidentiality,
    ObservabilityNonClaim::NoCompletenessBeyondPlan,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservabilityNonClaim {
    NoCapabilityAuthority,
    NoRepairAuthority,
    NoServiceCorrectness,
    NoClusterTruth,
    NoReleaseReadiness,
    NoConfidentiality,
    NoCompletenessBeyondPlan,
}

impl ObservabilityNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCapabilityAuthority => "observation-does-not-grant-capability-authority",
            Self::NoRepairAuthority => "finding-does-not-grant-repair-authority",
            Self::NoServiceCorrectness => "health-does-not-prove-service-correctness",
            Self::NoClusterTruth => "local-observation-does-not-prove-cluster-truth",
            Self::NoReleaseReadiness => "readiness-does-not-prove-release-eligibility",
            Self::NoConfidentiality => "redaction-does-not-prove-confidentiality",
            Self::NoCompletenessBeyondPlan => "scan-does-not-prove-completeness-beyond-plan",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationBounds {
    pub max_descriptors: usize,
    pub max_labels_per_sample: usize,
    pub max_label_name_bytes: usize,
    pub max_label_value_bytes: usize,
    pub max_series: usize,
    pub max_events: usize,
    pub max_event_detail_bytes: usize,
    pub max_queued_bytes: u64,
    pub max_snapshots: usize,
    pub max_scan_items: usize,
    pub max_findings: usize,
    pub max_diagnostics: usize,
    pub min_export_interval_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionRule {
    pub label_name: String,
    pub class: LabelClass,
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub bounds: ObservationBounds,
    pub redaction_rules: Vec<RedactionRule>,
    pub non_claims: Vec<ObservabilityNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimScope {
    LocalComponent,
    Adapter,
    SystemExtension,
    Cluster,
    Operator,
    ReleaseProduction,
}

impl ClaimScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalComponent => "local-component",
            Self::Adapter => "adapter",
            Self::SystemExtension => "system-extension",
            Self::Cluster => "cluster",
            Self::Operator => "operator",
            Self::ReleaseProduction => "release-production",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationContext {
    pub source_id: String,
    pub source_ref: String,
    pub profile_ref: String,
    pub scope: ClaimScope,
    pub generation: u64,
    pub observed_tick: u64,
    pub valid_until_tick: u64,
    pub resource_ref: String,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ObservabilityNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricKind {
    Counter,
    Gauge,
}

impl MetricKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricAggregation {
    Sum,
    Last,
    Minimum,
    Maximum,
}

impl MetricAggregation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Last => "last",
            Self::Minimum => "minimum",
            Self::Maximum => "maximum",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDescriptor {
    pub schema: String,
    pub descriptor_id: String,
    pub descriptor_ref: String,
    pub profile_ref: String,
    pub name: String,
    pub unit: String,
    pub kind: MetricKind,
    pub aggregation: MetricAggregation,
    pub allowed_label_names: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelClass {
    Public,
    Credential,
    Secret,
    PrivatePath,
    RawTicket,
    Payload,
    UnboundedIdentifier,
    Redacted,
}

impl LabelClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Credential => "credential",
            Self::Secret => "secret",
            Self::PrivatePath => "private-path",
            Self::RawTicket => "raw-ticket",
            Self::Payload => "payload",
            Self::UnboundedIdentifier => "unbounded-identifier",
            Self::Redacted => "redacted",
        }
    }

    pub const fn requires_redaction(self) -> bool {
        !matches!(self, Self::Public | Self::Redacted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetricLabel {
    pub name: String,
    pub value: String,
    pub class: LabelClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSample {
    pub schema: String,
    pub sample_ref: String,
    pub descriptor_ref: String,
    pub context: ObservationContext,
    pub labels: Vec<MetricLabel>,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeriesIdentity {
    pub descriptor_ref: String,
    pub labels: Vec<MetricLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedSeries {
    pub identity: SeriesIdentity,
    pub descriptor_id: String,
    pub metric_name: String,
    pub unit: String,
    pub kind: MetricKind,
    pub aggregation: MetricAggregation,
    pub value: i64,
    pub source_sample_refs: Vec<String>,
    pub latest_observed_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
}

impl EventSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationEvent {
    pub schema: String,
    pub event_ref: String,
    pub event_kind: String,
    pub severity: EventSeverity,
    pub context: ObservationContext,
    pub detail: String,
    pub attributes: Vec<MetricLabel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unavailable,
    Failed,
}

impl HealthState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }

    pub const fn severity(self) -> u8 {
        match self {
            Self::Healthy => HEALTHY_SEVERITY,
            Self::Degraded => DEGRADED_SEVERITY,
            Self::Unavailable => UNAVAILABLE_SEVERITY,
            Self::Failed => FAILED_SEVERITY,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthInput {
    pub schema: String,
    pub health_ref: String,
    pub context: ObservationContext,
    pub state: HealthState,
    pub diagnostic_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessPolicy {
    pub schema: String,
    pub policy_ref: String,
    pub target_scope: ClaimScope,
    pub required_source_ids: Vec<String>,
    pub scope_evidence_refs: Vec<String>,
    pub allow_degraded: bool,
    pub as_of_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessDecision {
    Pass,
    Degraded,
    Unavailable,
    Deny,
}

impl ReadinessDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthDecision {
    pub prior_state: HealthState,
    pub state: HealthState,
    pub readiness: ReadinessDecision,
    pub scope: ClaimScope,
    pub supporting_health_refs: Vec<String>,
    pub issues: Vec<ObservabilityIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityTargetKind {
    DurableRecord,
    LogEntry,
    Snapshot,
    Content,
    Index,
    Receipt,
    Checkpoint,
}

impl IntegrityTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DurableRecord => "durable-record",
            Self::LogEntry => "log-entry",
            Self::Snapshot => "snapshot",
            Self::Content => "content",
            Self::Index => "index",
            Self::Receipt => "receipt",
            Self::Checkpoint => "checkpoint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityTarget {
    pub item_ref: String,
    pub kind: IntegrityTargetKind,
    pub expected_content_ref: Option<String>,
    pub expected_length: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityPlan {
    pub schema: String,
    pub plan_ref: String,
    pub profile_ref: String,
    pub scope_ref: String,
    pub generation: u64,
    pub read_only: bool,
    pub require_complete: bool,
    pub max_items: usize,
    pub max_findings: usize,
    pub targets: Vec<IntegrityTarget>,
    pub resource_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ObservabilityNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanItemStatus {
    Present,
    Missing,
    Corrupt,
    PermissionDenied,
    Unsupported,
    OverBound,
    Cancelled,
}

impl ScanItemStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Missing => "missing",
            Self::Corrupt => "corrupt",
            Self::PermissionDenied => "permission-denied",
            Self::Unsupported => "unsupported",
            Self::OverBound => "over-bound",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanObservation {
    pub schema: String,
    pub observation_ref: String,
    pub plan_ref: String,
    pub item_ref: String,
    pub kind: IntegrityTargetKind,
    pub status: ScanItemStatus,
    pub observed_content_ref: Option<String>,
    pub observed_length: Option<u64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanCompletion {
    pub scanned_items: usize,
    pub declared_items: usize,
    pub exhausted: bool,
    pub cancelled: bool,
    pub unavailable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingClass {
    Missing,
    Corrupt,
    ContentMismatch,
    LengthMismatch,
    Unexpected,
    PermissionDenied,
    Unsupported,
    OverBound,
    PartialScan,
    Cancelled,
    Unavailable,
}

impl FindingClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Corrupt => "corrupt",
            Self::ContentMismatch => "content-mismatch",
            Self::LengthMismatch => "length-mismatch",
            Self::Unexpected => "unexpected",
            Self::PermissionDenied => "permission-denied",
            Self::Unsupported => "unsupported",
            Self::OverBound => "over-bound",
            Self::PartialScan => "partial-scan",
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairRecommendation {
    VerifyAgain,
    RepairCandidate,
    QuarantineCandidate,
    RestoreCandidate,
    OperatorReview,
}

impl RepairRecommendation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifyAgain => "verify-again",
            Self::RepairCandidate => "repair-candidate",
            Self::QuarantineCandidate => "quarantine-candidate",
            Self::RestoreCandidate => "restore-candidate",
            Self::OperatorReview => "operator-review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityFinding {
    pub schema: String,
    pub finding_id: String,
    pub item_ref: Option<String>,
    pub class: FindingClass,
    pub expected_ref: Option<String>,
    pub observed_ref: Option<String>,
    pub recommendation: RepairRecommendation,
    pub grants_mutation_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityDecision {
    Pass,
    Fail,
    Partial,
    Cancelled,
    Unavailable,
    Deny,
}

impl IntegrityDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Partial => "partial",
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityResult {
    pub plan_ref: String,
    pub decision: IntegrityDecision,
    pub scanned_items: usize,
    pub declared_items: usize,
    pub findings: Vec<IntegrityFinding>,
    pub complete: bool,
    pub mutation_performed: bool,
    pub issues: Vec<ObservabilityIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrityMutationOperation {
    Repair,
    Quarantine,
    Retain,
    Recover,
    Delete,
}

impl IntegrityMutationOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::Quarantine => "quarantine",
            Self::Retain => "retain",
            Self::Recover => "recover",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityMutationAuthority {
    pub schema: String,
    pub operation: IntegrityMutationOperation,
    pub authority_ref: String,
    pub policy_ref: String,
    pub finding_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityDecision {
    Admit,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationAdapterClass {
    Tracing,
    Prometheus,
    OpenTelemetry,
    DurableStateScan,
    ContentVerification,
    RuntimeCounter,
    DeterministicSimulation,
}

impl ObservationAdapterClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tracing => "tracing",
            Self::Prometheus => "prometheus",
            Self::OpenTelemetry => "opentelemetry",
            Self::DurableStateScan => "durable-state-scan",
            Self::ContentVerification => "content-verification",
            Self::RuntimeCounter => "runtime-counter",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationAdapterProfile {
    pub schema: String,
    pub adapter_id: String,
    pub adapter_ref: String,
    pub profile_ref: String,
    pub class: ObservationAdapterClass,
    pub max_queued_bytes: u64,
    pub timeout_ticks: u64,
    pub drop_on_backpressure: bool,
    pub required: bool,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ObservabilityNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDeliveryRequest {
    pub operation_ref: String,
    pub adapter_ref: String,
    pub payload_ref: String,
    pub payload_bytes: u64,
    pub submitted_tick: u64,
    pub deadline_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterFailureClass {
    PermissionDenied,
    UnsupportedCapability,
    CorruptInput,
    AdapterFailure,
}

impl AdapterFailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PermissionDenied => "permission-denied",
            Self::UnsupportedCapability => "unsupported-capability",
            Self::CorruptInput => "corrupt-input",
            Self::AdapterFailure => "adapter-failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRuntimeObservation {
    pub available: bool,
    pub queued_bytes: u64,
    pub completed_tick: u64,
    pub dropped_observations: u64,
    pub cancelled: bool,
    pub failure: Option<AdapterFailureClass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterOutcomeKind {
    Submit,
    Exported,
    Unavailable,
    Backpressure,
    Timeout,
    Dropped,
    Partial,
    Stale,
    Cancelled,
    PermissionDenied,
    Unsupported,
    Corrupt,
    Failed,
}

impl AdapterOutcomeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Exported => "exported",
            Self::Unavailable => "unavailable",
            Self::Backpressure => "backpressure",
            Self::Timeout => "timeout",
            Self::Dropped => "dropped",
            Self::Partial => "partial",
            Self::Stale => "stale",
            Self::Cancelled => "cancelled",
            Self::PermissionDenied => "permission-denied",
            Self::Unsupported => "unsupported",
            Self::Corrupt => "corrupt",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterOutcome {
    pub operation_ref: String,
    pub adapter_ref: String,
    pub payload_ref: String,
    pub kind: AdapterOutcomeKind,
    pub dropped_observations: u64,
    pub service_policy_signal: bool,
    pub issues: Vec<ObservabilityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationAdapterStatus {
    pub schema: String,
    pub adapter_ref: String,
    pub class: ObservationAdapterClass,
    pub kind: AdapterOutcomeKind,
    pub observed_tick: u64,
    pub queued_bytes: u64,
    pub dropped_observations: u64,
    pub evidence_refs: Vec<String>,
    pub issues: Vec<ObservabilityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSnapshot {
    pub schema: String,
    pub snapshot_id: String,
    pub profile_ref: String,
    pub scope: ClaimScope,
    pub generation: u64,
    pub as_of_tick: u64,
    pub valid_until_tick: u64,
    pub series: Vec<AggregatedSeries>,
    pub event_refs: Vec<String>,
    pub health_refs: Vec<String>,
    pub integrity_result_refs: Vec<String>,
    pub adapter_outcome_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub non_claims: Vec<ObservabilityNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservabilityIssue {
    SchemaMismatch(&'static str),
    EmptyField(&'static str),
    MalformedToken(&'static str),
    MalformedRef(&'static str),
    ZeroBound(&'static str),
    CollectionLimitExceeded(&'static str),
    DuplicateValue(&'static str),
    MissingNonClaim(&'static str),
    ProfileMismatch,
    UnsupportedLabel(String),
    LabelValueTooLarge(String),
    LabelRequiresRedaction(String),
    RedactionRuleMissing(String),
    RedactionMarkerInvalid(String),
    DescriptorMissing(String),
    DescriptorIncompatible,
    CounterRequiresSum,
    ArithmeticOverflow,
    ObservationStale(String),
    ObservationUnavailable(String),
    RequiredSourceMissing(String),
    ClaimScopeOverreach,
    MutationWithoutAuthority,
    PlanNotReadOnly,
    ScanTargetMissing(String),
    UnexpectedScanItem(String),
    ScanPlanMismatch,
    PartialScan,
    FindingLimitExceeded,
    AdapterMismatch,
    ExportFrequencyExceeded,
    QueueBoundExceeded,
    DeadlineExceeded,
    ExporterUnavailable,
    ObservationDropped,
    Cancelled,
    PermissionDenied,
    UnsupportedCapability,
    CorruptInput,
    AdapterFailure,
    TelemetryCannotGrantAuthority,
}
