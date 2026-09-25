
pub fn validate_materialization_receipt(receipt: &MaterializationReceipt) -> crate::error::Result<()> {
    // r[impl molten.filesystem_materialization.receipt]
    if receipt.decision != DECISION_PASS || !receipt.diagnostics.is_empty() {
        return Err(invalid("materialization receipt is not a passing receipt"));
    }
    if receipt.destination_authority != DESTINATION_AUTHORITY_CAPABILITY_ROOT {
        return Err(invalid("materialization receipt destination authority is unsupported"));
    }
    validate_profile(&receipt.profile)?;
    crate::preserves_rail::validate_content_ref(&receipt.plan_ref)?;
    crate::preserves_rail::validate_content_ref(&receipt.receipt_ref)?;
    if crate::preserves_rail::canonical_hash(&receipt.plan_value)? != receipt.plan_ref {
        return Err(invalid("materialization receipt embedded plan identity mismatch"));
    }
    let embedded_plan = parse_materialization_plan_value(&receipt.plan_value)?;
    let embedded_member_refs = embedded_plan
        .members
        .iter()
        .map(|member| (member.logical_path.as_str().to_string(), member.expected_content_ref.clone()))
        .collect::<Vec<_>>();
    if receipt.profile != embedded_plan.profile
        || receipt.replacement != embedded_plan.replacement
        || receipt.member_refs != embedded_member_refs
        || receipt.member_count != embedded_plan.members.len()
        || receipt.total_bytes != embedded_plan.total_bytes
    {
        return Err(invalid("materialization receipt does not match its embedded plan"));
    }
    if receipt.member_count != receipt.member_refs.len() {
        return Err(invalid("materialization receipt member count is inconsistent"));
    }
    let mut previous = None;
    for (path, reference) in &receipt.member_refs {
        MaterializationPath::parse_within(path, HARD_MAX_MATERIALIZATION_PATH_BYTES)?;
        crate::preserves_rail::validate_content_ref(reference)?;
        if previous.as_ref().is_some_and(|previous: &&String| *previous >= path) {
            return Err(invalid("materialization receipt members are not uniquely sorted"));
        }
        previous = Some(path);
    }
    let expected_non_claims = MATERIALIZATION_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        return Err(invalid("materialization receipt non-claims are incomplete or reordered"));
    }
    let expected_value = materialization_receipt_value(&MaterializationReceiptValueInput {
        decision: &receipt.decision,
        profile: &receipt.profile,
        plan_ref: &receipt.plan_ref,
        plan_value: &receipt.plan_value,
        replacement: receipt.replacement,
        destination_authority: &receipt.destination_authority,
        member_refs: &receipt.member_refs,
        member_count: receipt.member_count,
        total_bytes: receipt.total_bytes,
        diagnostics: &receipt.diagnostics,
        non_claims: &receipt.non_claims,
    })?;
    if expected_value != receipt.value {
        return Err(invalid("materialization receipt fields do not match canonical value"));
    }
    if crate::preserves_rail::canonical_hash(&receipt.value)? != receipt.receipt_ref {
        return Err(invalid("materialization receipt identity mismatch"));
    }
    Ok(())
}

pub fn parse_materialization_receipt(value: &preserves::IOValue) -> crate::error::Result<MaterializationReceipt> {
    let record = value
        .collect_simple_record("filesystem-materialization-receipt-v1", Some(MATERIALIZATION_RECEIPT_FIELD_COUNT))
        .ok_or_else(|| invalid("expected filesystem materialization receipt"))?;
    let fields = record.fields_iter().cloned().collect::<Vec<_>>();
    let [
        schema_field,
        decision_field,
        profile_field,
        plan_field,
        replacement_field,
        destination_field,
        members_field,
        summary_field,
        diagnostics_field,
        non_claims_field,
    ] = fields.as_slice()
    else {
        return Err(invalid("materialization receipt field count changed after parsing"));
    };
    let schema = required_preserves_string(schema_field, "materialization receipt schema")?;
    if schema != MATERIALIZATION_RECEIPT_SCHEMA {
        return Err(invalid("unsupported materialization receipt schema"));
    }
    let decision = required_named_string(decision_field, "decision")?;
    let profile = required_named_string(profile_field, "profile")?;
    let plan_fields = required_record_fields(plan_field, "plan", MATERIALIZATION_PLAN_RECORD_FIELD_COUNT)?;
    let [plan_ref_field, plan_value_field] = plan_fields.as_slice() else {
        return Err(invalid("materialization receipt plan field count changed after parsing"));
    };
    let plan_ref = required_preserves_string(plan_ref_field, "materialization receipt plan ref")?;
    let plan_value = crate::preserves_rail::value_to_iovalue(plan_value_field);
    let replacement = parse_replacement_policy(&required_named_string(replacement_field, "replacement")?)?;
    let destination_authority = required_named_string(destination_field, "destination-authority")?;
    let member_refs = parse_receipt_member_refs(members_field)?;
    let summary = required_record_fields(summary_field, "summary", MATERIALIZATION_SUMMARY_FIELD_COUNT)?;
    let [member_count_field, total_bytes_field] = summary.as_slice() else {
        return Err(invalid("materialization receipt summary field count changed after parsing"));
    };
    let member_count_u64 = required_preserves_u64(member_count_field, "materialization receipt member count")?;
    let member_count = usize::try_from(member_count_u64)
        .map_err(|_| invalid("materialization receipt member count does not fit usize"))?;
    let total_bytes = required_preserves_u64(total_bytes_field, "materialization receipt total bytes")?;
    let diagnostics = parse_named_string_sequence(diagnostics_field, "diagnostics")?;
    let non_claims = parse_named_string_sequence(non_claims_field, "non-claims")?;
    let receipt = MaterializationReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        profile,
        plan_ref,
        plan_value,
        replacement,
        destination_authority,
        member_refs,
        member_count,
        total_bytes,
        diagnostics,
        non_claims,
        value: value.clone(),
    };
    validate_materialization_receipt(&receipt)?;
    Ok(receipt)
}

fn parse_materialization_plan_value(value: &preserves::IOValue) -> crate::error::Result<MaterializationPlan> {
    let record = value
        .collect_simple_record("filesystem-materialization-plan-v1", Some(MATERIALIZATION_PLAN_FIELD_COUNT))
        .ok_or_else(|| invalid("expected filesystem materialization plan"))?;
    let fields = record.fields_iter().cloned().collect::<Vec<_>>();
    let [
        schema_field,
        profile_field,
        replacement_field,
        members_field,
        summary_field,
        reserved_field,
        bounds_field,
    ] = fields.as_slice()
    else {
        return Err(invalid("materialization plan field count changed after parsing"));
    };
    let schema = required_preserves_string(schema_field, "materialization plan schema")?;
    if schema != MATERIALIZATION_PLAN_SCHEMA {
        return Err(invalid("unsupported materialization plan schema"));
    }
    let profile = required_named_string(profile_field, "profile")?;
    let replacement = parse_replacement_policy(&required_named_string(replacement_field, "replacement")?)?;
    let inputs = parse_plan_members(members_field)?;
    let summary = required_record_fields(summary_field, "summary", MATERIALIZATION_SUMMARY_FIELD_COUNT)?;
    let [summary_count_field, summary_bytes_field] = summary.as_slice() else {
        return Err(invalid("materialization plan summary field count changed after parsing"));
    };
    let summary_count = required_preserves_usize(summary_count_field, "materialization plan member count")?;
    let summary_bytes = required_preserves_u64(summary_bytes_field, "materialization plan total bytes")?;
    let reserved_top_level_names = parse_named_string_sequence(reserved_field, "reserved-top-level")?;
    let bounds = required_record_fields(bounds_field, "bounds", MATERIALIZATION_BOUNDS_FIELD_COUNT)?;
    let [
        max_members_field,
        max_member_bytes_field,
        max_total_bytes_field,
        max_path_bytes_field,
    ] = bounds.as_slice()
    else {
        return Err(invalid("materialization plan bounds field count changed after parsing"));
    };
    let policy = MaterializationPolicy {
        profile,
        replacement,
        max_members: required_preserves_usize(max_members_field, "materialization plan maximum members")?,
        max_member_bytes: required_preserves_u64(max_member_bytes_field, "materialization plan maximum member bytes")?,
        max_total_bytes: required_preserves_u64(max_total_bytes_field, "materialization plan maximum total bytes")?,
        max_path_bytes: required_preserves_usize(max_path_bytes_field, "materialization plan maximum path bytes")?,
        reserved_top_level_names,
    };
    let plan = plan_materialization(&policy, &inputs)?;
    if plan.members.len() != summary_count || plan.total_bytes != summary_bytes || plan.value != *value {
        return Err(invalid("materialization plan summary or canonical value is inconsistent"));
    }
    Ok(plan)
}

fn parse_plan_members(
    members_field: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<MaterializationMemberInput>> {
    let member_fields = required_record_fields(members_field, "members", 1)?;
    let member_values = member_fields[0]
        .collect_sequence()
        .ok_or_else(|| invalid("expected materialization plan member sequence"))?;
    if member_values.len() > HARD_MAX_MATERIALIZATION_MEMBERS {
        return Err(invalid("materialization plan member sequence exceeds item bound"));
    }
    let mut inputs = Vec::with_capacity(member_values.len());
    for member_value in member_values.iter() {
        let member_value = crate::preserves_rail::value_to_iovalue(member_value);
        let member = member_value
            .collect_simple_record("member", Some(MATERIALIZATION_PLAN_MEMBER_FIELD_COUNT))
            .ok_or_else(|| invalid("expected materialization plan member"))?;
        let member_fields = member.fields_iter().collect::<Vec<_>>();
        let [path_field, kind_field, content_ref_field, size_field] = member_fields.as_slice() else {
            return Err(invalid("materialization plan member field count changed after parsing"));
        };
        let kind = required_preserves_string(kind_field, "materialization plan member kind")?;
        if kind != MaterializationMemberKind::RegularFile.as_str() {
            return Err(invalid(format!("unsupported materialization plan member kind {kind}")));
        }
        inputs.push(MaterializationMemberInput {
            logical_path: required_preserves_string(path_field, "materialization plan member path")?,
            kind: MaterializationMemberKind::RegularFile,
            expected_content_ref: required_preserves_string(
                content_ref_field,
                "materialization plan member content ref",
            )?,
            expected_size: required_preserves_u64(size_field, "materialization plan member size")?,
        });
    }
    Ok(inputs)
}

fn required_record_fields(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> crate::error::Result<Vec<preserves::Value<preserves::IOValue>>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| invalid(format!("expected {label} record")))?;
    Ok(record.fields_iter().cloned().collect())
}

fn required_preserves_string(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| invalid(format!("expected string for {label}")))
}

fn required_preserves_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| invalid(format!("expected u64 for {label}")))?
        .map_err(|error| invalid(format!("u64 out of range for {label}: {error}")))
}

fn required_preserves_usize(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<usize> {
    let value = required_preserves_u64(value, label)?;
    usize::try_from(value).map_err(|_| invalid(format!("u64 does not fit usize for {label}")))
}

fn required_named_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let fields = required_record_fields(value, label, 1)?;
    required_preserves_string(&fields[0], label)
}

fn parse_named_string_sequence(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Vec<String>> {
    let fields = required_record_fields(value, label, 1)?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| invalid(format!("expected string sequence for {label}")))?;
    if entries.len() > HARD_MAX_MATERIALIZATION_MEMBERS {
        return Err(invalid(format!("materialization receipt {label} exceeds item bound")));
    }
    entries.iter().map(|entry| required_preserves_string(entry, label)).collect()
}
