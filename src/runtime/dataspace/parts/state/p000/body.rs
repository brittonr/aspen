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

// r[impl molten.runtime_state_machine_proof.turn_commit_delta]
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
            let mut after = before.clone();
            after.logical_time = value + 1;
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
