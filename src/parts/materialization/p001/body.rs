
/// Denies a member that uses a reserved top-level name, is not a regular file, has an invalid
/// content ref, or exceeds the per-member byte bound.
fn validate_member_input(
    policy: &MaterializationPolicy,
    reserved: &std::collections::BTreeSet<&str>,
    logical_path: &MaterializationPath,
    input: &MaterializationMemberInput,
) -> crate::error::Result<()> {
    if reserved.contains(logical_path.top_level()) {
        return Err(invalid(format!(
            "materialization member {} uses reserved top-level name {}",
            logical_path.as_str(),
            logical_path.top_level()
        )));
    }
    if input.kind != MaterializationMemberKind::RegularFile {
        return Err(invalid(format!(
            "materialization member {} has unsupported kind {}",
            logical_path.as_str(),
            input.kind.as_str()
        )));
    }
    crate::preserves_rail::validate_content_ref(&input.expected_content_ref)
        .map_err(|error| invalid(format!("materialization member content ref is invalid: {error}")))?;
    if input.expected_size > policy.max_member_bytes {
        return Err(invalid(format!(
            "materialization member {} size {} exceeds maximum {}",
            logical_path.as_str(),
            input.expected_size,
            policy.max_member_bytes
        )));
    }
    Ok(())
}

pub fn validate_materialization_plan(plan: &MaterializationPlan) -> crate::error::Result<()> {
    let parsed = parse_materialization_plan_value(&plan.value)?;
    if parsed != *plan {
        return Err(invalid("materialization plan fields do not match canonical plan value"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub profile: String,
    pub plan_ref: String,
    pub plan_value: preserves::IOValue,
    pub replacement: ReplacementPolicy,
    pub destination_authority: String,
    pub member_refs: Vec<(String, String)>,
    pub member_count: usize,
    pub total_bytes: u64,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<String>,
    pub value: preserves::IOValue,
}

impl MaterializationReceipt {
    pub fn valid(&self) -> bool {
        validate_materialization_receipt(self).is_ok()
    }
}

struct MaterializationRootInner {
    dir: cap_std::fs::Dir,
}

pub struct MaterializationRoot {
    inner: std::sync::Arc<MaterializationRootInner>,
}

impl std::fmt::Debug for MaterializationRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MaterializationRoot").finish_non_exhaustive()
    }
}
