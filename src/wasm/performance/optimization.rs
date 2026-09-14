pub const BASELINE_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.baseline.v1";
pub const POOLING_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.pooling.v1";
pub const COW_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.cow.v1";
pub const INSTANCE_PRE_OPTIMIZATION_PROFILE_ID: &str = "molten.wasm.optimization.instance-pre.v1";

pub fn validate_optimization_profile(
    performance_profile: &super::model::PerformanceProfile,
    optimization: &super::model::OptimizationProfile,
) -> super::model::PerformanceResult<()> {
    super::profile::validate_performance_profile(performance_profile)?;
    let mut blockers = Vec::new();
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
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

pub fn optimization_profile_ref(optimization: &super::model::OptimizationProfile) -> String {
    let lines = [
        format!("configuration-ref:{}", optimization_configuration_ref(optimization)),
        format!("deterministic-conformance-ref:{}", optimization.deterministic_conformance_ref),
    ];
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn optimization_configuration_ref(optimization: &super::model::OptimizationProfile) -> String {
    let lines = [
        format!("profile-id:{}", optimization.profile_id),
        format!("pooling:{}", optimization.pooling_allocator),
        format!("copy-on-write:{}", optimization.copy_on_write_heap_images),
        format!("instance-pre:{}", optimization.instance_pre),
        format!("compilation-strategy:{}", optimization.compilation_strategy.as_str()),
        format!("max-concurrency:{}", optimization.max_concurrency),
        format!("max-queue-depth:{}", optimization.max_queue_depth),
        format!("max-pool-memories:{}", optimization.max_pool_memories),
        format!("max-pool-tables:{}", optimization.max_pool_tables),
    ];
    super::model::content_ref(lines.join("\n").as_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationConformanceRecord {
    pub record_ref: String,
    pub optimization_configuration_ref: String,
    pub component_profile_ref: String,
    pub input_ref: String,
    pub baseline_output_ref: String,
    pub optimized_output_ref: String,
    pub baseline_execution_receipt_ref: String,
    pub optimized_execution_receipt_ref: String,
    pub baseline_terminal_class: String,
    pub optimized_terminal_class: String,
    pub recorded_effect_refs: Vec<String>,
    pub passed: bool,
}

pub fn optimization_conformance_record_ref(record: &OptimizationConformanceRecord) -> String {
    let mut lines = vec![
        format!("optimization-configuration-ref:{}", record.optimization_configuration_ref),
        format!("component-profile-ref:{}", record.component_profile_ref),
        format!("input-ref:{}", record.input_ref),
        format!("baseline-output-ref:{}", record.baseline_output_ref),
        format!("optimized-output-ref:{}", record.optimized_output_ref),
        format!("baseline-execution-receipt-ref:{}", record.baseline_execution_receipt_ref),
        format!("optimized-execution-receipt-ref:{}", record.optimized_execution_receipt_ref),
        format!("baseline-terminal-class:{}", record.baseline_terminal_class),
        format!("optimized-terminal-class:{}", record.optimized_terminal_class),
        format!("passed:{}", record.passed),
    ];
    lines.extend(
        super::model::sorted_unique(&record.recorded_effect_refs)
            .into_iter()
            .map(|value| format!("recorded-effect-ref:{value}")),
    );
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn validate_optimization_conformance(
    optimization: &super::model::OptimizationProfile,
    record: &OptimizationConformanceRecord,
) -> super::model::PerformanceResult<()> {
    let component_profile = crate::wasm_component::supported_component_profile().map_err(|error| {
        super::model::PerformanceDenial::new(format!(
            "component profile required by optimization conformance is invalid: {error}"
        ))
    })?;
    let expected_component_profile_ref = crate::wasm_component::component_profile_ref(&component_profile);
    let mut blockers = Vec::new();
    if record.record_ref != optimization_conformance_record_ref(record)
        || record.record_ref != optimization.deterministic_conformance_ref
        || record.optimization_configuration_ref != optimization_configuration_ref(optimization)
    {
        blockers.push("optimization conformance record is stale or bound to another profile".to_string());
    }
    if record.component_profile_ref != expected_component_profile_ref {
        blockers.push("optimization conformance targets a stale component runtime profile".to_string());
    }
    for (label, value) in [
        ("component profile", record.component_profile_ref.as_str()),
        ("input", record.input_ref.as_str()),
        ("baseline output", record.baseline_output_ref.as_str()),
        ("optimized output", record.optimized_output_ref.as_str()),
        ("baseline execution receipt", record.baseline_execution_receipt_ref.as_str()),
        ("optimized execution receipt", record.optimized_execution_receipt_ref.as_str()),
    ] {
        if !super::model::valid_content_ref(value) {
            blockers.push(format!("optimization conformance {label} ref is malformed"));
        }
    }
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
        Err(super::model::PerformanceDenial::from_blockers(blockers))
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

fn validate_named_shape(optimization: &super::model::OptimizationProfile, blockers: &mut Vec<String>) {
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
