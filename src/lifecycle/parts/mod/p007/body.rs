// Shared service-state assertion vocabulary.
//
// The lifecycle state machine stays the transition authority. These assertions
// are a projection of committed state over one mapping table: `ready` implies
// `started`, `complete` reports a normal one-shot finish, `failed` reports a
// failure, and a user-defined value satisfies no requirement that names a
// built-in state. The projection never drives a transition and grants no
// authority.

const MAX_SERVICE_ASSERTIONS: usize = 32;
const MAX_USER_DEFINED_BYTES: usize = 128;

impl State {
    pub fn from_as_str(value: &str) -> Option<Self> {
        match value {
            "declared" => Some(Self::Declared),
            "spawning" => Some(Self::Spawning),
            "starting" => Some(Self::Starting),
            "ready" => Some(Self::Ready),
            "degraded" => Some(Self::Degraded),
            "stopping" => Some(Self::Stopping),
            "stopped" => Some(Self::Stopped),
            "failed" => Some(Self::Failed),
            "restarting" => Some(Self::Restarting),
            "cleaned" => Some(Self::Cleaned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceStateAssertion {
    Started,
    Ready,
    Complete,
    Failed,
    UserDefined(String),
}

impl ServiceStateAssertion {
    pub fn as_str(&self) -> String {
        match self {
            Self::Started => "started".to_string(),
            Self::Ready => "ready".to_string(),
            Self::Complete => "complete".to_string(),
            Self::Failed => "failed".to_string(),
            Self::UserDefined(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRequirement {
    Ready,
    Complete,
    Up,
}

impl ServiceRequirement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Complete => "complete",
            Self::Up => "up",
        }
    }

    pub fn satisfied_by(self, assertions: &ServiceAssertionSet) -> bool {
        match self {
            Self::Ready => assertions.contains_ready(),
            Self::Complete => assertions.contains_complete(),
            Self::Up => assertions.is_up(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceAssertionSet {
    values: Vec<ServiceStateAssertion>,
}

impl ServiceAssertionSet {
    pub fn from_values(values: Vec<ServiceStateAssertion>) -> Result<Self> {
        if values.len() > MAX_SERVICE_ASSERTIONS {
            return Err(MoltenError::invalid_harness(format!(
                "service assertion count {} exceeds {MAX_SERVICE_ASSERTIONS}",
                values.len()
            )));
        }
        let mut ordered = values;
        ordered.sort();
        ordered.dedup();
        Ok(Self { values: ordered })
    }

    pub fn values(&self) -> &[ServiceStateAssertion] {
        &self.values
    }

    pub fn contains(&self, assertion: &ServiceStateAssertion) -> bool {
        self.values.contains(assertion)
    }

    pub fn contains_ready(&self) -> bool {
        self.values.contains(&ServiceStateAssertion::Ready)
    }

    pub fn contains_complete(&self) -> bool {
        self.values.contains(&ServiceStateAssertion::Complete)
    }

    pub fn is_up(&self) -> bool {
        self.contains_ready() || self.contains_complete()
    }

    pub fn contains_user_defined(&self, value: &str) -> bool {
        self.values
            .iter()
            .any(|assertion| matches!(assertion, ServiceStateAssertion::UserDefined(name) if name == value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceReadinessOutcome {
    Ready,
    Failed,
}

impl ServiceReadinessOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceAssertionInput<'a> {
    pub lifecycle_state: Option<State>,
    pub extension_phase: Option<&'a str>,
    pub readiness: Option<ServiceReadinessOutcome>,
    pub completed_normally: bool,
    pub user_defined: &'a [String],
}

// r[impl molten.runtime_spine.service_state_union]
pub fn project_service_assertions(input: &ServiceAssertionInput<'_>) -> Result<ServiceAssertionSet> {
    let mut values = Vec::with_capacity(4 + input.user_defined.len());
    let mut rows: Vec<&'static ServiceStateMappingRow> = Vec::new();
    if let Some(state) = input.lifecycle_state {
        let row = mapping_row(ServiceStateVocabulary::Lifecycle, state.as_str()).ok_or_else(|| {
            MoltenError::invalid_harness(format!("unknown lifecycle state {}", state.as_str()))
        })?;
        rows.push(row);
    }
    if let Some(phase) = input.extension_phase {
        let row = mapping_row(ServiceStateVocabulary::Extension, phase).ok_or_else(|| {
            MoltenError::invalid_harness(format!("unknown system-extension phase {phase}"))
        })?;
        rows.push(row);
    }
    if let Some(readiness) = input.readiness {
        let row = mapping_row(ServiceStateVocabulary::Readiness, readiness.as_str()).ok_or_else(|| {
            MoltenError::invalid_harness(format!("unknown readiness outcome {}", readiness.as_str()))
        })?;
        rows.push(row);
    }
    let mut is_started = false;
    let mut is_ready = false;
    let mut is_failed = false;
    let mut is_complete = false;
    for row in rows {
        let (row_started, row_ready, row_failed, row_complete) = row_assertions(row, input.completed_normally);
        is_started |= row_started;
        is_ready |= row_ready;
        is_failed |= row_failed;
        is_complete |= row_complete;
    }
    if is_ready {
        is_started = true;
    }
    values.extend(assertion_values(is_started, is_ready, is_complete, is_failed));
    for value in input.user_defined {
        validate_non_empty(value, "user-defined service state")?;
        if value.len() > MAX_USER_DEFINED_BYTES {
            return Err(MoltenError::invalid_harness(format!(
                "user-defined service state exceeds {MAX_USER_DEFINED_BYTES} bytes"
            )));
        }
        values.push(ServiceStateAssertion::UserDefined(value.clone()));
    }
    ServiceAssertionSet::from_values(values)
}

fn assertion_values(
    is_started: bool,
    is_ready: bool,
    is_complete: bool,
    is_failed: bool,
) -> Vec<ServiceStateAssertion> {
    let mut values = Vec::with_capacity(4);
    if is_started {
        values.push(ServiceStateAssertion::Started);
    }
    if is_ready {
        values.push(ServiceStateAssertion::Ready);
    }
    if is_complete {
        values.push(ServiceStateAssertion::Complete);
    }
    if is_failed {
        values.push(ServiceStateAssertion::Failed);
    }
    values
}

fn row_assertions(
    row: &ServiceStateMappingRow,
    completed_normally: bool,
) -> (bool, bool, bool, bool) {
    let is_ready = row.is_ready();
    (
        row.is_started() || is_ready,
        is_ready,
        row.is_failed(),
        row.is_normal_finish() && completed_normally,
    )
}

/// Project a production readiness or service-status state into the shared set.
///
/// A state outside the readiness vocabulary reports no built-in assertion, so
/// it never satisfies a dependency that names one.
pub fn project_readiness_state(state: &str) -> Result<ServiceAssertionSet> {
    let Some(row) = mapping_row(ServiceStateVocabulary::Readiness, state) else {
        return ServiceAssertionSet::from_values(Vec::new());
    };
    let (is_started, is_ready, is_failed, is_complete) = row_assertions(row, false);
    ServiceAssertionSet::from_values(assertion_values(is_started, is_ready, is_complete, is_failed))
}
