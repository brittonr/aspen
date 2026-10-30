// r[impl molten.consensus.live_operation_generator]
//! Pure seeded planning. Onix schedules at most the admitted concurrency in each wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Value {
    Integer { value: i64 },
    Null,
}

impl Value {
    pub fn from_option(value: Option<i64>) -> Self {
        value.map_or(Self::Null, |value| Self::Integer { value })
    }
    pub fn as_option(self) -> Option<i64> {
        match self {
            Self::Integer { value } => Some(value),
            Self::Null => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Function {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Control,
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedOperation {
    pub id: String,
    pub process: String,
    pub function: Function,
    pub value: Value,
    pub endpoint: String,
    pub phase: Phase,
    pub object_key: String,
}

/// Every operation records three draws; client waves and final reads are deterministic projections.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub operation: u32,
    pub function_draw: u64,
    pub value_draw: u64,
    pub endpoint_draw: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationPlan {
    pub schema: String,
    pub generator_ref: String,
    pub adapter_ref: String,
    pub model_ref: String,
    pub outcome_ref: String,
    pub seed: u64,
    pub choices: Vec<Choice>,
    pub operations: Vec<PlannedOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanIssue {
    ChangedPlan,
    Bounds,
}

pub fn generate(admitted: &crate::live_reliability::profile::Admitted) -> Result<OperationPlan, PlanIssue> {
    let profile = admitted.profile();
    let mut state = profile.seed;
    let choices: Vec<Choice> = (0..profile.operations)
        .map(|index| Choice {
            operation: index,
            function_draw: draw(&mut state),
            value_draw: draw(&mut state),
            endpoint_draw: draw(&mut state),
        })
        .collect();
    let operations = choices.iter().map(|choice| operation(admitted, choice)).collect::<Result<Vec<_>, _>>()?;
    Ok(OperationPlan {
        schema: "onixos.live-operation-plan.v1".into(),
        generator_ref: profile.generator_ref.clone(),
        adapter_ref: profile.adapter_ref.clone(),
        model_ref: profile.model_ref.clone(),
        outcome_ref: profile.outcome_ref.clone(),
        seed: profile.seed,
        choices,
        operations,
    })
}

pub fn validate_plan(
    admitted: &crate::live_reliability::profile::Admitted,
    plan: &OperationPlan,
) -> Result<(), PlanIssue> {
    if &generate(admitted)? == plan {
        Ok(())
    } else {
        Err(PlanIssue::ChangedPlan)
    }
}

fn operation(
    admitted: &crate::live_reliability::profile::Admitted,
    choice: &Choice,
) -> Result<PlannedOperation, PlanIssue> {
    let profile = admitted.profile();
    let weight = std::num::NonZeroU64::new(u64::from(profile.read_weight) + u64::from(profile.write_weight))
        .ok_or(PlanIssue::Bounds)?;
    let final_start = profile.operations - profile.endpoints.len() as u32;
    let final_index = choice.operation.checked_sub(final_start);
    let function = if final_index.is_some() || choice.function_draw % weight < u64::from(profile.read_weight) {
        Function::Read
    } else {
        Function::Write
    };
    let value = match function {
        Function::Read => Value::Null,
        Function::Write => Value::Integer {
            value: *pick(&profile.values, choice.value_draw)?,
        },
    };
    // A wave rotates one admitted client window; all operations in that wave have distinct processes.
    let concurrency = std::num::NonZeroU32::new(profile.concurrency).ok_or(PlanIssue::Bounds)?;
    let wave = choice.operation / concurrency;
    let offset = choice.operation % concurrency;
    let endpoint = match final_index {
        Some(index) => profile
            .endpoints
            .get(usize::try_from(index).map_err(|_| PlanIssue::Bounds)?)
            .ok_or(PlanIssue::Bounds)?,
        None => pick(&profile.endpoints, choice.endpoint_draw)?,
    };
    Ok(PlannedOperation {
        id: format!("{}:operation:{}", admitted.profile_ref(), choice.operation),
        process: pick(&profile.clients, u64::from(wave) + u64::from(offset))?.clone(),
        function,
        value,
        endpoint: endpoint.clone(),
        phase: if choice.operation < profile.control_operations {
            Phase::Control
        } else {
            Phase::Fault
        },
        object_key: profile.keys.first().ok_or(PlanIssue::Bounds)?.clone(),
    })
}

/// Selects `items[draw mod len]`; the admitted profile guarantees non-empty, bounded collections.
fn pick<T>(items: &[T], draw: u64) -> Result<&T, PlanIssue> {
    let length = u64::try_from(items.len()).ok().and_then(std::num::NonZeroU64::new).ok_or(PlanIssue::Bounds)?;
    let index = usize::try_from(draw % length).map_err(|_| PlanIssue::Bounds)?;
    items.get(index).ok_or(PlanIssue::Bounds)
}

// SplitMix64: explicit wrapping arithmetic, including seed zero; no ambient RNG state.
fn draw(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
