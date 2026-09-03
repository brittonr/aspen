#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "profile validation helpers append to one bounded diagnostics vector owned by the caller"
)]

use content_identity_core::Domain;
use content_identity_core::IdentityError;
use content_identity_core::IdentityHasher;

use super::*;

// r[impl molten.prolly_map.profile]
pub fn standard_prolly_profile() -> Result<ProllyProfile, Vec<ProllyIssue>> {
    let mut profile = ProllyProfile {
        schema: PROLLY_PROFILE_SCHEMA.to_string(),
        format: PROLLY_MAP_FORMAT.to_string(),
        format_version: PROLLY_FORMAT_VERSION,
        key_codec: PROLLY_KEY_CODEC.to_string(),
        value_codec: PROLLY_VALUE_CODEC.to_string(),
        comparison: PROLLY_COMPARISON.to_string(),
        node_codec: PROLLY_NODE_CODEC.to_string(),
        boundary_domain: PROLLY_BOUNDARY_DOMAIN.to_string(),
        boundary_seed_ref: PROLLY_BOUNDARY_SEED_REF.to_string(),
        size_accounting: PROLLY_SIZE_ACCOUNTING.to_string(),
        min_node_bytes: MIN_NODE_BYTES,
        target_node_bytes: TARGET_NODE_BYTES,
        max_node_bytes: MAX_NODE_BYTES,
        min_fanout: MIN_FANOUT,
        target_fanout: TARGET_FANOUT,
        max_fanout: MAX_FANOUT,
        profile_domain: PROLLY_PROFILE_DOMAIN.to_string(),
        leaf_domain: PROLLY_LEAF_DOMAIN.to_string(),
        internal_domain: PROLLY_INTERNAL_DOMAIN.to_string(),
        root_domain: PROLLY_ROOT_DOMAIN.to_string(),
        limits: ProllyLimits {
            max_key_bytes: MAX_KEY_BYTES,
            max_value_bytes: MAX_VALUE_BYTES,
            max_entries: MAX_ENTRIES,
            max_tree_height: MAX_TREE_HEIGHT,
            max_diff_records: MAX_DIFF_RECORDS,
            max_graph_facts: MAX_GRAPH_FACTS,
        },
        profile_ref: ProfileRef::new(String::new()),
    };
    profile.profile_ref = derive_profile_ref(&profile).map_err(|issue| vec![issue])?;
    let issues = validate_profile(&profile);
    if issues.is_empty() { Ok(profile) } else { Err(issues) }
}

pub fn validate_profile(profile: &ProllyProfile) -> Vec<ProllyIssue> {
    let mut issues = Vec::new();
    check_exact(&mut issues, profile.schema == PROLLY_PROFILE_SCHEMA, "schema");
    check_exact(&mut issues, profile.format == PROLLY_MAP_FORMAT, "format");
    check_exact(&mut issues, profile.format_version == PROLLY_FORMAT_VERSION, "format-version");
    check_exact(&mut issues, profile.key_codec == PROLLY_KEY_CODEC, "key-codec");
    check_exact(&mut issues, profile.value_codec == PROLLY_VALUE_CODEC, "value-codec");
    check_exact(&mut issues, profile.comparison == PROLLY_COMPARISON, "comparison");
    check_exact(&mut issues, profile.node_codec == PROLLY_NODE_CODEC, "node-codec");
    check_exact(&mut issues, profile.boundary_domain == PROLLY_BOUNDARY_DOMAIN, "boundary-domain");
    check_exact(&mut issues, profile.size_accounting == PROLLY_SIZE_ACCOUNTING, "size-accounting");
    check_exact(&mut issues, profile.profile_domain == PROLLY_PROFILE_DOMAIN, "profile-domain");
    check_exact(&mut issues, profile.leaf_domain == PROLLY_LEAF_DOMAIN, "leaf-domain");
    check_exact(&mut issues, profile.internal_domain == PROLLY_INTERNAL_DOMAIN, "internal-domain");
    check_exact(&mut issues, profile.root_domain == PROLLY_ROOT_DOMAIN, "root-domain");
    if !is_content_ref(&profile.boundary_seed_ref) {
        issues.push(ProllyIssue::MalformedReference(profile.boundary_seed_ref.clone()));
    }
    validate_node_bounds(profile, &mut issues);
    validate_limits(profile.limits, &mut issues);
    match derive_profile_ref(profile) {
        Ok(expected) if expected != profile.profile_ref => issues.push(ProllyIssue::ProfileIdentityMismatch),
        Ok(_) => {}
        Err(issue) => issues.push(issue),
    }
    issues.sort();
    issues.dedup();
    issues
}

pub fn derive_profile_ref(profile: &ProllyProfile) -> Result<ProfileRef, ProllyIssue> {
    let mut hasher = hasher(&profile.profile_domain)?;
    hasher.update_tagged_str(b"schema", &profile.schema).map_err(identity_issue)?;
    hasher.update_tagged_str(b"format", &profile.format).map_err(identity_issue)?;
    hasher.update_tagged_u64_le(b"format-version", u64::from(profile.format_version));
    hasher.update_tagged_str(b"key-codec", &profile.key_codec).map_err(identity_issue)?;
    hasher.update_tagged_str(b"value-codec", &profile.value_codec).map_err(identity_issue)?;
    hasher.update_tagged_str(b"comparison", &profile.comparison).map_err(identity_issue)?;
    hasher.update_tagged_str(b"node-codec", &profile.node_codec).map_err(identity_issue)?;
    hasher.update_tagged_str(b"boundary-domain", &profile.boundary_domain).map_err(identity_issue)?;
    hasher.update_tagged_str(b"boundary-seed", &profile.boundary_seed_ref).map_err(identity_issue)?;
    hasher.update_tagged_str(b"size-accounting", &profile.size_accounting).map_err(identity_issue)?;
    update_bounds(&mut hasher, profile);
    Ok(ProfileRef::new(format!("blake3:{}", hasher.finish().to_hex())))
}

pub(crate) fn hasher(domain: &str) -> Result<IdentityHasher, ProllyIssue> {
    Domain::new(domain.to_string()).map(IdentityHasher::new).map_err(identity_issue)
}

pub(crate) fn identity_issue(error: IdentityError) -> ProllyIssue {
    ProllyIssue::IdentityFailure(error.to_string())
}

pub(crate) fn is_content_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    hex.len() == blake3::OUT_LEN * 2 && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn update_bounds(hasher: &mut IdentityHasher, profile: &ProllyProfile) {
    hasher.update_tagged_u64_le(b"min-node-bytes", u64::from(profile.min_node_bytes));
    hasher.update_tagged_u64_le(b"target-node-bytes", u64::from(profile.target_node_bytes));
    hasher.update_tagged_u64_le(b"max-node-bytes", u64::from(profile.max_node_bytes));
    hasher.update_tagged_u64_le(b"min-fanout", u64::from(profile.min_fanout));
    hasher.update_tagged_u64_le(b"target-fanout", u64::from(profile.target_fanout));
    hasher.update_tagged_u64_le(b"max-fanout", u64::from(profile.max_fanout));
    hasher.update_tagged_u64_le(b"max-key-bytes", u64::from(profile.limits.max_key_bytes));
    hasher.update_tagged_u64_le(b"max-value-bytes", u64::from(profile.limits.max_value_bytes));
    hasher.update_tagged_u64_le(b"max-entries", u64::from(profile.limits.max_entries));
    hasher.update_tagged_u64_le(b"max-tree-height", u64::from(profile.limits.max_tree_height));
    hasher.update_tagged_u64_le(b"max-diff-records", u64::from(profile.limits.max_diff_records));
    hasher.update_tagged_u64_le(b"max-graph-facts", u64::from(profile.limits.max_graph_facts));
}

fn validate_node_bounds(profile: &ProllyProfile, issues: &mut Vec<ProllyIssue>) {
    if profile.min_node_bytes == 0
        || profile.min_node_bytes > profile.target_node_bytes
        || profile.target_node_bytes >= profile.max_node_bytes
        || profile.max_node_bytes > MAX_NODE_BYTES
    {
        issues.push(ProllyIssue::ProfileBoundInvalid("node-bytes"));
    }
    if profile.min_fanout < MIN_FANOUT
        || profile.min_fanout > profile.target_fanout
        || profile.target_fanout >= profile.max_fanout
        || profile.max_fanout > MAX_FANOUT
    {
        issues.push(ProllyIssue::ProfileBoundInvalid("fanout"));
    }
}

fn validate_limits(limits: ProllyLimits, issues: &mut Vec<ProllyIssue>) {
    if limits.max_key_bytes == 0 || limits.max_key_bytes > MAX_KEY_BYTES {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-key-bytes"));
    }
    if limits.max_value_bytes == 0 || limits.max_value_bytes > MAX_VALUE_BYTES {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-value-bytes"));
    }
    if limits.max_entries == 0 || limits.max_entries > MAX_ENTRIES {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-entries"));
    }
    if limits.max_tree_height == 0 || limits.max_tree_height > MAX_TREE_HEIGHT {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-tree-height"));
    }
    if limits.max_diff_records == 0 || limits.max_diff_records > MAX_DIFF_RECORDS {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-diff-records"));
    }
    if limits.max_graph_facts == 0 || limits.max_graph_facts > MAX_GRAPH_FACTS {
        issues.push(ProllyIssue::ProfileBoundInvalid("max-graph-facts"));
    }
}

fn check_exact(issues: &mut Vec<ProllyIssue>, is_match: bool, field: &'static str) {
    if !is_match {
        issues.push(ProllyIssue::ProfileFieldMismatch(field));
    }
}
