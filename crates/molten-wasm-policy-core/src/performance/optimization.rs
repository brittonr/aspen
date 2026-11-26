//! Reviewed Wasmtime optimization profiles: shape, capacity, and deterministic conformance.

use alloc::string::ToString;

pub const BASELINE_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.baseline.v1";
pub const POOLING_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.pooling.v1";
pub const COW_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.cow.v1";
pub const INSTANCE_PRE_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.instance-pre.v1";

pub fn validate_profile(
    performance_profile: &super::model::Profile,
    optimization: &super::model::OptimizationProfile,
) -> super::model::Result<()> {
    super::profile::validate(performance_profile)?;
    let mut blockers = alloc::vec::Vec::new();
    if !performance_profile
        .optimization_limits
        .reviewed_profile_ids
        .iter()
        .any(|profile_id| profile_id == &optimization.profile_id)
    {
        blockers.push("Wasm optimization profile id is not reviewed".to_string());
    }
    if optimization.max_concurrency == 0
        || optimization.max_concurrency > performance_profile.optimization_limits.max_concurrency
        || optimization.max_queue_depth == 0
        || optimization.max_queue_depth > performance_profile.optimization_limits.max_queue_depth
        || optimization.max_pool_memories == 0
        || optimization.max_pool_memories > performance_profile.optimization_limits.max_pool_memories
        || optimization.max_pool_tables == 0
        || optimization.max_pool_tables > performance_profile.optimization_limits.max_pool_tables
    {
        blockers.push("Wasm optimization profile exceeds a reviewed resource or capacity bound".to_string());
    }
    if !super::model::valid_content_ref(&optimization.deterministic_conformance_ref) {
        blockers.push("Wasm optimization profile lacks a deterministic conformance receipt".to_string());
    }
    validate_named_shape(optimization, &mut blockers);
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::Denial::from_blockers(blockers))
    }
}

pub fn profile_ref(optimization: &super::model::OptimizationProfile) -> alloc::string::String {
    let lines = [
        alloc::format!("configuration-ref:{}", configuration_ref(optimization)),
        alloc::format!("deterministic-conformance-ref:{}", optimization.deterministic_conformance_ref),
    ];
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn configuration_ref(optimization: &super::model::OptimizationProfile) -> alloc::string::String {
    let lines = [
        alloc::format!("profile-id:{}", optimization.profile_id),
        alloc::format!("pooling:{}", optimization.pooling_allocator),
        alloc::format!("copy-on-write:{}", optimization.copy_on_write_heap_images),
        alloc::format!("instance-pre:{}", optimization.instance_pre),
        alloc::format!("compilation-strategy:{}", optimization.compilation_strategy.as_str()),
        alloc::format!("max-concurrency:{}", optimization.max_concurrency),
        alloc::format!("max-queue-depth:{}", optimization.max_queue_depth),
        alloc::format!("max-pool-memories:{}", optimization.max_pool_memories),
        alloc::format!("max-pool-tables:{}", optimization.max_pool_tables),
    ];
    super::model::content_ref(lines.join("\n").as_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceRecord {
    pub record_ref: alloc::string::String,
    pub optimization_configuration_ref: alloc::string::String,
    pub component_profile_ref: alloc::string::String,
    pub input_ref: alloc::string::String,
    pub baseline_output_ref: alloc::string::String,
    pub optimized_output_ref: alloc::string::String,
    pub baseline_execution_receipt_ref: alloc::string::String,
    pub optimized_execution_receipt_ref: alloc::string::String,
    pub baseline_terminal_class: alloc::string::String,
    pub optimized_terminal_class: alloc::string::String,
    pub recorded_effect_refs: alloc::vec::Vec<alloc::string::String>,
    pub passed: bool,
}

pub fn conformance_record_ref(record: &ConformanceRecord) -> alloc::string::String {
    let mut lines = alloc::vec![
        alloc::format!("optimization-configuration-ref:{}", record.optimization_configuration_ref),
        alloc::format!("component-profile-ref:{}", record.component_profile_ref),
        alloc::format!("input-ref:{}", record.input_ref),
        alloc::format!("baseline-output-ref:{}", record.baseline_output_ref),
        alloc::format!("optimized-output-ref:{}", record.optimized_output_ref),
        alloc::format!("baseline-execution-receipt-ref:{}", record.baseline_execution_receipt_ref),
        alloc::format!("optimized-execution-receipt-ref:{}", record.optimized_execution_receipt_ref),
        alloc::format!("baseline-terminal-class:{}", record.baseline_terminal_class),
        alloc::format!("optimized-terminal-class:{}", record.optimized_terminal_class),
        alloc::format!("passed:{}", record.passed),
    ];
    lines.extend(
        super::model::sorted_unique(&record.recorded_effect_refs)
            .into_iter()
            .map(|value| alloc::format!("recorded-effect-ref:{value}")),
    );
    super::model::content_ref(lines.join("\n").as_bytes())
}

/// Validates a deterministic conformance record against the reviewed component runtime profile
/// the conformance run targeted; the shell supplies its embedded supported component profile.
pub fn validate_conformance(
    optimization: &super::model::OptimizationProfile,
    record: &ConformanceRecord,
    component_profile: &crate::component::model::RuntimeProfile,
) -> super::model::Result<()> {
    let expected_component_profile_ref = crate::component::profile::r#ref(component_profile);
    let mut blockers = alloc::vec::Vec::new();
    if record.record_ref != conformance_record_ref(record)
        || record.record_ref != optimization.deterministic_conformance_ref
        || record.optimization_configuration_ref != configuration_ref(optimization)
    {
        blockers.push("optimization conformance record is stale or bound to another profile".to_string());
    }
    if record.component_profile_ref != expected_component_profile_ref {
        blockers.push("optimization conformance targets a stale component runtime profile".to_string());
    }
    blockers.extend(
        [
            ("component profile", record.component_profile_ref.as_str()),
            ("input", record.input_ref.as_str()),
            ("baseline output", record.baseline_output_ref.as_str()),
            ("optimized output", record.optimized_output_ref.as_str()),
            ("baseline execution receipt", record.baseline_execution_receipt_ref.as_str()),
            ("optimized execution receipt", record.optimized_execution_receipt_ref.as_str()),
        ]
        .into_iter()
        .filter(|(_, value)| !super::model::valid_content_ref(value))
        .map(|(label, _)| alloc::format!("optimization conformance {label} ref is malformed")),
    );
    if !super::model::valid_ref_collection(&record.recorded_effect_refs) {
        blockers.push("optimization conformance recorded-effect refs are missing, duplicate, or malformed".to_string());
    }
    if !record.passed
        || record.baseline_output_ref != record.optimized_output_ref
        || record.baseline_terminal_class != record.optimized_terminal_class
        || record.baseline_terminal_class.trim().is_empty()
    {
        blockers.push("optimization conformance does not preserve output and terminal classification".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::Denial::from_blockers(blockers))
    }
}

pub fn admit_capacity(
    optimization: &super::model::OptimizationProfile,
    running: u32,
    queued: u32,
) -> super::model::CapacityDecision {
    if running < optimization.max_concurrency {
        super::model::CapacityDecision::Start
    } else if queued < optimization.max_queue_depth {
        super::model::CapacityDecision::Backpressure
    } else {
        super::model::CapacityDecision::Deny
    }
}

fn validate_named_shape(
    optimization: &super::model::OptimizationProfile,
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) {
    let is_shape_matches_name = match optimization.profile_id.as_str() {
        BASELINE_OPTIMIZATION_PROFILE_ID => {
            !optimization.pooling_allocator
                && !optimization.copy_on_write_heap_images
                && !optimization.instance_pre
                && optimization.compilation_strategy == super::model::CompilationStrategy::Cranelift
        }
        POOLING_OPTIMIZATION_PROFILE_ID => {
            optimization.pooling_allocator
                && !optimization.copy_on_write_heap_images
                && !optimization.instance_pre
                && optimization.compilation_strategy == super::model::CompilationStrategy::Cranelift
        }
        COW_OPTIMIZATION_PROFILE_ID => {
            optimization.copy_on_write_heap_images
                && !optimization.pooling_allocator
                && !optimization.instance_pre
                && optimization.compilation_strategy == super::model::CompilationStrategy::Cranelift
        }
        INSTANCE_PRE_OPTIMIZATION_PROFILE_ID => {
            optimization.instance_pre
                && !optimization.pooling_allocator
                && !optimization.copy_on_write_heap_images
                && optimization.compilation_strategy == super::model::CompilationStrategy::Cranelift
        }
        _ => true,
    };
    if !is_shape_matches_name {
        blockers.push("Wasm optimization profile name and enabled knobs disagree".to_string());
    }
}

/// Whether the optimization limits equal the reviewed hard caps and reviewed optimization profile
/// ids.
pub(super) fn limits_match_reviewed_caps(optimization: &super::model::OptimizationLimits) -> bool {
    optimization.max_concurrency == super::profile::MAX_OPTIMIZATION_CONCURRENCY
        && optimization.max_queue_depth == super::profile::MAX_OPTIMIZATION_QUEUE_DEPTH
        && optimization.max_pool_memories == super::profile::MAX_POOL_MEMORIES
        && optimization.max_pool_tables == super::profile::MAX_POOL_TABLES
        && optimization.reviewed_profile_ids.iter().eq(super::profile::REVIEWED_OPTIMIZATION_PROFILE_IDS)
}
