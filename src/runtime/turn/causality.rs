// The closed cause vocabulary of a committed turn.
//
// The cause comes from the transition input. It is never derived from a wall
// clock, and an unknown or absent cause fails validation.

type IoValue = preserves::IOValue;

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

/// The release reason of a linked task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskReleaseReason {
    Cancelled,
    Normal,
}

impl TaskReleaseReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Normal => "normal",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cancelled" => Some(Self::Cancelled),
            "normal" => Some(Self::Normal),
            _ => None,
        }
    }
}

/// The closed cause vocabulary of a committed turn.
///
/// The cause comes from the transition input. It is never derived from a wall
/// clock, and an unknown or absent cause fails validation.
// r[impl molten.runtime_spine.turn_causality]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTurnCause {
    /// The turn that released this work, by turn ref.
    PriorTurn { turn_ref: String },
    /// The automatic retraction of an owner's outstanding assertions.
    Cleanup,
    /// A linked task released by ref, with the reason it ended.
    LinkedTaskRelease {
        task_ref: String,
        reason: TaskReleaseReason,
    },
    /// A declared periodic activation.
    PeriodicActivation { period_ticks: u64 },
    /// A delayed turn, naming the causing turn and the amount that expired.
    Delay {
        causing_turn_ref: String,
        amount_ticks: u64,
    },
    /// An input outside the runtime model, with a bounded description.
    External { description: String },
}

/// Why a turn cause was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCauseIssue {
    MalformedCause(String),
    UnknownCause(String),
    UnknownReleaseReason(String),
    DescriptionTooLong { actual: usize, maximum: usize },
}

const MAX_CAUSE_DESCRIPTION_BYTES: usize = 256;

impl RuntimeTurnCause {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PriorTurn { .. } => "prior-turn",
            Self::Cleanup => "cleanup",
            Self::LinkedTaskRelease { .. } => "linked-task-release",
            Self::PeriodicActivation { .. } => "periodic-activation",
            Self::Delay { .. } => "delay",
            Self::External { .. } => "external",
        }
    }

    /// The external cause for an input outside the runtime model.
    pub fn external(description: &str) -> Self {
        Self::External {
            description: description.to_string(),
        }
    }

    /// The canonical cause record.
    pub fn to_value(&self) -> IoValue {
        let mut fields = vec![string(self.as_str())];
        match self {
            Self::PriorTurn { turn_ref } => fields.push(string(turn_ref)),
            Self::Cleanup => {}
            Self::LinkedTaskRelease { task_ref, reason } => {
                fields.push(string(task_ref));
                fields.push(string(reason.as_str()));
            }
            Self::PeriodicActivation { period_ticks } => fields.push(u64_value(*period_ticks)),
            Self::Delay {
                causing_turn_ref,
                amount_ticks,
            } => {
                fields.push(string(causing_turn_ref));
                fields.push(u64_value(*amount_ticks));
            }
            Self::External { description } => fields.push(string(description)),
        }
        record("runtime-turn-cause-v1", fields)
    }

    /// Decode one cause record. An absent or unknown cause is refused.
    pub fn from_value(value: &IoValue) -> std::result::Result<Self, RuntimeCauseIssue> {
        let fields = value
            .collect_simple_record("runtime-turn-cause-v1", None)
            .ok_or_else(|| RuntimeCauseIssue::MalformedCause("the turn cause record is malformed".to_string()))?;
        let kind = fields
            .fields_iter()
            .next()
            .and_then(|field| field.as_string())
            .ok_or_else(|| RuntimeCauseIssue::MalformedCause("the turn cause kind is unreadable".to_string()))?;
        let kind = kind.to_string();
        let argument = |index: usize| -> std::result::Result<String, RuntimeCauseIssue> {
            fields
                .fields_iter()
                .nth(index)
                .and_then(|field| field.as_string())
                .map(|value| value.to_string())
                .ok_or_else(|| RuntimeCauseIssue::MalformedCause("the turn cause argument is unreadable".to_string()))
        };
        let number = |index: usize| -> std::result::Result<u64, RuntimeCauseIssue> {
            fields
                .fields_iter()
                .nth(index)
                .and_then(|field| field.as_u64().and_then(|value| value.ok()))
                .ok_or_else(|| RuntimeCauseIssue::MalformedCause("the turn cause number is unreadable".to_string()))
        };
        let field_count = fields.fields_iter().count();
        let arity = match kind.as_str() {
            "prior-turn" | "periodic-activation" | "external" => 2,
            "cleanup" => 1,
            "linked-task-release" | "delay" => 3,
            other => return Err(RuntimeCauseIssue::UnknownCause(other.to_string())),
        };
        if field_count != arity {
            return Err(RuntimeCauseIssue::MalformedCause(format!(
                "the {kind} cause has {field_count} fields, expected {arity}"
            )));
        }
        match kind.as_str() {
            "prior-turn" => Ok(Self::PriorTurn { turn_ref: argument(1)? }),
            "cleanup" => Ok(Self::Cleanup),
            "linked-task-release" => {
                let reason = argument(2)?;
                let reason =
                    TaskReleaseReason::parse(&reason).ok_or(RuntimeCauseIssue::UnknownReleaseReason(reason))?;
                Ok(Self::LinkedTaskRelease {
                    task_ref: argument(1)?,
                    reason,
                })
            }
            "periodic-activation" => Ok(Self::PeriodicActivation {
                period_ticks: number(1)?,
            }),
            "delay" => Ok(Self::Delay {
                causing_turn_ref: argument(1)?,
                amount_ticks: number(2)?,
            }),
            "external" => {
                let description = argument(1)?;
                if description.len() > MAX_CAUSE_DESCRIPTION_BYTES {
                    return Err(RuntimeCauseIssue::DescriptionTooLong {
                        actual: description.len(),
                        maximum: MAX_CAUSE_DESCRIPTION_BYTES,
                    });
                }
                Ok(Self::External { description })
            }
            other => Err(RuntimeCauseIssue::UnknownCause(other.to_string())),
        }
    }
}
