#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictContract {
    pub contract_ref: String,
    pub version: String,
    pub command_schema_ref: String,
    pub state_schema_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandShape {
    Get { key: String },
    Put { key: String, value_ref: String },
    Delete { key: String },
    Range { start: String, end: String },
    Alias { alias: String },
    Conditional { key: String, precondition_ref: String },
    ResponseDependent { key: String, dependency_ref: String },
    Unknown { schema_ref: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictDecision {
    Independent,
    Conflict,
    ConservativeFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictContractIssue {
    EmptyField(&'static str),
    UnsupportedVersion,
}

// r[impl molten.consensus.fast_path_model.conflict_contract]
pub fn validate_conflict_contract(contract: &ConflictContract) -> Vec<ConflictContractIssue> {
    let mut issues = Vec::new();
    for (field, value) in [
        ("contract ref", contract.contract_ref.as_str()),
        ("command schema ref", contract.command_schema_ref.as_str()),
        ("state schema ref", contract.state_schema_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            issues.push(ConflictContractIssue::EmptyField(field));
        }
    }
    if contract.version != "v1" {
        issues.push(ConflictContractIssue::UnsupportedVersion);
    }
    issues
}

// r[impl molten.consensus.fast_path_model.conflict_contract]
pub fn classify_conflict(left: &CommandShape, right: &CommandShape) -> ConflictDecision {
    use CommandShape::Alias;
    use CommandShape::Conditional;
    use CommandShape::Delete;
    use CommandShape::Get;
    use CommandShape::Put;
    use CommandShape::Range;
    use CommandShape::ResponseDependent;
    use CommandShape::Unknown;
    if requires_conservative_fallback(left) || requires_conservative_fallback(right) {
        return ConflictDecision::ConservativeFallback;
    }
    match (left, right) {
        (Get { key: left }, Get { key: right }) => independent_keys(left, right),
        (Get { key: left }, Put { key: right, .. })
        | (Put { key: left, .. }, Get { key: right })
        | (Get { key: left }, Delete { key: right })
        | (Delete { key: left }, Get { key: right })
        | (Put { key: left, .. }, Put { key: right, .. })
        | (Put { key: left, .. }, Delete { key: right })
        | (Delete { key: left }, Put { key: right, .. })
        | (Delete { key: left }, Delete { key: right }) => independent_keys(left, right),
        (Range { .. }, _)
        | (_, Range { .. })
        | (Alias { .. }, _)
        | (_, Alias { .. })
        | (Conditional { .. }, _)
        | (_, Conditional { .. })
        | (ResponseDependent { .. }, _)
        | (_, ResponseDependent { .. })
        | (Unknown { .. }, _)
        | (_, Unknown { .. }) => ConflictDecision::ConservativeFallback,
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
    matches!(
        command,
        CommandShape::Range { .. }
            | CommandShape::Alias { .. }
            | CommandShape::Conditional { .. }
            | CommandShape::ResponseDependent { .. }
            | CommandShape::Unknown { .. }
    )
}
