use std::io::Read;
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

pub const DEFAULT_MAX_MATERIALIZATION_MEMBERS: usize = 4_096;
pub const DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES: u64 = 16 * 1_024 * 1_024;
pub const DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES: u64 = 256 * 1_024 * 1_024;
pub const DEFAULT_MAX_MATERIALIZATION_PATH_BYTES: usize = 1_024;

const HARD_MAX_MATERIALIZATION_MEMBERS: usize = 16_384;
const HARD_MAX_MATERIALIZATION_MEMBER_BYTES: u64 = 256 * 1_024 * 1_024;
const HARD_MAX_MATERIALIZATION_TOTAL_BYTES: u64 = 1_024 * 1_024 * 1_024;
const HARD_MAX_MATERIALIZATION_PATH_BYTES: usize = 4_096;

const STAGING_DIRECTORY: &str = ".molten-materialize";
const STAGING_TREE_DIRECTORY: &str = "tree";
const STAGING_BACKUP_DIRECTORY: &str = "backup";
const ARCHIVE_READ_ONLY_MODE: u32 = 0o444;
const MATERIALIZATION_PLAN_SCHEMA: &str = "molten.filesystem-materialization.plan.v1";
const MATERIALIZATION_RECEIPT_SCHEMA: &str = "molten.filesystem-materialization.receipt.v1";
const DECISION_PASS: &str = "pass";
const DESTINATION_AUTHORITY_CAPABILITY_ROOT: &str = "capability-root";
const MATERIALIZATION_RECEIPT_FIELD_COUNT: usize = 10;
const MATERIALIZATION_PLAN_FIELD_COUNT: usize = 7;
const MATERIALIZATION_PLAN_MEMBER_FIELD_COUNT: usize = 4;
const MATERIALIZATION_RECEIPT_MEMBER_FIELD_COUNT: usize = 2;
const MATERIALIZATION_PLAN_RECORD_FIELD_COUNT: usize = 2;
const MATERIALIZATION_SUMMARY_FIELD_COUNT: usize = 2;
const MATERIALIZATION_BOUNDS_FIELD_COUNT: usize = 4;
const MATERIALIZATION_NON_CLAIMS: &[&str] = &[
    "not-authenticity-proof",
    "not-signature-validity-proof",
    "not-policy-authority",
    "not-confidentiality-proof",
    "not-artifact-semantic-correctness",
    "not-source-trust",
    "not-disclosure-authorization",
    "not-release-eligibility",
    "not-distributed-atomicity",
    "not-durability-proof",
    "not-concurrent-adversarial-race-proof",
    "not-crash-atomic-persistence",
];

const _: () = assert!(DEFAULT_MAX_MATERIALIZATION_MEMBERS > 0);
const _: () = assert!(DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES > 0);
const _: () = assert!(DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES >= DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES);
const _: () = assert!(DEFAULT_MAX_MATERIALIZATION_PATH_BYTES > 0);
const _: () = assert!(HARD_MAX_MATERIALIZATION_MEMBERS >= DEFAULT_MAX_MATERIALIZATION_MEMBERS);
const _: () = assert!(HARD_MAX_MATERIALIZATION_MEMBER_BYTES >= DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES);
const _: () = assert!(HARD_MAX_MATERIALIZATION_TOTAL_BYTES >= DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES);
const _: () = assert!(HARD_MAX_MATERIALIZATION_PATH_BYTES >= DEFAULT_MAX_MATERIALIZATION_PATH_BYTES);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementPolicy {
    NoReplace,
    ReplaceRegularFiles,
}

impl ReplacementPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoReplace => "no-replace",
            Self::ReplaceRegularFiles => "replace-regular-files",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationMemberKind {
    RegularFile,
    Directory,
    Symlink,
    HardLink,
    Special,
}

impl MaterializationMemberKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegularFile => "regular-file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::HardLink => "hard-link",
            Self::Special => "special",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterializationPath {
    normalized: String,
}

impl MaterializationPath {
    pub fn parse(value: &str, max_path_bytes: u64) -> crate::error::Result<Self> {
        Self::parse_within(value, crate::bounded::usize_from_u64(max_path_bytes, "materialization path byte bound")?)
    }

    pub(crate) fn parse_within(value: &str, max_path_bytes: usize) -> crate::error::Result<Self> {
        validate_materialization_path(value, max_path_bytes)?;
        Ok(Self {
            normalized: value.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    pub fn as_path(&self) -> &std::path::Path {
        std::path::Path::new(&self.normalized)
    }

    fn top_level(&self) -> &str {
        self.normalized.split('/').next().unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationPolicy {
    pub profile: String,
    pub replacement: ReplacementPolicy,
    pub max_members: usize,
    pub max_member_bytes: u64,
    pub max_total_bytes: u64,
    pub max_path_bytes: usize,
    pub reserved_top_level_names: Vec<String>,
}

impl MaterializationPolicy {
    pub fn bounded(profile: &str, replacement: ReplacementPolicy) -> crate::error::Result<Self> {
        validate_profile(profile)?;
        Ok(Self {
            profile: profile.to_string(),
            replacement,
            max_members: DEFAULT_MAX_MATERIALIZATION_MEMBERS,
            max_member_bytes: DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES,
            max_total_bytes: DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES,
            max_path_bytes: DEFAULT_MAX_MATERIALIZATION_PATH_BYTES,
            reserved_top_level_names: vec![STAGING_DIRECTORY.to_string()],
        })
    }

    pub fn with_bounds(
        mut self,
        max_members: u64,
        max_member_bytes: u64,
        max_total_bytes: u64,
        max_path_bytes: u64,
    ) -> crate::error::Result<Self> {
        let max_members = crate::bounded::usize_from_u64(max_members, "materialization member bound")?;
        let max_path_bytes = crate::bounded::usize_from_u64(max_path_bytes, "materialization path byte bound")?;
        validate_bounds(max_members, max_member_bytes, max_total_bytes, max_path_bytes)?;
        self.max_members = max_members;
        self.max_member_bytes = max_member_bytes;
        self.max_total_bytes = max_total_bytes;
        self.max_path_bytes = max_path_bytes;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationMemberInput {
    pub logical_path: String,
    pub kind: MaterializationMemberKind,
    pub expected_content_ref: String,
    pub expected_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationMember {
    pub logical_path: MaterializationPath,
    pub kind: MaterializationMemberKind,
    pub expected_content_ref: String,
    pub expected_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationPlan {
    pub profile: String,
    pub replacement: ReplacementPolicy,
    pub members: Vec<MaterializationMember>,
    pub total_bytes: u64,
    pub plan_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationPayload {
    pub logical_path: String,
    pub bytes: Vec<u8>,
}

impl MaterializationPayload {
    pub fn new(logical_path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            logical_path: logical_path.into(),
            bytes: bytes.into(),
        }
    }

    pub fn member_input(&self) -> crate::error::Result<MaterializationMemberInput> {
        Ok(MaterializationMemberInput {
            logical_path: self.logical_path.clone(),
            kind: MaterializationMemberKind::RegularFile,
            expected_content_ref: crate::preserves_rail::content_ref_from_bytes(&self.bytes),
            expected_size: u64::try_from(self.bytes.len())
                .map_err(|_| invalid("materialization payload size does not fit u64"))?,
        })
    }
}

pub fn plan_payloads(
    policy: &MaterializationPolicy,
    payloads: &[MaterializationPayload],
) -> crate::error::Result<MaterializationPlan> {
    let inputs = payloads
        .iter()
        .map(MaterializationPayload::member_input)
        .collect::<crate::error::Result<Vec<_>>>()?;
    plan_materialization(policy, &inputs)
}

pub fn plan_materialization(
    policy: &MaterializationPolicy,
    inputs: &[MaterializationMemberInput],
) -> crate::error::Result<MaterializationPlan> {
    // r[impl molten.filesystem_materialization.plan]
    let mut policy = policy.clone();
    policy.reserved_top_level_names.sort();
    validate_policy(&policy)?;
    if inputs.is_empty() {
        return Err(invalid("materialization plan must contain at least one member"));
    }
    if inputs.len() > policy.max_members {
        return Err(invalid(format!(
            "materialization member count {} exceeds maximum {}",
            inputs.len(),
            policy.max_members
        )));
    }

    let reserved = policy
        .reserved_top_level_names
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let mut members = Vec::with_capacity(inputs.len());
    let mut seen = std::collections::BTreeSet::new();
    let mut total_bytes = 0u64;
    for input in inputs {
        let logical_path = MaterializationPath::parse_within(&input.logical_path, policy.max_path_bytes)?;
        validate_member_input(&policy, &reserved, &logical_path, input)?;
        total_bytes = total_bytes
            .checked_add(input.expected_size)
            .ok_or_else(|| invalid("materialization total byte count overflow"))?;
        if total_bytes > policy.max_total_bytes {
            return Err(invalid(format!(
                "materialization total bytes {total_bytes} exceed maximum {}",
                policy.max_total_bytes
            )));
        }
        if !seen.insert(logical_path.clone()) {
            return Err(invalid(format!("duplicate normalized materialization member: {}", logical_path.as_str())));
        }
        members.push(MaterializationMember {
            logical_path,
            kind: input.kind,
            expected_content_ref: input.expected_content_ref.clone(),
            expected_size: input.expected_size,
        });
    }
    members.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    let value = materialization_plan_value(&policy, &members, total_bytes)?;
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(MaterializationPlan {
        profile: policy.profile.clone(),
        replacement: policy.replacement,
        members,
        total_bytes,
        plan_ref,
        value,
    })
}
