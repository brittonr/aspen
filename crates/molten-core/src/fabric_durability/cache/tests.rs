use super::*;

const CAPACITY: u32 = 3;
const HIGH_WATERMARK: u32 = 3;
const LOW_WATERMARK: u32 = 1;
const STRICT_LRU_THRESHOLD: u8 = 0;
const LAZY_THRESHOLD: u8 = 2;
const FIFO_THRESHOLD_TEST: u8 = 100;
const DUPLICATE_COUNT: u32 = 2;

fn policy(threshold: u8) -> Policy {
    Policy::new(CAPACITY, HIGH_WATERMARK, LOW_WATERMARK, threshold).expect("valid policy")
}

fn projection<'a>(
    dataspace_identity: &'a str,
    normalized_arguments: &'a [String],
    capability_context: Option<&'a str>,
) -> AccessProjection<'a> {
    AccessProjection {
        dataspace_identity,
        normalized_arguments,
        capability_context,
    }
}

// r[verify aspen.dataspace_access_cache.projection]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn equal_accesses_project_to_equal_keys_and_distinct_accesses_do_not() {
    let arguments = vec!["subject=alpha".to_string(), "operation=observe".to_string()];
    let equal_left = project_key(&projection("dataspace:main", &arguments, Some("capability:read"))).expect("left key");
    let equal_right =
        project_key(&projection("dataspace:main", &arguments, Some("capability:read"))).expect("right key");
    let distinct =
        project_key(&projection("dataspace:other", &arguments, Some("capability:read"))).expect("distinct key");

    assert_eq!(equal_left, equal_right);
    assert_ne!(equal_left, distinct);
    assert!(equal_left.starts_with("blake3:"));
}

#[test]
fn capability_context_changes_the_key_and_ambient_state_is_absent() {
    let arguments = vec!["subject=alpha".to_string()];
    let reader = project_key(&projection("dataspace:main", &arguments, Some("capability:read"))).expect("reader key");
    let writer = project_key(&projection("dataspace:main", &arguments, Some("capability:write"))).expect("writer key");

    assert_ne!(reader, writer);
}

// r[verify aspen.dataspace_access_cache.bound]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn capacity_watermarks_and_thresholds_fail_closed() {
    assert_eq!(Policy::new(0, 0, 0, STRICT_LRU_THRESHOLD), Err(Issue::MissingCapacity));
    assert_eq!(
        Policy::new(CAPACITY, LOW_WATERMARK, HIGH_WATERMARK, STRICT_LRU_THRESHOLD),
        Err(Issue::InvalidWatermarks)
    );
    assert_eq!(
        Policy::new(CAPACITY, HIGH_WATERMARK, LOW_WATERMARK, FIFO_THRESHOLD_TEST + 1),
        Err(Issue::InvalidPromotionThreshold)
    );
}

// r[verify aspen.dataspace_access_cache.decision]
// r[verify aspen.dataspace_access_cache.verification]
#[test]
fn promotion_threshold_covers_strict_lazy_and_fifo_boundaries() {
    assert_eq!(decide_promotion(policy(STRICT_LRU_THRESHOLD), 0), Promotion::Promote);
    assert_eq!(decide_promotion(policy(LAZY_THRESHOLD), 1), Promotion::Retain);
    assert_eq!(decide_promotion(policy(LAZY_THRESHOLD), 2), Promotion::Promote);
    assert_eq!(decide_promotion(policy(FIFO_THRESHOLD_TEST), u32::MAX), Promotion::Retain);
}

#[test]
fn insertion_at_high_watermark_trims_to_low_before_insertion() {
    let order = vec!["oldest".to_string(), "middle".to_string(), "newest".to_string()];
    let plan = plan_insertion(&InsertionInput {
        policy: policy(STRICT_LRU_THRESHOLD),
        active_count: CAPACITY,
        eviction_order: &order,
    })
    .expect("insertion plan");

    assert_eq!(plan.evict_keys, vec!["oldest".to_string(), "middle".to_string()]);
    assert_eq!(plan.active_after_insert, LOW_WATERMARK + 1);
}

#[test]
fn single_slot_policy_evicts_one_and_remains_bounded() {
    const SINGLE_CAPACITY: u32 = 1;
    const SINGLE_HIGH: u32 = 1;
    const SINGLE_LOW: u32 = 0;
    let single = Policy::new(SINGLE_CAPACITY, SINGLE_HIGH, SINGLE_LOW, STRICT_LRU_THRESHOLD).expect("single policy");
    let order = vec!["only".to_string()];
    let plan = plan_insertion(&InsertionInput {
        policy: single,
        active_count: SINGLE_CAPACITY,
        eviction_order: &order,
    })
    .expect("single insertion");

    assert_eq!(plan.evict_keys, order);
    assert_eq!(plan.active_after_insert, SINGLE_CAPACITY);
}

#[test]
fn malformed_projection_and_eviction_order_are_denied() {
    let arguments = vec!["subject=alpha".to_string()];
    assert_eq!(project_key(&projection("", &arguments, None)), Err(Issue::EmptyDataspaceIdentity));
    assert_eq!(project_key(&projection("dataspace:main", &arguments, Some(""))), Err(Issue::EmptyCapabilityContext));

    let duplicate = vec!["same".to_string(), "same".to_string()];
    assert_eq!(
        plan_insertion(&InsertionInput {
            policy: policy(STRICT_LRU_THRESHOLD),
            active_count: DUPLICATE_COUNT,
            eviction_order: &duplicate,
        }),
        Err(Issue::DuplicateEvictionKey)
    );
}
