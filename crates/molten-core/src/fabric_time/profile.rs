use super::FABRIC_TIME_PROFILE_SCHEMA;
use super::MAX_TIME_COLLECTION_ITEMS;
use super::REQUIRED_TIME_NON_CLAIMS;
use super::SchedulerPolicy;
use super::TimeNonClaim;
use super::has_duplicates;
use super::valid_time_id;
use super::valid_time_ref;

const MAX_PROFILE_TIMERS: u64 = 65_536;
const MAX_PROFILE_RUNNABLES: u64 = 65_536;
const MAX_PROFILE_ENTROPY_REQUEST_BYTES: u64 = 1_048_576;
const MAX_PROFILE_ENTROPY_TOTAL_BYTES: u64 = 1_073_741_824;
const MAX_PROFILE_CONCURRENCY: u64 = 4_096;
const MAX_PROFILE_QUEUE_DEPTH: u64 = 65_536;
const MAX_PROFILE_FAIRNESS_TURNS: u64 = 1_000_000;
const REQUIRED_DOMAIN_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeDomain {
    WallClock,
    Monotonic,
    Logical,
    Virtual,
}

impl TimeDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WallClock => "wall-clock",
            Self::Monotonic => "monotonic",
            Self::Logical => "logical",
            Self::Virtual => "virtual",
        }
    }
}

pub const REQUIRED_TIME_DOMAINS: [TimeDomain; REQUIRED_DOMAIN_COUNT] = [
    TimeDomain::WallClock,
    TimeDomain::Monotonic,
    TimeDomain::Logical,
    TimeDomain::Virtual,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeProfileKind {
    Live,
    DeterministicSimulation,
}

impl TimeProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeEvidenceMode {
    Aggregate,
    SelectedSemanticBoundaries,
}

impl TimeEvidenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Aggregate => "aggregate",
            Self::SelectedSemanticBoundaries => "selected-semantic-boundaries",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeProfileDescriptor {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub kind: TimeProfileKind,
    pub supported_domains: Vec<TimeDomain>,
    pub max_duration_ticks: u64,
    pub max_uncertainty_ticks: u64,
    pub max_timers: u64,
    pub max_runnables: u64,
    pub max_entropy_request_bytes: u64,
    pub max_entropy_total_bytes: u64,
    pub max_scheduler_concurrency: u64,
    pub max_scheduler_queue_depth: u64,
    pub fairness_bound_turns: Option<u64>,
    pub scheduler_policy: SchedulerPolicy,
    pub evidence_mode: TimeEvidenceMode,
    pub non_claims: Vec<TimeNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedTimeProfile {
    pub profile_id: String,
    pub profile_ref: String,
    pub kind: TimeProfileKind,
    pub supported_domains: Vec<TimeDomain>,
    pub max_duration_ticks: u64,
    pub max_uncertainty_ticks: u64,
    pub max_timers: u64,
    pub max_runnables: u64,
    pub max_entropy_request_bytes: u64,
    pub max_entropy_total_bytes: u64,
    pub max_scheduler_concurrency: u64,
    pub max_scheduler_queue_depth: u64,
    pub fairness_bound_turns: Option<u64>,
    pub scheduler_policy: SchedulerPolicy,
    pub evidence_mode: TimeEvidenceMode,
    pub non_claims: Vec<TimeNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeProfileIssue {
    SchemaMismatch {
        actual: String,
        expected: String,
    },
    MalformedId {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    DuplicateDomain,
    MissingDomain(TimeDomain),
    ZeroLimit(&'static str),
    HardLimitExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    InconsistentLimits {
        smaller: &'static str,
        larger: &'static str,
    },
    DuplicateNonClaim,
    MissingNonClaim(TimeNonClaim),
    TooManyItems {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
}

// r[impl molten.fabric_time.time_domains]
// r[impl molten.fabric_time.non_claims]
pub fn admit_time_profile(descriptor: &TimeProfileDescriptor) -> Result<AdmittedTimeProfile, Vec<TimeProfileIssue>> {
    let mut issues = Vec::new();
    if descriptor.schema != FABRIC_TIME_PROFILE_SCHEMA {
        issues.push(TimeProfileIssue::SchemaMismatch {
            actual: descriptor.schema.clone(),
            expected: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        });
    }
    validate_identity(descriptor, &mut issues);
    validate_domains(descriptor, &mut issues);
    validate_limits(descriptor, &mut issues);
    validate_non_claims(descriptor, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut supported_domains = descriptor.supported_domains.clone();
    supported_domains.sort();
    let mut non_claims = descriptor.non_claims.clone();
    non_claims.sort();
    Ok(AdmittedTimeProfile {
        profile_id: descriptor.profile_id.clone(),
        profile_ref: descriptor.profile_ref.clone(),
        kind: descriptor.kind,
        supported_domains,
        max_duration_ticks: descriptor.max_duration_ticks,
        max_uncertainty_ticks: descriptor.max_uncertainty_ticks,
        max_timers: descriptor.max_timers,
        max_runnables: descriptor.max_runnables,
        max_entropy_request_bytes: descriptor.max_entropy_request_bytes,
        max_entropy_total_bytes: descriptor.max_entropy_total_bytes,
        max_scheduler_concurrency: descriptor.max_scheduler_concurrency,
        max_scheduler_queue_depth: descriptor.max_scheduler_queue_depth,
        fairness_bound_turns: descriptor.fairness_bound_turns,
        scheduler_policy: descriptor.scheduler_policy,
        evidence_mode: descriptor.evidence_mode,
        non_claims,
    })
}

fn validate_identity(descriptor: &TimeProfileDescriptor, issues: &mut Vec<TimeProfileIssue>) {
    if !valid_time_id(&descriptor.profile_id) {
        issues.push(TimeProfileIssue::MalformedId {
            field: "profile-id",
            value: descriptor.profile_id.clone(),
        });
    }
    if !valid_time_ref(&descriptor.profile_ref) {
        issues.push(TimeProfileIssue::MalformedRef {
            field: "profile-ref",
            value: descriptor.profile_ref.clone(),
        });
    }
}

fn validate_domains(descriptor: &TimeProfileDescriptor, issues: &mut Vec<TimeProfileIssue>) {
    if descriptor.supported_domains.len() > MAX_TIME_COLLECTION_ITEMS {
        issues.push(TimeProfileIssue::TooManyItems {
            field: "supported-domains",
            actual: descriptor.supported_domains.len(),
            maximum: MAX_TIME_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&descriptor.supported_domains) {
        issues.push(TimeProfileIssue::DuplicateDomain);
    }
    for domain in REQUIRED_TIME_DOMAINS {
        if !descriptor.supported_domains.contains(&domain) {
            issues.push(TimeProfileIssue::MissingDomain(domain));
        }
    }
}

fn validate_limits(descriptor: &TimeProfileDescriptor, issues: &mut Vec<TimeProfileIssue>) {
    validate_positive("max-duration-ticks", descriptor.max_duration_ticks, u64::MAX, issues);
    validate_limit("max-uncertainty-ticks", descriptor.max_uncertainty_ticks, u64::MAX, issues);
    validate_positive("max-timers", descriptor.max_timers, MAX_PROFILE_TIMERS, issues);
    validate_positive("max-runnables", descriptor.max_runnables, MAX_PROFILE_RUNNABLES, issues);
    validate_positive(
        "max-entropy-request-bytes",
        descriptor.max_entropy_request_bytes,
        MAX_PROFILE_ENTROPY_REQUEST_BYTES,
        issues,
    );
    validate_positive(
        "max-entropy-total-bytes",
        descriptor.max_entropy_total_bytes,
        MAX_PROFILE_ENTROPY_TOTAL_BYTES,
        issues,
    );
    validate_positive(
        "max-scheduler-concurrency",
        descriptor.max_scheduler_concurrency,
        MAX_PROFILE_CONCURRENCY,
        issues,
    );
    validate_positive(
        "max-scheduler-queue-depth",
        descriptor.max_scheduler_queue_depth,
        MAX_PROFILE_QUEUE_DEPTH,
        issues,
    );
    if let Some(bound) = descriptor.fairness_bound_turns {
        validate_positive("fairness-bound-turns", bound, MAX_PROFILE_FAIRNESS_TURNS, issues);
    }
    validate_limit_relation(
        "max-entropy-request-bytes",
        descriptor.max_entropy_request_bytes,
        "max-entropy-total-bytes",
        descriptor.max_entropy_total_bytes,
        issues,
    );
    validate_limit_relation(
        "max-scheduler-concurrency",
        descriptor.max_scheduler_concurrency,
        "max-runnables",
        descriptor.max_runnables,
        issues,
    );
    validate_limit_relation(
        "max-scheduler-queue-depth",
        descriptor.max_scheduler_queue_depth,
        "max-runnables",
        descriptor.max_runnables,
        issues,
    );
}

fn validate_non_claims(descriptor: &TimeProfileDescriptor, issues: &mut Vec<TimeProfileIssue>) {
    if descriptor.non_claims.len() > MAX_TIME_COLLECTION_ITEMS {
        issues.push(TimeProfileIssue::TooManyItems {
            field: "non-claims",
            actual: descriptor.non_claims.len(),
            maximum: MAX_TIME_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&descriptor.non_claims) {
        issues.push(TimeProfileIssue::DuplicateNonClaim);
    }
    for non_claim in REQUIRED_TIME_NON_CLAIMS {
        if !descriptor.non_claims.contains(&non_claim) {
            issues.push(TimeProfileIssue::MissingNonClaim(non_claim));
        }
    }
}

fn validate_limit_relation(
    smaller: &'static str,
    smaller_value: u64,
    larger: &'static str,
    larger_value: u64,
    issues: &mut Vec<TimeProfileIssue>,
) {
    if smaller_value > larger_value {
        issues.push(TimeProfileIssue::InconsistentLimits { smaller, larger });
    }
}

fn validate_positive(field: &'static str, actual: u64, maximum: u64, issues: &mut Vec<TimeProfileIssue>) {
    if actual == 0 {
        issues.push(TimeProfileIssue::ZeroLimit(field));
    }
    validate_limit(field, actual, maximum, issues);
}

fn validate_limit(field: &'static str, actual: u64, maximum: u64, issues: &mut Vec<TimeProfileIssue>) {
    if actual > maximum {
        issues.push(TimeProfileIssue::HardLimitExceeded { field, actual, maximum });
    }
}
