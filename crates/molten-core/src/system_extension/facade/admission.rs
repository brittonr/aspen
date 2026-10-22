use super::super::MAX_SYSTEM_EXTENSION_ITEMS;
use super::super::duplicates;
use super::super::valid_ref;
use super::super::valid_token;

pub const FACADE_MANIFEST_SCHEMA: &str = "molten.system-extension.facade-manifest.v1";
pub const MAX_FACADE_PHASES: usize = 32;
pub const MAX_FACADE_POSTPONEMENT_BOUND: u64 = 1_024;
pub const MAX_FACADE_STATE_TIMEOUT_TICKS: u64 = 1_000_000;

/// The library-level facade profile over the system-extension interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacadeProfile {
    Service,
    StateMachine,
    Task,
}

impl FacadeProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::StateMachine => "state-machine",
            Self::Task => "task",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "service" => Some(Self::Service),
            "state-machine" => Some(Self::StateMachine),
            "task" => Some(Self::Task),
            _ => None,
        }
    }
}

/// How a state timeout is fenced. A facade never admits an unfenced timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutFencing {
    IncarnationAndPhase,
}

impl TimeoutFencing {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IncarnationAndPhase => "incarnation-and-phase",
        }
    }
}

/// One phase of a state-machine facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadePhaseEntry {
    pub phase: String,
    pub legal_events: Vec<String>,
    pub postponable_events: Vec<String>,
    pub postponement_bound: Option<u64>,
    pub state_timeout_ticks: Option<u64>,
    pub timeout_fencing: Option<TimeoutFencing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacadeManifestInput {
    pub schema: String,
    pub extension_manifest_ref: String,
    pub service_id: String,
    pub profile: FacadeProfile,
    pub event_schema_ref: String,
    pub state_schema_ref: String,
    pub reply_schema_ref: String,
    pub completion_schema_ref: Option<String>,
    pub phase_table: Vec<FacadePhaseEntry>,
    pub concurrency_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedFacadeManifest {
    pub extension_manifest_ref: String,
    pub service_id: String,
    pub profile: FacadeProfile,
    pub event_schema_ref: String,
    pub state_schema_ref: String,
    pub reply_schema_ref: String,
    pub completion_schema_ref: Option<String>,
    pub phase_table: Vec<FacadePhaseEntry>,
    pub concurrency_limit: u64,
}

impl AdmittedFacadeManifest {
    pub fn phase_entry(&self, phase: &str) -> Option<&FacadePhaseEntry> {
        self.phase_table.iter().find(|entry| entry.phase == phase)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacadeAdmissionIssue {
    SchemaMismatch { actual: String },
    MissingRef { field: &'static str },
    MalformedRef { field: &'static str, value: String },
    MalformedToken { field: &'static str, value: String },
    EmptyServiceId,
    MissingPhaseTable,
    TooManyPhases { actual: usize, maximum: usize },
    DuplicatePhase { phase: String },
    EmptyPhase { phase: String },
    UnknownPhaseEvent { phase: String, event: String },
    DuplicatePhaseEvent { phase: String, event: String },
    UnboundedPostponement { phase: String },
    PostponementBoundTooLarge { phase: String, bound: u64 },
    UnfencedStateTimeout { phase: String },
    StateTimeoutTooLarge { phase: String, ticks: u64 },
    MissingTaskCompletionSchema,
    UnexpectedTaskCompletionSchema,
    UnexpectedPhaseTable,
    ConcurrencyLimitBelowOne,
}

// r[impl molten.system_extension.service_facades]
pub fn admit_facade_manifest(input: &FacadeManifestInput) -> Result<AdmittedFacadeManifest, Vec<FacadeAdmissionIssue>> {
    let mut issues = Vec::new();
    if input.schema != FACADE_MANIFEST_SCHEMA {
        issues.push(FacadeAdmissionIssue::SchemaMismatch {
            actual: input.schema.clone(),
        });
    }
    if input.service_id.trim().is_empty() {
        issues.push(FacadeAdmissionIssue::EmptyServiceId);
    }
    for (field, value) in [
        ("extension-manifest-ref", input.extension_manifest_ref.as_str()),
        ("event-schema-ref", input.event_schema_ref.as_str()),
        ("state-schema-ref", input.state_schema_ref.as_str()),
        ("reply-schema-ref", input.reply_schema_ref.as_str()),
    ] {
        if value.is_empty() {
            issues.push(FacadeAdmissionIssue::MissingRef { field });
        } else if !valid_ref(value) {
            issues.push(FacadeAdmissionIssue::MalformedRef {
                field,
                value: value.to_string(),
            });
        }
    }
    for (field, value) in [
        ("event-schema-ref", input.event_schema_ref.as_str()),
        ("state-schema-ref", input.state_schema_ref.as_str()),
        ("reply-schema-ref", input.reply_schema_ref.as_str()),
    ] {
        if !value.is_empty() && !valid_token(value) {
            issues.push(FacadeAdmissionIssue::MalformedToken {
                field,
                value: value.to_string(),
            });
        }
    }
    match input.profile {
        FacadeProfile::Task => {
            if input.completion_schema_ref.is_none() {
                issues.push(FacadeAdmissionIssue::MissingTaskCompletionSchema);
            }
        }
        _ => {
            if input.completion_schema_ref.is_some() {
                issues.push(FacadeAdmissionIssue::UnexpectedTaskCompletionSchema);
            }
        }
    }
    if let Some(completion_schema_ref) = &input.completion_schema_ref
        && !valid_ref(completion_schema_ref)
    {
        issues.push(FacadeAdmissionIssue::MalformedRef {
            field: "completion-schema-ref",
            value: completion_schema_ref.clone(),
        });
    }
    if input.concurrency_limit < 1 {
        issues.push(FacadeAdmissionIssue::ConcurrencyLimitBelowOne);
    }
    validate_phase_table(input, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(AdmittedFacadeManifest {
        extension_manifest_ref: input.extension_manifest_ref.clone(),
        service_id: input.service_id.clone(),
        profile: input.profile,
        event_schema_ref: input.event_schema_ref.clone(),
        state_schema_ref: input.state_schema_ref.clone(),
        reply_schema_ref: input.reply_schema_ref.clone(),
        completion_schema_ref: input.completion_schema_ref.clone(),
        phase_table: input.phase_table.clone(),
        concurrency_limit: input.concurrency_limit,
    })
}

fn validate_phase_table(input: &FacadeManifestInput, issues: &mut Vec<FacadeAdmissionIssue>) {
    if input.profile != FacadeProfile::StateMachine {
        if !input.phase_table.is_empty() {
            issues.push(FacadeAdmissionIssue::UnexpectedPhaseTable);
        }
        return;
    }
    if input.phase_table.is_empty() {
        issues.push(FacadeAdmissionIssue::MissingPhaseTable);
        return;
    }
    if input.phase_table.len() > MAX_FACADE_PHASES {
        issues.push(FacadeAdmissionIssue::TooManyPhases {
            actual: input.phase_table.len(),
            maximum: MAX_FACADE_PHASES,
        });
        return;
    }
    let phases: Vec<&str> = input.phase_table.iter().map(|entry| entry.phase.as_str()).collect();
    if duplicates(&phases) {
        for phase in &phases {
            if input.phase_table.iter().filter(|entry| entry.phase == *phase).count() > 1 {
                issues.push(FacadeAdmissionIssue::DuplicatePhase {
                    phase: (*phase).to_string(),
                });
                break;
            }
        }
    }
    for entry in &input.phase_table {
        if entry.phase.trim().is_empty() {
            issues.push(FacadeAdmissionIssue::EmptyPhase {
                phase: entry.phase.clone(),
            });
            continue;
        }
        if entry.legal_events.len() + entry.postponable_events.len() > MAX_SYSTEM_EXTENSION_ITEMS {
            issues.push(FacadeAdmissionIssue::TooManyPhases {
                actual: entry.legal_events.len() + entry.postponable_events.len(),
                maximum: MAX_SYSTEM_EXTENSION_ITEMS,
            });
        }
        for event in entry.legal_events.iter().chain(entry.postponable_events.iter()) {
            if event.trim().is_empty() {
                issues.push(FacadeAdmissionIssue::UnknownPhaseEvent {
                    phase: entry.phase.clone(),
                    event: event.clone(),
                });
            }
        }
        let mut seen: Vec<&str> = entry.legal_events.iter().map(String::as_str).collect();
        for event in &entry.postponable_events {
            if seen.contains(&event.as_str()) {
                issues.push(FacadeAdmissionIssue::DuplicatePhaseEvent {
                    phase: entry.phase.clone(),
                    event: event.clone(),
                });
            }
            seen.push(event.as_str());
        }
        if !entry.postponable_events.is_empty() {
            match entry.postponement_bound {
                None => issues.push(FacadeAdmissionIssue::UnboundedPostponement {
                    phase: entry.phase.clone(),
                }),
                Some(bound) if bound > MAX_FACADE_POSTPONEMENT_BOUND => {
                    issues.push(FacadeAdmissionIssue::PostponementBoundTooLarge {
                        phase: entry.phase.clone(),
                        bound,
                    });
                }
                Some(_) => {}
            }
        } else if entry.postponement_bound.is_some() {
            issues.push(FacadeAdmissionIssue::UnboundedPostponement {
                phase: entry.phase.clone(),
            });
        }
        match (entry.state_timeout_ticks, entry.timeout_fencing) {
            (Some(ticks), Some(TimeoutFencing::IncarnationAndPhase)) => {
                if ticks > MAX_FACADE_STATE_TIMEOUT_TICKS {
                    issues.push(FacadeAdmissionIssue::StateTimeoutTooLarge {
                        phase: entry.phase.clone(),
                        ticks,
                    });
                }
            }
            (Some(_), None) => issues.push(FacadeAdmissionIssue::UnfencedStateTimeout {
                phase: entry.phase.clone(),
            }),
            (None, Some(_)) => issues.push(FacadeAdmissionIssue::UnfencedStateTimeout {
                phase: entry.phase.clone(),
            }),
            (None, None) => {}
        }
    }
}
