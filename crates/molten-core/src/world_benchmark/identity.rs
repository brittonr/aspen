use super::*;

pub fn plan_world_benchmark(
    profile: &WorldBenchmarkProfile,
    current_source_revision: &str,
) -> Result<WorldBenchmarkPlan, Vec<WorldBenchmarkIssue>> {
    let issues = validate_world_benchmark_profile(profile, current_source_revision);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut adapters = profile.adapters.clone();
    adapters.sort();
    let mut thresholds = profile.thresholds.clone();
    thresholds.sort_by(|left, right| left.name.cmp(&right.name));
    let plan_ref = identify_world_benchmark_plan(profile, &adapters, &thresholds).map_err(|issue| vec![issue])?;
    Ok(WorldBenchmarkPlan {
        schema: WORLD_BENCHMARK_PLAN_SCHEMA.to_string(),
        plan_ref,
        profile_ref: profile.profile_ref.clone(),
        source_revision: profile.source_revision.clone(),
        dataset_ref: profile.dataset_ref.clone(),
        class: profile.class,
        preparation: profile.preparation,
        operations: profile.operations.clone(),
        repetitions: profile.repetitions,
        adapters,
        hardware_cohort: profile.hardware_cohort.clone(),
        bounds: profile.bounds.clone(),
        thresholds,
        non_claims: world_benchmark_non_claims(),
    })
}

pub fn identify_world_benchmark_receipt(receipt: &WorldBenchmarkReceipt) -> Result<String, WorldBenchmarkIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_BENCHMARK_RECEIPT_DOMAIN);
    update_text(&mut hasher, &receipt.plan_ref)?;
    update_text(&mut hasher, &receipt.consumer_id)?;
    update_text(&mut hasher, &receipt.profile_ref)?;
    update_text(&mut hasher, &receipt.source_revision)?;
    update_text(&mut hasher, &receipt.dataset_ref)?;
    update_text(&mut hasher, receipt.preparation.as_str())?;
    update_text(&mut hasher, receipt.class.as_str())?;
    for adapter in &receipt.adapters {
        update_text(&mut hasher, adapter)?;
    }
    update_text(&mut hasher, &receipt.hardware_cohort)?;
    update_bounds(&mut hasher, &receipt.bounds);
    for result in &receipt.results {
        update_text(&mut hasher, result.operation.as_str())?;
        update_number(&mut hasher, u64::from(result.repetition));
        update_text(&mut hasher, &result.adapter_ref)?;
        for metric in &result.metrics {
            update_text(&mut hasher, metric.kind.as_str())?;
            update_number(&mut hasher, metric.value);
        }
        update_optional_number(&mut hasher, result.duration_nanoseconds);
        update_optional_number(&mut hasher, result.peak_memory_bytes);
        update_snapshot(&mut hasher, result.snapshot.as_ref())?;
        update_bool(&mut hasher, result.physical_measurement_independent);
    }
    for threshold in &receipt.threshold_results {
        update_text(&mut hasher, &threshold.name)?;
        update_text(&mut hasher, threshold.metric.as_str())?;
        update_number(&mut hasher, threshold.observed_maximum);
        update_number(&mut hasher, threshold.admitted_maximum);
        update_bool(&mut hasher, threshold.passed);
    }
    for row in &receipt.unsupported_rows {
        update_text(&mut hasher, row.operation.as_str())?;
        update_text(&mut hasher, &row.reason)?;
    }
    update_bool(&mut hasher, receipt.accepted);
    for non_claim in &receipt.non_claims {
        update_text(&mut hasher, non_claim)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn identify_world_benchmark_plan(
    profile: &WorldBenchmarkProfile,
    adapters: &[String],
    thresholds: &[WorldBenchmarkThreshold],
) -> Result<String, WorldBenchmarkIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_BENCHMARK_PLAN_DOMAIN);
    update_text(&mut hasher, &profile.profile_ref)?;
    update_text(&mut hasher, &profile.source_revision)?;
    update_text(&mut hasher, &profile.dataset_ref)?;
    update_text(&mut hasher, profile.preparation.as_str())?;
    update_text(&mut hasher, profile.class.as_str())?;
    for adapter in adapters {
        update_text(&mut hasher, adapter)?;
    }
    for operation in &profile.operations {
        update_text(&mut hasher, operation.as_str())?;
    }
    update_bounds(&mut hasher, &profile.bounds);
    update_number(&mut hasher, u64::from(profile.repetitions));
    update_text(&mut hasher, &profile.hardware_cohort)?;
    for threshold in thresholds {
        update_text(&mut hasher, &threshold.name)?;
        update_text(&mut hasher, threshold.metric.as_str())?;
        update_number(&mut hasher, threshold.maximum);
        match threshold.operation {
            Some(operation) => update_text(&mut hasher, operation.as_str())?,
            None => update_text(&mut hasher, "all")?,
        }
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_bounds(hasher: &mut blake3::Hasher, bounds: &WorldBenchmarkBounds) {
    update_number(hasher, u64::from(bounds.max_operations));
    update_number(hasher, u64::from(bounds.max_repetitions));
    update_number(hasher, bounds.max_logical_bytes);
    update_number(hasher, bounds.max_physical_bytes);
    update_number(hasher, bounds.max_objects);
    update_number(hasher, bounds.max_pages);
    update_number(hasher, bounds.max_references);
    update_number(hasher, bounds.max_keys);
    update_number(hasher, bounds.max_conflicts);
    update_number(hasher, bounds.max_duration_nanoseconds);
    update_number(hasher, bounds.max_peak_memory_bytes);
}

fn update_snapshot(
    hasher: &mut blake3::Hasher,
    snapshot: Option<&WorldBenchmarkSnapshotBinding>,
) -> Result<(), WorldBenchmarkIssue> {
    match snapshot {
        Some(snapshot) => {
            update_bool(hasher, true);
            update_text(hasher, &snapshot.descriptor_ref)?;
            update_text(hasher, &snapshot.source_revision)?;
            update_text(hasher, &snapshot.completeness_profile)?;
            update_number(hasher, snapshot.memory_bytes);
            update_number(hasher, snapshot.closure_members);
        }
        None => update_bool(hasher, false),
    }
    Ok(())
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<(), WorldBenchmarkIssue> {
    let length = u64::try_from(value.len()).map_err(|_| WorldBenchmarkIssue::InvalidReference("identity_length"))?;
    update_number(hasher, length);
    hasher.update(value.as_bytes());
    Ok(())
}

fn update_number(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

fn update_optional_number(hasher: &mut blake3::Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            update_bool(hasher, true);
            update_number(hasher, value);
        }
        None => update_bool(hasher, false),
    }
}

fn update_bool(hasher: &mut blake3::Hasher, value: bool) {
    hasher.update(&[u8::from(value)]);
}
