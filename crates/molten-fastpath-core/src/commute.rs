//! The versioned conflict contract and the conservative command-pair classifier.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictContract {
    pub contract_ref: alloc::string::String,
    pub version: alloc::string::String,
    pub command_schema_ref: alloc::string::String,
    pub state_schema_ref: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandShape {
    Get {
        key: alloc::string::String,
    },
    Put {
        key: alloc::string::String,
        value_ref: alloc::string::String,
    },
    Delete {
        key: alloc::string::String,
    },
    Range {
        start: alloc::string::String,
        end: alloc::string::String,
    },
    Alias {
        alias: alloc::string::String,
    },
    Conditional {
        key: alloc::string::String,
        precondition_ref: alloc::string::String,
    },
    ResponseDependent {
        key: alloc::string::String,
        dependency_ref: alloc::string::String,
    },
    Unknown {
        schema_ref: alloc::string::String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictDecision {
    Independent,
    Conflict,
    ConservativeFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictContractIssue {
    EmptyField(&'static str),
    UnsupportedVersion,
}

impl core::fmt::Display for ConflictContractIssue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "empty-field:{field}"),
            Self::UnsupportedVersion => formatter.write_str("unsupported-version"),
        }
    }
}

impl core::error::Error for ConflictContractIssue {}

// r[impl molten.consensus.fast_path_model.conflict_contract]
pub fn validate_conflict_contract(contract: &ConflictContract) -> alloc::vec::Vec<ConflictContractIssue> {
    let fields = [
        ("contract ref", contract.contract_ref.as_str()),
        ("command schema ref", contract.command_schema_ref.as_str()),
        ("state schema ref", contract.state_schema_ref.as_str()),
    ];
    let mut issues: alloc::vec::Vec<ConflictContractIssue> = fields
        .iter()
        .filter(|(_, value)| value.trim().is_empty())
        .map(|(field, _)| ConflictContractIssue::EmptyField(field))
        .collect();
    if contract.version != "v1" {
        issues.push(ConflictContractIssue::UnsupportedVersion);
    }
    issues
}

// r[impl molten.consensus.fast_path_model.conflict_contract]
pub fn classify_conflict(left: &CommandShape, right: &CommandShape) -> ConflictDecision {
    if requires_conservative_fallback(left) || requires_conservative_fallback(right) {
        return ConflictDecision::ConservativeFallback;
    }
    match (point_key(left), point_key(right)) {
        (Some(left), Some(right)) => independent_keys(left, right),
        _ => ConflictDecision::ConservativeFallback,
    }
}

/// The single key a point read, write, or delete touches; `None` for every other shape.
fn point_key(command: &CommandShape) -> Option<&str> {
    match command {
        CommandShape::Get { key } | CommandShape::Put { key, .. } | CommandShape::Delete { key } => Some(key),
        CommandShape::Range { .. }
        | CommandShape::Alias { .. }
        | CommandShape::Conditional { .. }
        | CommandShape::ResponseDependent { .. }
        | CommandShape::Unknown { .. } => None,
    }
}

fn independent_keys(left: &str, right: &str) -> ConflictDecision {
    if left == right {
        ConflictDecision::Conflict
    } else {
        ConflictDecision::Independent
    }
}

fn requires_conservative_fallback(command: &CommandShape) -> bool {
    point_key(command).is_none()
}
