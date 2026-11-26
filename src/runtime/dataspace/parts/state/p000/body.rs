use super::*;

const RANDOM_XORSHIFT_RIGHT_A: u32 = 12;
const RANDOM_XORSHIFT_LEFT_B: u32 = 25;
const RANDOM_XORSHIFT_RIGHT_C: u32 = 27;
const RANDOM_XORSHIFT_MULTIPLIER: u64 = 0x2545_F491_4F6C_DD1D;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRecordedEffectTransition {
    pub after: RuntimeSnapshot,
    pub response: Event,
}

fn effect_response(effect: Effect, actor: String, sequence: u64, upper: Option<u64>, value: u64) -> Event {
    Event::EffectResponse {
        effect,
        actor,
        sequence,
        upper,
        value,
    }
}

fn deterministic_random_step(rng_state: u64, upper: u64) -> (u64, u64) {
    let mut next_state = rng_state;
    next_state ^= next_state >> RANDOM_XORSHIFT_RIGHT_A;
    next_state ^= next_state << RANDOM_XORSHIFT_LEFT_B;
    next_state ^= next_state >> RANDOM_XORSHIFT_RIGHT_C;
    let mixed = next_state.wrapping_mul(RANDOM_XORSHIFT_MULTIPLIER);
    let value = if upper == 0 { 0 } else { mixed % upper };
    (next_state, value)
}

fn observe_value_matches(pattern_value: &Value, candidate: &Value) -> bool {
    RuntimePattern::from_observe_value(pattern_value)
        .and_then(|pattern| pattern.matches_value(candidate).map(|(is_match, _bindings)| is_match))
        .unwrap_or(false)
}

/// The successor of a runtime counter, or a typed failure when the successor
/// would reach `u64::MAX`.
///
/// `RuntimeSnapshot::validate_for_restore` rejects an exhausted counter, so a
/// live or replayed transition must never produce one: every committed
/// snapshot stays restorable.
fn next_restorable_counter(current: u64, counter: &str) -> Result<u64> {
    current.checked_add(1).filter(|next| *next < u64::MAX).ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "runtime {counter} is exhausted; advancing past {current} would leave an unrestorable snapshot"
        ))
    })
}

/// The admitted random value range is `0..upper`; an empty range admits no value.
fn validate_random_value(upper: u64, value: u64) -> Result<()> {
    if value < upper {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "random effect value {value} is outside the admitted range 0..{upper}"
        )))
    }
}

// r[impl molten.runtime_state_machine_proof.turn_commit_delta]
// r[impl molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
pub fn recorded_effect_response_transition(
    before: &RuntimeSnapshot,
    request: &Event,
    value: u64,
) -> Result<RuntimeRecordedEffectTransition> {
    match request {
        Event::EffectRequest {
            effect: Effect::Clock,
            actor,
            sequence,
            upper,
        } => {
            if value < before.logical_time {
                return Err(MoltenError::invalid_harness(format!(
                    "recorded clock value {value} regresses logical time {}",
                    before.logical_time
                )));
            }
            let mut after = before.clone();
            after.logical_time = next_restorable_counter(value, "logical time")?;
            Ok(RuntimeRecordedEffectTransition {
                after,
                response: effect_response(Effect::Clock, actor.clone(), *sequence, *upper, value),
            })
        }
        Event::EffectRequest {
            effect: Effect::Random,
            actor,
            sequence,
            upper: Some(upper),
        } => {
            validate_random_value(*upper, value)?;
            let mut after = before.clone();
            let (next_state, _ignored_local_value) = deterministic_random_step(after.rng_state, *upper);
            after.rng_state = next_state;
            Ok(RuntimeRecordedEffectTransition {
                after,
                response: effect_response(Effect::Random, actor.clone(), *sequence, Some(*upper), value),
            })
        }
        Event::EffectRequest {
            effect: Effect::Random, ..
        } => Err(MoltenError::invalid_harness("recorded random effect request missing upper bound")),
        _ => Err(MoltenError::invalid_harness("recorded effect response requires an effect request")),
    }
}
