
fn parse_receipt_member_refs(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<Vec<(String, String)>> {
    let fields = required_record_fields(value, "members", 1)?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| invalid("expected materialization receipt member sequence"))?;
    if entries.len() > HARD_MAX_MATERIALIZATION_MEMBERS {
        return Err(invalid("materialization receipt member sequence exceeds item bound"));
    }
    let mut members = Vec::with_capacity(entries.len());
    for entry in entries.iter() {
        let entry = crate::preserves_rail::value_to_iovalue(entry);
        let member = entry
            .collect_simple_record("member", Some(MATERIALIZATION_RECEIPT_MEMBER_FIELD_COUNT))
            .ok_or_else(|| invalid("expected materialization receipt member"))?;
        let member_fields = member.fields_iter().collect::<Vec<_>>();
        let [path_field, reference_field] = member_fields.as_slice() else {
            return Err(invalid("materialization receipt member field count changed after parsing"));
        };
        members.push((
            required_preserves_string(path_field, "materialization receipt member path")?,
            required_preserves_string(reference_field, "materialization receipt member ref")?,
        ));
    }
    Ok(members)
}

fn parse_replacement_policy(value: &str) -> crate::error::Result<ReplacementPolicy> {
    match value {
        "no-replace" => Ok(ReplacementPolicy::NoReplace),
        "replace-regular-files" => Ok(ReplacementPolicy::ReplaceRegularFiles),
        _ => Err(invalid(format!("unsupported materialization replacement policy {value}"))),
    }
}

fn build_materialization_receipt(plan: &MaterializationPlan) -> crate::error::Result<MaterializationReceipt> {
    // r[impl molten.filesystem_materialization.receipt]
    validate_materialization_plan(plan)?;
    let member_refs = plan
        .members
        .iter()
        .map(|member| (member.logical_path.as_str().to_string(), member.expected_content_ref.clone()))
        .collect::<Vec<_>>();
    let non_claims = MATERIALIZATION_NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect::<Vec<_>>();
    let value = materialization_receipt_value(&MaterializationReceiptValueInput {
        decision: DECISION_PASS,
        profile: &plan.profile,
        plan_ref: &plan.plan_ref,
        plan_value: &plan.value,
        replacement: plan.replacement,
        destination_authority: DESTINATION_AUTHORITY_CAPABILITY_ROOT,
        member_refs: &member_refs,
        member_count: plan.members.len(),
        total_bytes: plan.total_bytes,
        diagnostics: &[],
        non_claims: &non_claims,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(MaterializationReceipt {
        receipt_ref,
        decision: DECISION_PASS.to_string(),
        profile: plan.profile.clone(),
        plan_ref: plan.plan_ref.clone(),
        plan_value: plan.value.clone(),
        replacement: plan.replacement,
        destination_authority: DESTINATION_AUTHORITY_CAPABILITY_ROOT.to_string(),
        member_refs,
        member_count: plan.members.len(),
        total_bytes: plan.total_bytes,
        diagnostics: Vec::new(),
        non_claims,
        value,
    })
}

struct MaterializationReceiptValueInput<'a> {
    decision: &'a str,
    profile: &'a str,
    plan_ref: &'a str,
    plan_value: &'a preserves::IOValue,
    replacement: ReplacementPolicy,
    destination_authority: &'a str,
    member_refs: &'a [(String, String)],
    member_count: usize,
    total_bytes: u64,
    diagnostics: &'a [String],
    non_claims: &'a [String],
}

fn materialization_receipt_value(
    input: &MaterializationReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    let member_count =
        u64::try_from(input.member_count).map_err(|_| invalid("materialization member count does not fit u64"))?;
    Ok(crate::preserves_rail::record("filesystem-materialization-receipt-v1", vec![
        crate::preserves_rail::string(MATERIALIZATION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(input.profile)]),
        crate::preserves_rail::record("plan", vec![
            crate::preserves_rail::string(input.plan_ref),
            input.plan_value.clone(),
        ]),
        crate::preserves_rail::record("replacement", vec![crate::preserves_rail::string(input.replacement.as_str())]),
        crate::preserves_rail::record("destination-authority", vec![crate::preserves_rail::string(
            input.destination_authority,
        )]),
        crate::preserves_rail::record("members", vec![crate::preserves_rail::sequence(
            input
                .member_refs
                .iter()
                .map(|(path, reference)| {
                    crate::preserves_rail::record("member", vec![
                        crate::preserves_rail::string(path),
                        crate::preserves_rail::string(reference),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("summary", vec![
            crate::preserves_rail::u64_value(member_count),
            crate::preserves_rail::u64_value(input.total_bytes),
        ]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("non-claims", vec![crate::preserves_rail::sequence(
            input.non_claims.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ]))
}

fn validate_payloads<'a>(
    plan: &MaterializationPlan,
    payloads: &'a [MaterializationPayload],
) -> crate::error::Result<std::collections::BTreeMap<MaterializationPath, &'a [u8]>> {
    validate_materialization_plan(plan)?;
    let planned_paths = plan
        .members
        .iter()
        .map(|member| (member.logical_path.as_str(), &member.logical_path))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut payload_map = std::collections::BTreeMap::new();
    for payload in payloads {
        let path = planned_paths
            .get(payload.logical_path.as_str())
            .ok_or_else(|| invalid(format!("unplanned materialization payload: {}", payload.logical_path)))?;
        if payload_map.insert((*path).clone(), payload.bytes.as_slice()).is_some() {
            return Err(invalid(format!("duplicate materialization payload: {}", path.as_str())));
        }
    }
    if payload_map.len() != plan.members.len() {
        return Err(invalid("materialization payload set does not match planned member count"));
    }
    for member in &plan.members {
        let bytes = payload_map
            .get(&member.logical_path)
            .ok_or_else(|| invalid(format!("missing materialization payload: {}", member.logical_path.as_str())))?;
        verify_payload_bytes(member, bytes)?;
    }
    Ok(payload_map)
}

fn verify_payload_bytes(member: &MaterializationMember, bytes: &[u8]) -> crate::error::Result<()> {
    let size = u64::try_from(bytes.len()).map_err(|_| invalid("materialization payload size does not fit u64"))?;
    if size != member.expected_size {
        return Err(invalid(format!(
            "materialization payload {} size mismatch: expected {} observed {size}",
            member.logical_path.as_str(),
            member.expected_size
        )));
    }
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(bytes);
    if observed_ref != member.expected_content_ref {
        return Err(invalid(format!(
            "materialization payload {} ref mismatch: expected {} observed {observed_ref}",
            member.logical_path.as_str(),
            member.expected_content_ref
        )));
    }
    Ok(())
}

fn verify_member_bytes(
    member: &MaterializationMember,
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<()> {
    let bytes = read_regular_file_bounded(dir, path, member.expected_size)?;
    verify_payload_bytes(member, &bytes)
}

fn verify_published_members(dir: &cap_std::fs::Dir, plan: &MaterializationPlan) -> crate::error::Result<()> {
    for member in &plan.members {
        verify_member_bytes(member, dir, member.logical_path.as_path())?;
    }
    Ok(())
}

fn validate_policy(policy: &MaterializationPolicy) -> crate::error::Result<()> {
    validate_profile(&policy.profile)?;
    validate_bounds(policy.max_members, policy.max_member_bytes, policy.max_total_bytes, policy.max_path_bytes)?;
    let mut reserved = std::collections::BTreeSet::new();
    for name in &policy.reserved_top_level_names {
        validate_reserved_name(name)?;
        if !reserved.insert(name) {
            return Err(invalid("materialization policy contains duplicate reserved names"));
        }
    }
    Ok(())
}

fn validate_bounds(
    max_members: usize,
    max_member_bytes: u64,
    max_total_bytes: u64,
    max_path_bytes: usize,
) -> crate::error::Result<()> {
    if max_members == 0 || max_member_bytes == 0 || max_total_bytes == 0 || max_path_bytes == 0 {
        return Err(invalid("materialization bounds must be non-zero"));
    }
    if max_total_bytes < max_member_bytes {
        return Err(invalid("materialization total-byte bound cannot be smaller than member-byte bound"));
    }
    if max_members > HARD_MAX_MATERIALIZATION_MEMBERS
        || max_member_bytes > HARD_MAX_MATERIALIZATION_MEMBER_BYTES
        || max_total_bytes > HARD_MAX_MATERIALIZATION_TOTAL_BYTES
        || max_path_bytes > HARD_MAX_MATERIALIZATION_PATH_BYTES
    {
        return Err(invalid("materialization bounds exceed hard safety ceilings"));
    }
    Ok(())
}

fn validate_profile(profile: &str) -> crate::error::Result<()> {
    if profile.is_empty() {
        return Err(invalid("materialization profile cannot be empty"));
    }
    if !profile
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.')
    {
        return Err(invalid("materialization profile contains unsupported characters"));
    }
    Ok(())
}

fn validate_reserved_name(name: &str) -> crate::error::Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(invalid("materialization reserved name must be one logical component"));
    }
    Ok(())
}

fn logical_path_from_relative_path(path: &std::path::Path) -> crate::error::Result<String> {
    let components = path
        .components()
        .map(|component| {
            let std::path::Component::Normal(component) = component else {
                return Err(invalid("materialization source path is not normalized"));
            };
            component.to_str().ok_or_else(|| invalid("materialization source path must be UTF-8"))
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(invalid("materialization source path cannot be empty"));
    }
    Ok(components.join("/"))
}
