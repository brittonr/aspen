//! Pure retention decisions for bounded dataspace-access caches.

const KEY_CONTEXT: &str = "molten.dataspace-access-cache-key.v1";
const MAX_CAPACITY: u32 = 1_048_576;
const MAX_PROJECTION_ARGUMENTS: usize = 256;
const MAX_PROJECTION_TEXT_BYTES: usize = 4_096;
const FIFO_THRESHOLD: u8 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    MissingCapacity,
    CapacityTooLarge,
    InvalidWatermarks,
    InvalidPromotionThreshold,
    EmptyDataspaceIdentity,
    EmptyCapabilityContext,
    ProjectionTextTooLarge,
    TooManyArguments,
    ActiveCountExceedsCapacity,
    EvictionOrderCountMismatch,
    DuplicateEvictionKey,
    CountRepresentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub capacity: u32,
    pub high_watermark: u32,
    pub low_watermark: u32,
    pub promotion_threshold: u8,
}

impl Policy {
    // r[impl aspen.dataspace_access_cache.bound]
    // r[impl aspen.dataspace_access_cache.decision]
    pub fn new(capacity: u32, high_watermark: u32, low_watermark: u32, promotion_threshold: u8) -> Result<Self, Issue> {
        if capacity == 0 {
            return Err(Issue::MissingCapacity);
        }
        if capacity > MAX_CAPACITY {
            return Err(Issue::CapacityTooLarge);
        }
        if high_watermark == 0 || high_watermark > capacity || low_watermark >= high_watermark {
            return Err(Issue::InvalidWatermarks);
        }
        if promotion_threshold > FIFO_THRESHOLD {
            return Err(Issue::InvalidPromotionThreshold);
        }
        Ok(Self {
            capacity,
            high_watermark,
            low_watermark,
            promotion_threshold,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessProjection<'a> {
    pub dataspace_identity: &'a str,
    pub normalized_arguments: &'a [String],
    pub capability_context: Option<&'a str>,
}

// r[impl aspen.dataspace_access_cache.projection]
pub fn project_key(input: &AccessProjection<'_>) -> Result<String, Issue> {
    validate_projection(input)?;
    let mut hasher = blake3::Hasher::new_derive_key(KEY_CONTEXT);
    hash_text(&mut hasher, "dataspace", input.dataspace_identity);
    hash_number(&mut hasher, "argument-count", input.normalized_arguments.len());
    for argument in input.normalized_arguments {
        hash_text(&mut hasher, "argument", argument);
    }
    hash_text(&mut hasher, "capability", input.capability_context.unwrap_or("none"));
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn validate_projection(input: &AccessProjection<'_>) -> Result<(), Issue> {
    if input.dataspace_identity.trim().is_empty() {
        return Err(Issue::EmptyDataspaceIdentity);
    }
    validate_text(input.dataspace_identity)?;
    if input.normalized_arguments.len() > MAX_PROJECTION_ARGUMENTS {
        return Err(Issue::TooManyArguments);
    }
    for argument in input.normalized_arguments {
        validate_text(argument)?;
    }
    if let Some(context) = input.capability_context {
        if context.trim().is_empty() {
            return Err(Issue::EmptyCapabilityContext);
        }
        validate_text(context)?;
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<(), Issue> {
    if value.len() > MAX_PROJECTION_TEXT_BYTES {
        Err(Issue::ProjectionTextTooLarge)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promotion {
    Promote,
    Retain,
}

// r[impl aspen.dataspace_access_cache.decision]
pub fn decide_promotion(policy: Policy, accesses_since_promotion: u32) -> Promotion {
    if policy.promotion_threshold == FIFO_THRESHOLD {
        return Promotion::Retain;
    }
    if accesses_since_promotion >= u32::from(policy.promotion_threshold) {
        Promotion::Promote
    } else {
        Promotion::Retain
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InsertionInput<'a> {
    pub policy: Policy,
    pub active_count: u32,
    pub eviction_order: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertionPlan {
    pub evict_keys: Vec<String>,
    pub active_after_insert: u32,
}

// r[impl aspen.dataspace_access_cache.decision]
// r[impl aspen.dataspace_access_cache.deferral]
pub fn plan_insertion(input: &InsertionInput<'_>) -> Result<InsertionPlan, Issue> {
    if input.active_count > input.policy.capacity {
        return Err(Issue::ActiveCountExceedsCapacity);
    }
    let active_count = usize::try_from(input.active_count).map_err(|_| Issue::CountRepresentation)?;
    if input.eviction_order.len() != active_count {
        return Err(Issue::EvictionOrderCountMismatch);
    }
    validate_unique_order(input.eviction_order)?;

    let retained_before_insert = retained_count(input.policy, input.active_count);
    let eviction_count = input.active_count - retained_before_insert;
    let eviction_count = usize::try_from(eviction_count).map_err(|_| Issue::CountRepresentation)?;
    let mut evict_keys = Vec::with_capacity(eviction_count);
    for key in input.eviction_order {
        if evict_keys.len() == eviction_count {
            break;
        }
        evict_keys.push(key.clone());
    }
    if evict_keys.len() != eviction_count {
        return Err(Issue::EvictionOrderCountMismatch);
    }

    Ok(InsertionPlan {
        evict_keys,
        active_after_insert: retained_before_insert + 1,
    })
}

fn retained_count(policy: Policy, active_count: u32) -> u32 {
    if active_count >= policy.high_watermark {
        policy.low_watermark
    } else if active_count >= policy.capacity {
        policy.capacity - 1
    } else {
        active_count
    }
}

fn validate_unique_order(keys: &[String]) -> Result<(), Issue> {
    let mut unique = std::collections::BTreeSet::new();
    for key in keys {
        if !unique.insert(key) {
            return Err(Issue::DuplicateEvictionKey);
        }
    }
    Ok(())
}

fn hash_number(hasher: &mut blake3::Hasher, label: &str, value: usize) {
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

#[cfg(test)]
mod tests;
