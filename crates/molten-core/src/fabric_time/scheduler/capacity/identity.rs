const PLAN_CONTEXT: &str = "molten.fabric-time.scheduler-capacity-plan.v1";
const OBSERVATION_CONTEXT: &str = "molten.fabric-time.scheduler-capacity-observation.v1";

pub(super) struct PlanInput<'a> {
    pub profile_ref: &'a str,
    pub generation: u64,
    pub runnable_slots: u64,
    pub queue_slots: u64,
    pub concurrency_slots: u64,
    pub total_slots: u64,
}

pub(super) fn plan(input: &PlanInput<'_>) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(PLAN_CONTEXT);
    hash_text(&mut hasher, "profile", input.profile_ref);
    hash_number(&mut hasher, "generation", input.generation);
    hash_number(&mut hasher, "runnable-slots", input.runnable_slots);
    hash_number(&mut hasher, "queue-slots", input.queue_slots);
    hash_number(&mut hasher, "concurrency-slots", input.concurrency_slots);
    hash_number(&mut hasher, "total-slots", input.total_slots);
    format!("blake3:{}", hasher.finalize().to_hex())
}

pub(super) fn observation(state: &super::UseState) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(OBSERVATION_CONTEXT);
    hash_text(&mut hasher, "plan", &state.plan_ref);
    hash_text(&mut hasher, "profile", &state.profile_ref);
    hash_number(&mut hasher, "generation", state.generation);
    hash_number(&mut hasher, "runnable-usage", state.runnable_usage);
    hash_number(&mut hasher, "queue-usage", state.queue_usage);
    hash_number(&mut hasher, "runnable-high-water", state.runnable_high_water);
    hash_number(&mut hasher, "queue-high-water", state.queue_high_water);
    hash_number(&mut hasher, "exhaustions", state.exhaustion_count);
    hash_text(&mut hasher, "released", if state.is_released { "true" } else { "false" });
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn hash_number(hasher: &mut blake3::Hasher, label: &str, value: u64) {
    hash_text(hasher, label, &value.to_string());
}

fn hash_text(hasher: &mut blake3::Hasher, label: &str, value: &str) {
    hasher.update(label.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(label.as_bytes());
    hasher.update(value.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
}
