use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use cap_fs_ext::FollowSymlinks;
use cap_fs_ext::OpenOptionsFollowExt;

use crate::error::MoltenError;
use crate::error::Result;

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
    pub fn parse(value: &str, max_path_bytes: usize) -> Result<Self> {
        validate_materialization_path(value, max_path_bytes)?;
        Ok(Self {
            normalized: value.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.normalized)
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
    pub fn bounded(profile: &str, replacement: ReplacementPolicy) -> Result<Self> {
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
        max_members: usize,
        max_member_bytes: u64,
        max_total_bytes: u64,
        max_path_bytes: usize,
    ) -> Result<Self> {
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

    pub fn member_input(&self) -> Result<MaterializationMemberInput> {
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
) -> Result<MaterializationPlan> {
    let inputs = payloads.iter().map(MaterializationPayload::member_input).collect::<Result<Vec<_>>>()?;
    plan_materialization(policy, &inputs)
}

pub fn plan_materialization(
    policy: &MaterializationPolicy,
    inputs: &[MaterializationMemberInput],
) -> Result<MaterializationPlan> {
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

    let reserved = policy.reserved_top_level_names.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut members = Vec::with_capacity(inputs.len());
    let mut seen = BTreeSet::new();
    let mut total_bytes = 0u64;
    for input in inputs {
        let logical_path = MaterializationPath::parse(&input.logical_path, policy.max_path_bytes)?;
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

pub fn validate_materialization_plan(plan: &MaterializationPlan) -> Result<()> {
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
    inner: Arc<MaterializationRootInner>,
}

impl std::fmt::Debug for MaterializationRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MaterializationRoot").finish_non_exhaustive()
    }
}

impl MaterializationRoot {
    pub fn open(destination: &Path) -> Result<Self> {
        // r[impl molten.filesystem_materialization.root]
        match std::fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(invalid("materialization destination must be a real directory"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(destination).map_err(MoltenError::from)?;
            }
            Err(error) => return Err(MoltenError::from(error)),
        }
        let dir =
            cap_std::fs::Dir::open_ambient_dir(destination, cap_std::ambient_authority()).map_err(MoltenError::from)?;
        Ok(Self::from_dir(dir))
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self {
            inner: Arc::new(MaterializationRootInner { dir }),
        }
    }

    pub fn stage(
        &self,
        plan: &MaterializationPlan,
        payloads: &[MaterializationPayload],
    ) -> Result<StagedMaterialization> {
        validate_materialization_plan(plan)?;
        let payload_map = validate_payloads(plan, payloads)?;
        let stage_path = stage_path(plan)?;
        let stage_result = self.stage_inner(plan, &payload_map, &stage_path);
        if let Err(error) = stage_result {
            let cleanup_result = remove_tree_if_present(&self.inner.dir, &stage_path);
            return match cleanup_result {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(invalid(format!(
                    "materialization stage failed: {error}; stage cleanup failed: {cleanup_error}"
                ))),
            };
        }
        Ok(StagedMaterialization {
            root: Arc::clone(&self.inner),
            plan_ref: plan.plan_ref.clone(),
            stage_path,
        })
    }

    pub fn commit(&self, plan: &MaterializationPlan, staged: &StagedMaterialization) -> Result<MaterializationReceipt> {
        self.commit_inner(plan, staged, None)
    }

    fn commit_inner(
        &self,
        plan: &MaterializationPlan,
        staged: &StagedMaterialization,
        fail_after_publications: Option<usize>,
    ) -> Result<MaterializationReceipt> {
        // r[impl molten.filesystem_materialization.commit]
        validate_materialization_plan(plan)?;
        if !Arc::ptr_eq(&self.inner, &staged.root) {
            return Err(invalid("staged materialization belongs to a different destination root"));
        }
        if staged.plan_ref != plan.plan_ref {
            return Err(invalid("staged materialization plan identity is stale or mismatched"));
        }
        self.preflight_publication(plan)?;
        let mut created_final_directories = Vec::new();
        for member in &plan.members {
            if let Err(error) = create_directory_tree_recording(
                &self.inner.dir,
                member.logical_path.as_path().parent(),
                &mut created_final_directories,
            ) {
                return Err(setup_failure(&self.inner.dir, &created_final_directories, error));
            }
            let backup_path = staged.stage_path.join(STAGING_BACKUP_DIRECTORY).join(member.logical_path.as_path());
            if let Err(error) = create_directory_tree(&self.inner.dir, backup_path.parent()) {
                return Err(setup_failure(&self.inner.dir, &created_final_directories, error));
            }
        }
        let mut states = Vec::with_capacity(plan.members.len());
        for member in &plan.members {
            let final_path = member.logical_path.as_path().to_path_buf();
            let backup_path = staged.stage_path.join(STAGING_BACKUP_DIRECTORY).join(member.logical_path.as_path());
            let mut state = PublicationState {
                final_path: final_path.clone(),
                backup_path: None,
                published: false,
            };
            let existing = match entry_kind(&self.inner.dir, &final_path) {
                Ok(existing) => existing,
                Err(error) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        error,
                    ));
                }
            };
            match (plan.replacement, existing) {
                (ReplacementPolicy::NoReplace, None) | (ReplacementPolicy::ReplaceRegularFiles, None) => {}
                (ReplacementPolicy::NoReplace, Some(_)) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        invalid("no-replace materialization target appeared during publication"),
                    ));
                }
                (ReplacementPolicy::ReplaceRegularFiles, Some(MaterializationMemberKind::RegularFile)) => {
                    if let Err(error) = self.inner.dir.rename(&final_path, &self.inner.dir, &backup_path) {
                        return Err(publication_failure(
                            &self.inner.dir,
                            &state,
                            &states,
                            &created_final_directories,
                            MoltenError::from(error),
                        ));
                    }
                    state.backup_path = Some(backup_path);
                }
                (ReplacementPolicy::ReplaceRegularFiles, Some(_)) => {
                    return Err(publication_failure(
                        &self.inner.dir,
                        &state,
                        &states,
                        &created_final_directories,
                        invalid("replacement target changed to a link or special entry during publication"),
                    ));
                }
            }
            let staged_file = staged.stage_path.join(STAGING_TREE_DIRECTORY).join(member.logical_path.as_path());
            if let Err(error) = self.inner.dir.hard_link(&staged_file, &self.inner.dir, &final_path) {
                return Err(publication_failure(
                    &self.inner.dir,
                    &state,
                    &states,
                    &created_final_directories,
                    MoltenError::from(error),
                ));
            }
            state.published = true;
            states.push(state);
            if let Err(error) = self.inner.dir.remove_file(&staged_file) {
                return Err(rollback_failure(
                    &self.inner.dir,
                    &states,
                    &created_final_directories,
                    MoltenError::from(error),
                ));
            }
            if fail_after_publications.is_some_and(|limit| states.len() == limit) {
                return Err(rollback_failure(
                    &self.inner.dir,
                    &states,
                    &created_final_directories,
                    invalid("injected materialization publication failure"),
                ));
            }
        }
        if let Err(error) = verify_published_members(&self.inner.dir, plan) {
            return Err(rollback_failure(&self.inner.dir, &states, &created_final_directories, error));
        }
        remove_tree_if_present(&self.inner.dir, &staged.stage_path)?;
        build_materialization_receipt(plan)
    }

    pub fn abort(&self, staged: &StagedMaterialization) -> Result<()> {
        if !Arc::ptr_eq(&self.inner, &staged.root) {
            return Err(invalid("cannot abort a stage owned by another materialization root"));
        }
        remove_tree_if_present(&self.inner.dir, &staged.stage_path)
    }

    pub fn materialize(
        &self,
        plan: &MaterializationPlan,
        payloads: &[MaterializationPayload],
    ) -> Result<MaterializationReceipt> {
        let staged = self.stage(plan, payloads)?;
        match self.commit(plan, &staged) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                let abort_result = self.abort(&staged);
                match abort_result {
                    Ok(()) => Err(error),
                    Err(abort_error) => Err(invalid(format!(
                        "materialization commit failed: {error}; stage quarantine cleanup failed: {abort_error}"
                    ))),
                }
            }
        }
    }

    pub fn read(&self, path: &MaterializationPath) -> Result<Vec<u8>> {
        read_regular_file_bounded(&self.inner.dir, path.as_path(), DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES)
    }

    fn stage_inner(
        &self,
        plan: &MaterializationPlan,
        payloads: &BTreeMap<MaterializationPath, &[u8]>,
        stage_path: &Path,
    ) -> Result<()> {
        create_staging_root(&self.inner.dir, stage_path)?;
        for member in &plan.members {
            let bytes = payloads
                .get(&member.logical_path)
                .ok_or_else(|| invalid("materialization payload disappeared after validation"))?;
            let staged_path = stage_path.join(STAGING_TREE_DIRECTORY).join(member.logical_path.as_path());
            create_directory_tree(&self.inner.dir, staged_path.parent())?;
            write_create_new(&self.inner.dir, &staged_path, bytes)?;
            verify_member_bytes(member, &self.inner.dir, &staged_path)?;
        }
        Ok(())
    }

    fn preflight_publication(&self, plan: &MaterializationPlan) -> Result<()> {
        for member in &plan.members {
            let final_path = member.logical_path.as_path();
            ensure_no_symlink_components(&self.inner.dir, final_path.parent())?;
            match entry_kind(&self.inner.dir, final_path)? {
                None => {}
                Some(MaterializationMemberKind::RegularFile)
                    if plan.replacement == ReplacementPolicy::ReplaceRegularFiles => {}
                Some(kind) if plan.replacement == ReplacementPolicy::ReplaceRegularFiles => {
                    return Err(invalid(format!(
                        "materialization replacement target {} has unsupported kind {}",
                        member.logical_path.as_str(),
                        kind.as_str()
                    )));
                }
                Some(_) => {
                    return Err(invalid(format!(
                        "materialization no-replace target already exists: {}",
                        member.logical_path.as_str()
                    )));
                }
            }
        }
        Ok(())
    }
}

pub struct StagedMaterialization {
    root: Arc<MaterializationRootInner>,
    plan_ref: String,
    stage_path: PathBuf,
}

impl std::fmt::Debug for StagedMaterialization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StagedMaterialization")
            .field("plan_ref", &self.plan_ref)
            .finish_non_exhaustive()
    }
}

pub struct SourceDirectoryRoot {
    dir: cap_std::fs::Dir,
}

impl std::fmt::Debug for SourceDirectoryRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SourceDirectoryRoot").finish_non_exhaustive()
    }
}

impl SourceDirectoryRoot {
    pub fn open_existing(source: &Path) -> Result<Self> {
        let metadata = std::fs::symlink_metadata(source).map_err(MoltenError::from)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(invalid("materialization source must be a real directory"));
        }
        let dir =
            cap_std::fs::Dir::open_ambient_dir(source, cap_std::ambient_authority()).map_err(MoltenError::from)?;
        Ok(Self { dir })
    }

    pub fn from_dir(dir: cap_std::fs::Dir) -> Self {
        Self { dir }
    }

    pub fn read_payloads(
        &self,
        policy: &MaterializationPolicy,
        plan: &MaterializationPlan,
    ) -> Result<Vec<MaterializationPayload>> {
        validate_materialization_plan(plan)?;
        let planned_inputs = plan
            .members
            .iter()
            .map(|member| MaterializationMemberInput {
                logical_path: member.logical_path.as_str().to_string(),
                kind: member.kind,
                expected_content_ref: member.expected_content_ref.clone(),
                expected_size: member.expected_size,
            })
            .collect::<Vec<_>>();
        if plan_materialization(policy, &planned_inputs)? != *plan {
            return Err(invalid("source read policy does not match materialization plan"));
        }
        let mut payloads = Vec::with_capacity(plan.members.len());
        for member in &plan.members {
            let bytes = read_regular_file_bounded(&self.dir, member.logical_path.as_path(), policy.max_member_bytes)?;
            verify_payload_bytes(member, &bytes)?;
            payloads.push(MaterializationPayload::new(member.logical_path.as_str(), bytes));
        }
        Ok(payloads)
    }

    pub fn read_path(&self, path: &MaterializationPath, max_bytes: u64) -> Result<Vec<u8>> {
        read_regular_file_bounded(&self.dir, path.as_path(), max_bytes)
    }

    pub fn open_subdir(&self, path: &MaterializationPath) -> Result<Self> {
        ensure_no_symlink_components(&self.dir, Some(path.as_path()))?;
        let dir = self.dir.open_dir(path.as_path()).map_err(MoltenError::from)?;
        Ok(Self { dir })
    }

    pub fn list_regular_files_recursive(&self, policy: &MaterializationPolicy) -> Result<Vec<MaterializationPath>> {
        validate_policy(policy)?;
        let mut directories = vec![PathBuf::new()];
        let mut files = Vec::new();
        let mut observed_entries = 0usize;
        while let Some(directory) = directories.pop() {
            let read_path = if directory.as_os_str().is_empty() {
                Path::new(".")
            } else {
                directory.as_path()
            };
            for entry_result in self.dir.read_dir(read_path).map_err(MoltenError::from)? {
                observed_entries = observed_entries
                    .checked_add(1)
                    .ok_or_else(|| invalid("materialization source entry count overflow"))?;
                if observed_entries > policy.max_members {
                    return Err(invalid("materialization source traversal exceeds member bound"));
                }
                let entry = entry_result.map_err(MoltenError::from)?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| invalid("materialization source name must be UTF-8"))?;
                let relative = directory.join(name);
                let file_type = entry.file_type().map_err(MoltenError::from)?;
                if file_type.is_dir() {
                    directories.push(relative);
                } else if file_type.is_file() {
                    let rendered = logical_path_from_relative_path(&relative)?;
                    files.push(MaterializationPath::parse(&rendered, policy.max_path_bytes)?);
                } else {
                    return Err(invalid("materialization source contains a link or special entry"));
                }
            }
        }
        files.sort();
        Ok(files)
    }
}

pub fn materialize_path(
    destination: &Path,
    policy: &MaterializationPolicy,
    payloads: &[MaterializationPayload],
) -> Result<MaterializationReceipt> {
    let plan = plan_payloads(policy, payloads)?;
    let root = MaterializationRoot::open(destination)?;
    root.materialize(&plan, payloads)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedArchive {
    pub plan: MaterializationPlan,
    pub payloads: Vec<MaterializationPayload>,
}

pub fn write_archive<W: Write>(
    writer: W,
    policy: &MaterializationPolicy,
    payloads: &[MaterializationPayload],
) -> Result<W> {
    // r[impl molten.filesystem_materialization.archive_members]
    let plan = plan_payloads(policy, payloads)?;
    let payload_map = validate_payloads(&plan, payloads)?;
    let mut builder = tar::Builder::new(writer);
    for member in &plan.members {
        let bytes = payload_map
            .get(&member.logical_path)
            .ok_or_else(|| invalid("archive payload disappeared after validation"))?;
        let mut header = tar::Header::new_gnu();
        header.set_size(member.expected_size);
        header.set_mode(ARCHIVE_READ_ONLY_MODE);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, member.logical_path.as_str(), std::io::Cursor::new(*bytes))
            .map_err(MoltenError::from)?;
    }
    builder.into_inner().map_err(MoltenError::from)
}

pub fn verify_archive<R: Read>(reader: R, policy: &MaterializationPolicy) -> Result<VerifiedArchive> {
    // r[impl molten.filesystem_materialization.archive_members]
    validate_policy(policy)?;
    let mut archive = tar::Archive::new(reader);
    let mut payloads = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total_bytes = 0u64;
    let entries = archive.entries().map_err(MoltenError::from)?;
    for entry_result in entries {
        if payloads.len() >= policy.max_members {
            return Err(invalid("archive member count exceeds materialization policy"));
        }
        let mut entry = entry_result.map_err(MoltenError::from)?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() {
            return Err(invalid("archive contains a link, directory, or unsupported special entry"));
        }
        let raw_name = entry.path_bytes();
        let name = std::str::from_utf8(raw_name.as_ref()).map_err(|_| invalid("archive member name must be UTF-8"))?;
        let logical_path = MaterializationPath::parse(name, policy.max_path_bytes)?;
        if policy.reserved_top_level_names.iter().any(|reserved| reserved == logical_path.top_level()) {
            return Err(invalid("archive member uses a reserved materialization name"));
        }
        if !seen.insert(logical_path.clone()) {
            return Err(invalid(format!("duplicate normalized archive member: {}", logical_path.as_str())));
        }
        let declared_size = entry.header().size().map_err(MoltenError::from)?;
        if declared_size > policy.max_member_bytes {
            return Err(invalid("archive member exceeds materialization byte bound"));
        }
        total_bytes =
            total_bytes.checked_add(declared_size).ok_or_else(|| invalid("archive total byte count overflow"))?;
        if total_bytes > policy.max_total_bytes {
            return Err(invalid("archive total bytes exceed materialization policy"));
        }
        let bytes = read_bounded(&mut entry, policy.max_member_bytes)?;
        if u64::try_from(bytes.len()).map_err(|_| invalid("archive member size does not fit u64"))? != declared_size {
            return Err(invalid("archive member byte count does not match header"));
        }
        payloads.push(MaterializationPayload::new(logical_path.as_str(), bytes));
    }
    let plan = plan_payloads(policy, &payloads)?;
    Ok(VerifiedArchive { plan, payloads })
}

pub fn create_explicit_output_file(path: &Path) -> Result<std::fs::File> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let leaf = path.file_name().ok_or_else(|| invalid("explicit output file path has no file name"))?;
    std::fs::create_dir_all(parent).map_err(MoltenError::from)?;
    let parent_dir =
        cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority()).map_err(MoltenError::from)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true).follow(FollowSymlinks::No);
    let file = parent_dir.open_with(Path::new(leaf), &options).map_err(MoltenError::from)?;
    if !file.metadata().map_err(MoltenError::from)?.is_file() {
        return Err(invalid("explicit output leaf must be a regular file"));
    }
    Ok(file.into_std())
}

pub fn open_explicit_input_file(path: &Path) -> Result<std::fs::File> {
    let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let leaf = path.file_name().ok_or_else(|| invalid("explicit input file path has no file name"))?;
    let parent_dir =
        cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority()).map_err(MoltenError::from)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = parent_dir.open_with(Path::new(leaf), &options).map_err(MoltenError::from)?;
    if !file.metadata().map_err(MoltenError::from)?.is_file() {
        return Err(invalid("explicit input leaf must be a regular file"));
    }
    Ok(file.into_std())
}

fn materialization_plan_value(
    policy: &MaterializationPolicy,
    members: &[MaterializationMember],
    total_bytes: u64,
) -> Result<preserves::IOValue> {
    let member_count =
        u64::try_from(members.len()).map_err(|_| invalid("materialization member count does not fit u64"))?;
    let maximum_members =
        u64::try_from(policy.max_members).map_err(|_| invalid("materialization member bound does not fit u64"))?;
    let maximum_path_bytes =
        u64::try_from(policy.max_path_bytes).map_err(|_| invalid("materialization path bound does not fit u64"))?;
    Ok(crate::preserves_rail::record("filesystem-materialization-plan-v1", vec![
        crate::preserves_rail::string(MATERIALIZATION_PLAN_SCHEMA),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(&policy.profile)]),
        crate::preserves_rail::record("replacement", vec![crate::preserves_rail::string(policy.replacement.as_str())]),
        crate::preserves_rail::record("members", vec![crate::preserves_rail::sequence(
            members
                .iter()
                .map(|member| {
                    crate::preserves_rail::record("member", vec![
                        crate::preserves_rail::string(member.logical_path.as_str()),
                        crate::preserves_rail::string(member.kind.as_str()),
                        crate::preserves_rail::string(&member.expected_content_ref),
                        crate::preserves_rail::u64_value(member.expected_size),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("summary", vec![
            crate::preserves_rail::u64_value(member_count),
            crate::preserves_rail::u64_value(total_bytes),
        ]),
        crate::preserves_rail::record("reserved-top-level", vec![crate::preserves_rail::sequence(
            policy.reserved_top_level_names.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("bounds", vec![
            crate::preserves_rail::u64_value(maximum_members),
            crate::preserves_rail::u64_value(policy.max_member_bytes),
            crate::preserves_rail::u64_value(policy.max_total_bytes),
            crate::preserves_rail::u64_value(maximum_path_bytes),
        ]),
    ]))
}

pub fn validate_materialization_receipt(receipt: &MaterializationReceipt) -> Result<()> {
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
        MaterializationPath::parse(path, HARD_MAX_MATERIALIZATION_PATH_BYTES)?;
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

pub fn parse_materialization_receipt(value: &preserves::IOValue) -> Result<MaterializationReceipt> {
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

fn parse_materialization_plan_value(value: &preserves::IOValue) -> Result<MaterializationPlan> {
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

fn required_record_fields(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
    arity: usize,
) -> Result<Vec<preserves::Value<preserves::IOValue>>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| invalid(format!("expected {label} record")))?;
    Ok(record.fields_iter().cloned().collect())
}

fn required_preserves_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| invalid(format!("expected string for {label}")))
}

fn required_preserves_u64(value: &preserves::Value<preserves::IOValue>, label: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| invalid(format!("expected u64 for {label}")))?
        .map_err(|error| invalid(format!("u64 out of range for {label}: {error}")))
}

fn required_preserves_usize(value: &preserves::Value<preserves::IOValue>, label: &str) -> Result<usize> {
    let value = required_preserves_u64(value, label)?;
    usize::try_from(value).map_err(|_| invalid(format!("u64 does not fit usize for {label}")))
}

fn required_named_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> Result<String> {
    let fields = required_record_fields(value, label, 1)?;
    required_preserves_string(&fields[0], label)
}

fn parse_named_string_sequence(value: &preserves::Value<preserves::IOValue>, label: &str) -> Result<Vec<String>> {
    let fields = required_record_fields(value, label, 1)?;
    let entries = fields[0]
        .collect_sequence()
        .ok_or_else(|| invalid(format!("expected string sequence for {label}")))?;
    if entries.len() > HARD_MAX_MATERIALIZATION_MEMBERS {
        return Err(invalid(format!("materialization receipt {label} exceeds item bound")));
    }
    entries.iter().map(|entry| required_preserves_string(entry, label)).collect()
}

fn parse_receipt_member_refs(value: &preserves::Value<preserves::IOValue>) -> Result<Vec<(String, String)>> {
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

fn parse_replacement_policy(value: &str) -> Result<ReplacementPolicy> {
    match value {
        "no-replace" => Ok(ReplacementPolicy::NoReplace),
        "replace-regular-files" => Ok(ReplacementPolicy::ReplaceRegularFiles),
        _ => Err(invalid(format!("unsupported materialization replacement policy {value}"))),
    }
}

fn build_materialization_receipt(plan: &MaterializationPlan) -> Result<MaterializationReceipt> {
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

fn materialization_receipt_value(input: &MaterializationReceiptValueInput<'_>) -> Result<preserves::IOValue> {
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
) -> Result<BTreeMap<MaterializationPath, &'a [u8]>> {
    validate_materialization_plan(plan)?;
    let planned_paths = plan
        .members
        .iter()
        .map(|member| (member.logical_path.as_str(), &member.logical_path))
        .collect::<BTreeMap<_, _>>();
    let mut payload_map = BTreeMap::new();
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

fn verify_payload_bytes(member: &MaterializationMember, bytes: &[u8]) -> Result<()> {
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

fn verify_member_bytes(member: &MaterializationMember, dir: &cap_std::fs::Dir, path: &Path) -> Result<()> {
    let bytes = read_regular_file_bounded(dir, path, member.expected_size)?;
    verify_payload_bytes(member, &bytes)
}

fn verify_published_members(dir: &cap_std::fs::Dir, plan: &MaterializationPlan) -> Result<()> {
    for member in &plan.members {
        verify_member_bytes(member, dir, member.logical_path.as_path())?;
    }
    Ok(())
}

fn validate_policy(policy: &MaterializationPolicy) -> Result<()> {
    validate_profile(&policy.profile)?;
    validate_bounds(policy.max_members, policy.max_member_bytes, policy.max_total_bytes, policy.max_path_bytes)?;
    let mut reserved = BTreeSet::new();
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
) -> Result<()> {
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

fn validate_profile(profile: &str) -> Result<()> {
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

fn validate_reserved_name(name: &str) -> Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(invalid("materialization reserved name must be one logical component"));
    }
    Ok(())
}

fn logical_path_from_relative_path(path: &Path) -> Result<String> {
    let mut components = Vec::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(invalid("materialization source path is not normalized"));
        };
        let component = component.to_str().ok_or_else(|| invalid("materialization source path must be UTF-8"))?;
        components.push(component);
    }
    if components.is_empty() {
        return Err(invalid("materialization source path cannot be empty"));
    }
    Ok(components.join("/"))
}

fn validate_materialization_path(value: &str, max_path_bytes: usize) -> Result<()> {
    if value.is_empty() {
        return Err(invalid("materialization member path cannot be empty"));
    }
    if value.len() > max_path_bytes {
        return Err(invalid("materialization member path exceeds configured byte bound"));
    }
    if value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value.contains('\0')
        || value.contains("://")
    {
        return Err(invalid("materialization member path is absolute or separator-ambiguous"));
    }
    let bytes = value.as_bytes();
    if bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':') {
        return Err(invalid("materialization member path has a platform prefix"));
    }
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(invalid("materialization member path contains an unsafe component"));
        }
    }
    for component in Path::new(value).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(invalid("materialization member path must be relative and normalized"));
        }
    }
    Ok(())
}

fn stage_path(plan: &MaterializationPlan) -> Result<PathBuf> {
    let token = plan
        .plan_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| invalid("materialization plan ref is not BLAKE3"))?;
    Ok(Path::new(STAGING_DIRECTORY).join(token))
}

fn create_staging_root(dir: &cap_std::fs::Dir, stage_path: &Path) -> Result<()> {
    match entry_kind(dir, Path::new(STAGING_DIRECTORY))? {
        None => dir.create_dir(STAGING_DIRECTORY).map_err(MoltenError::from)?,
        Some(MaterializationMemberKind::Directory) => {}
        Some(_) => return Err(invalid("materialization staging root must be a real directory")),
    }
    ensure_no_symlink_components(dir, stage_path.parent())?;
    dir.create_dir(stage_path).map_err(MoltenError::from)?;
    create_directory_tree(dir, Some(&stage_path.join(STAGING_TREE_DIRECTORY)))?;
    Ok(())
}

fn create_directory_tree(dir: &cap_std::fs::Dir, path: Option<&Path>) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = PathBuf::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(invalid("capability directory creation received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => dir.create_dir(&current).map_err(MoltenError::from)?,
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn create_directory_tree_recording(
    dir: &cap_std::fs::Dir,
    path: Option<&Path>,
    created: &mut Vec<PathBuf>,
) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = PathBuf::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(invalid("capability directory creation received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => {
                dir.create_dir(&current).map_err(MoltenError::from)?;
                created.push(current.clone());
            }
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn ensure_no_symlink_components(dir: &cap_std::fs::Dir, path: Option<&Path>) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = PathBuf::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(invalid("materialization parent check received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => return Ok(()),
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn entry_kind(dir: &cap_std::fs::Dir, path: &Path) -> Result<Option<MaterializationMemberKind>> {
    match dir.symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            let kind = if file_type.is_file() {
                MaterializationMemberKind::RegularFile
            } else if file_type.is_dir() {
                MaterializationMemberKind::Directory
            } else if file_type.is_symlink() {
                MaterializationMemberKind::Symlink
            } else {
                MaterializationMemberKind::Special
            };
            Ok(Some(kind))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(MoltenError::from(error)),
    }
}

fn write_create_new(dir: &cap_std::fs::Dir, path: &Path, bytes: &[u8]) -> Result<()> {
    ensure_no_symlink_components(dir, path.parent())?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(FollowSymlinks::No);
    let mut file = dir.open_with(path, &options).map_err(MoltenError::from)?;
    file.write_all(bytes).map_err(MoltenError::from)?;
    file.flush().map_err(MoltenError::from)
}

fn read_regular_file_bounded(dir: &cap_std::fs::Dir, path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    ensure_no_symlink_components(dir, path.parent())?;
    if entry_kind(dir, path)? != Some(MaterializationMemberKind::RegularFile) {
        return Err(invalid("materialization read target must be a regular file"));
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = dir.open_with(path, &options).map_err(MoltenError::from)?;
    read_bounded(&mut file, max_bytes)
}

fn read_bounded(reader: &mut impl Read, max_bytes: u64) -> Result<Vec<u8>> {
    let read_limit = max_bytes.checked_add(1).ok_or_else(|| invalid("materialization read bound overflow"))?;
    let mut bytes = Vec::new();
    reader.take(read_limit).read_to_end(&mut bytes).map_err(MoltenError::from)?;
    if u64::try_from(bytes.len()).map_err(|_| invalid("materialization read size does not fit u64"))? > max_bytes {
        return Err(invalid("materialization read exceeded configured byte bound"));
    }
    Ok(bytes)
}

struct PublicationState {
    final_path: PathBuf,
    backup_path: Option<PathBuf>,
    published: bool,
}

fn restore_current_backup(dir: &cap_std::fs::Dir, state: &PublicationState) -> Result<()> {
    let Some(backup) = state.backup_path.as_ref() else {
        return Ok(());
    };
    dir.rename(backup, dir, &state.final_path).map_err(MoltenError::from)
}

fn setup_failure(dir: &cap_std::fs::Dir, created_directories: &[PathBuf], primary: MoltenError) -> MoltenError {
    match rollback_created_directories(dir, created_directories) {
        Ok(()) => primary,
        Err(rollback) => invalid(format!(
            "materialization publication setup failed: {primary}; directory rollback failed: {rollback}"
        )),
    }
}

fn publication_failure(
    dir: &cap_std::fs::Dir,
    current: &PublicationState,
    prior: &[PublicationState],
    created_directories: &[PathBuf],
    primary: MoltenError,
) -> MoltenError {
    let current_result = restore_current_backup(dir, current);
    let prior_result = rollback_publication(dir, prior);
    let directory_result = rollback_created_directories(dir, created_directories);
    match (current_result, prior_result, directory_result) {
        (Ok(()), Ok(()), Ok(())) => primary,
        (current, prior, directories) => invalid(format!(
            "materialization publication failed: {primary}; current rollback: {current:?}; prior rollback: {prior:?}; directory rollback: {directories:?}"
        )),
    }
}

fn rollback_failure(
    dir: &cap_std::fs::Dir,
    states: &[PublicationState],
    created_directories: &[PathBuf],
    primary: MoltenError,
) -> MoltenError {
    let publication_result = rollback_publication(dir, states);
    let directory_result = rollback_created_directories(dir, created_directories);
    match (publication_result, directory_result) {
        (Ok(()), Ok(())) => primary,
        (publication, directories) => invalid(format!(
            "materialization verification failed: {primary}; publication rollback: {publication:?}; directory rollback: {directories:?}"
        )),
    }
}

fn rollback_created_directories(dir: &cap_std::fs::Dir, created_directories: &[PathBuf]) -> Result<()> {
    let mut diagnostics = Vec::new();
    for directory in created_directories.iter().rev() {
        match dir.remove_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => diagnostics.push(format!("remove directory {}: {error}", directory.display())),
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!("materialization directory rollback failed: {}", diagnostics.join("; "))))
    }
}

fn rollback_publication(dir: &cap_std::fs::Dir, states: &[PublicationState]) -> Result<()> {
    let mut diagnostics = Vec::new();
    for state in states.iter().rev() {
        if state.published {
            match dir.remove_file(&state.final_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => diagnostics.push(format!("remove {}: {error}", state.final_path.display())),
            }
        }
        if let Some(backup) = state.backup_path.as_ref()
            && let Err(error) = dir.rename(backup, dir, &state.final_path)
        {
            diagnostics.push(format!("restore {}: {error}", state.final_path.display()));
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!("materialization rollback failed: {}", diagnostics.join("; "))))
    }
}

fn invalid(message: impl Into<String>) -> MoltenError {
    MoltenError::invalid_harness(message.into())
}

fn remove_tree_if_present(dir: &cap_std::fs::Dir, path: &Path) -> Result<()> {
    match entry_kind(dir, path)? {
        None => Ok(()),
        Some(MaterializationMemberKind::Directory) => dir.remove_dir_all(path).map_err(MoltenError::from),
        Some(_) => Err(invalid("materialization stage path is not a real directory")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL_MAX_MEMBERS: usize = 8;
    const SMALL_MAX_MEMBER_BYTES: u64 = 1_024;
    const SMALL_MAX_TOTAL_BYTES: u64 = 4_096;
    const SMALL_MAX_PATH_BYTES: usize = 128;

    fn policy(replacement: ReplacementPolicy) -> MaterializationPolicy {
        MaterializationPolicy::bounded("test-bundle-v1", replacement)
            .expect("base policy")
            .with_bounds(SMALL_MAX_MEMBERS, SMALL_MAX_MEMBER_BYTES, SMALL_MAX_TOTAL_BYTES, SMALL_MAX_PATH_BYTES)
            .expect("bounded policy")
    }

    fn payloads() -> Vec<MaterializationPayload> {
        vec![
            MaterializationPayload::new("nested/b.preserves", b"bravo".to_vec()),
            MaterializationPayload::new("a.txt", b"alpha".to_vec()),
        ]
    }

    #[test]
    fn pure_plan_is_order_independent_and_portable() {
        // r[verify molten.filesystem_materialization.plan]
        // r[verify molten.filesystem_materialization.receipt]
        let policy = policy(ReplacementPolicy::NoReplace);
        let first_payloads = payloads();
        let mut reversed = first_payloads.clone();
        reversed.reverse();
        let first = plan_payloads(&policy, &first_payloads).expect("first plan");
        let second = plan_payloads(&policy, &reversed).expect("second plan");
        assert_eq!(first, second);
        assert_eq!(first.members[0].logical_path.as_str(), "a.txt");
        let mut first_policy_order = policy.clone();
        first_policy_order
            .reserved_top_level_names
            .extend(["z-reserved".to_string(), "a-reserved".to_string()]);
        let mut second_policy_order = first_policy_order.clone();
        second_policy_order.reserved_top_level_names.reverse();
        assert_eq!(
            plan_payloads(&first_policy_order, &first_payloads).expect("first reserved order"),
            plan_payloads(&second_policy_order, &first_payloads).expect("second reserved order")
        );

        let first_workspace =
            crate::test_support::process_workspace("materialize_portable_first").expect("first workspace");
        let second_workspace =
            crate::test_support::process_workspace("materialize_portable_second").expect("second workspace");
        let first_receipt =
            materialize_path(&first_workspace, &policy, &first_payloads).expect("first materialization");
        let second_receipt =
            materialize_path(&second_workspace, &policy, &first_payloads).expect("second materialization");
        assert_eq!(first_receipt.receipt_ref, second_receipt.receipt_ref);
        assert_eq!(first_receipt.decision, DECISION_PASS);
        assert!(first_receipt.non_claims.iter().any(|claim| claim == "not-release-eligibility"));
    }

    #[test]
    fn planner_rejects_unsafe_duplicate_reserved_and_over_bound_members() {
        // r[verify molten.filesystem_materialization.validation]
        let policy = policy(ReplacementPolicy::NoReplace);
        for path in [
            "",
            "/absolute",
            "../parent",
            "a/../parent",
            "a//ambiguous",
            "a\\windows",
            "C:/prefixed",
            ".molten-materialize/member",
        ] {
            let input = MaterializationMemberInput {
                logical_path: path.to_string(),
                kind: MaterializationMemberKind::RegularFile,
                expected_content_ref: crate::preserves_rail::content_ref_from_bytes(b"x"),
                expected_size: 1,
            };
            assert!(plan_materialization(&policy, &[input]).is_err(), "unsafe path accepted: {path}");
        }
        assert!(plan_payloads(&policy, &[]).is_err());
        let duplicate = MaterializationPayload::new("same", b"one".to_vec());
        assert!(plan_payloads(&policy, &[duplicate.clone(), duplicate]).is_err());
        let unsupported = MaterializationMemberInput {
            logical_path: "link".to_string(),
            kind: MaterializationMemberKind::Symlink,
            expected_content_ref: crate::preserves_rail::content_ref_from_bytes(b"x"),
            expected_size: 1,
        };
        assert!(plan_materialization(&policy, &[unsupported]).is_err());
        let oversized =
            MaterializationPayload::new("large", vec![
                0;
                usize::try_from(SMALL_MAX_MEMBER_BYTES).expect("small bound") + 1
            ]);
        assert!(plan_payloads(&policy, &[oversized]).is_err());
        let hard_ceiling = MaterializationPolicy::bounded("hard-ceiling-test-v1", ReplacementPolicy::NoReplace)
            .expect("hard-ceiling base policy")
            .with_bounds(
                HARD_MAX_MATERIALIZATION_MEMBERS.saturating_add(1),
                DEFAULT_MAX_MATERIALIZATION_MEMBER_BYTES,
                DEFAULT_MAX_MATERIALIZATION_TOTAL_BYTES,
                DEFAULT_MAX_MATERIALIZATION_PATH_BYTES,
            );
        assert!(hard_ceiling.is_err());
    }

    #[test]
    fn materialization_publishes_verified_receipts_and_replaces_only_when_selected() {
        // r[verify molten.filesystem_materialization.commit]
        // r[verify molten.filesystem_materialization.receipt]
        let root_path = crate::test_support::process_workspace("materialize_publish").expect("root");
        let root = MaterializationRoot::open(&root_path).expect("root capability");
        let payloads = payloads();
        let no_replace = policy(ReplacementPolicy::NoReplace);
        let plan = plan_payloads(&no_replace, &payloads).expect("plan");
        let receipt = root.materialize(&plan, &payloads).expect("materialize");
        assert!(receipt.valid());
        assert_eq!(receipt.plan_ref, plan.plan_ref);
        let receipt_text = crate::preserves_rail::to_text(&receipt.value).expect("receipt text");
        let reparsed_value = crate::preserves_rail::parse_text(&receipt_text).expect("receipt Preserves parse");
        let reparsed = parse_materialization_receipt(&reparsed_value).expect("typed receipt parse");
        assert_eq!(reparsed, receipt);
        let first_path = MaterializationPath::parse("a.txt", no_replace.max_path_bytes).expect("first payload path");
        assert_eq!(root.read(&first_path).expect("read"), b"alpha");

        let replacement = [MaterializationPayload::new("a.txt", b"replacement".to_vec())];
        let replace = policy(ReplacementPolicy::ReplaceRegularFiles);
        let replace_plan = plan_payloads(&replace, &replacement).expect("replacement plan");
        let replacement_receipt = root.materialize(&replace_plan, &replacement).expect("replace");
        assert!(replacement_receipt.valid());
        assert_eq!(std::fs::read(root_path.join("a.txt")).expect("replacement bytes"), b"replacement");

        let mut tampered_receipt = replacement_receipt;
        tampered_receipt.member_refs[0].1 = crate::preserves_rail::content_ref_from_bytes(b"tampered");
        assert!(!tampered_receipt.valid());
    }

    #[test]
    fn injected_mid_publication_failure_restores_replaced_members_without_a_receipt() {
        const FAIL_AFTER_FIRST_PUBLICATION: usize = 1;

        let root_path = crate::test_support::process_workspace("materialize_mid_commit_failure").expect("root");
        std::fs::write(root_path.join("a.txt"), b"old-a").expect("first original");
        let root = MaterializationRoot::open(&root_path).expect("root capability");
        let replacement = policy(ReplacementPolicy::ReplaceRegularFiles);
        let payloads = payloads();
        let plan = plan_payloads(&replacement, &payloads).expect("replacement plan");
        let staged = root.stage(&plan, &payloads).expect("stage");
        let result = root.commit_inner(&plan, &staged, Some(FAIL_AFTER_FIRST_PUBLICATION));
        assert!(result.is_err(), "fault injection must not return a passing receipt");
        assert_eq!(std::fs::read(root_path.join("a.txt")).expect("first restored"), b"old-a");
        assert!(!root_path.join("nested").exists(), "new destination directory must roll back");
        root.abort(&staged).expect("abort failed stage");
    }

    #[test]
    fn staged_commit_denies_wrong_root_stale_plan_partial_bytes_and_replacement() {
        // r[verify molten.filesystem_materialization.commit]
        // r[verify molten.filesystem_materialization.validation]
        let no_replace = policy(ReplacementPolicy::NoReplace);
        let payloads = payloads();
        let plan = plan_payloads(&no_replace, &payloads).expect("plan");
        let first_path = crate::test_support::process_workspace("materialize_stage_first").expect("first root");
        let second_path = crate::test_support::process_workspace("materialize_stage_second").expect("second root");
        let first = MaterializationRoot::open(&first_path).expect("first root");
        let second = MaterializationRoot::open(&second_path).expect("second root");
        let mut field_tampered_plan = plan.clone();
        field_tampered_plan.total_bytes = field_tampered_plan.total_bytes.saturating_add(1);
        assert!(first.stage(&field_tampered_plan, &payloads).is_err());
        let staged = first.stage(&plan, &payloads).expect("stage");
        assert!(second.commit(&plan, &staged).is_err());
        let other_plan =
            plan_payloads(&no_replace, &[MaterializationPayload::new("other", b"other".to_vec())]).expect("other plan");
        assert!(first.commit(&other_plan, &staged).is_err());
        first.abort(&staged).expect("abort stale stage");

        let mut tampered = payloads.clone();
        tampered[1].bytes = b"tampered".to_vec();
        assert!(first.stage(&plan, &tampered).is_err());
        assert!(!first.inner.dir.try_exists(stage_path(&plan).expect("stage path")).expect("stage absence"));

        std::fs::write(first_path.join("a.txt"), b"existing").expect("existing destination");
        assert!(first.materialize(&plan, &payloads).is_err());
        assert_eq!(std::fs::read(first_path.join("a.txt")).expect("existing survives"), b"existing");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_parent_and_leaf_cannot_redirect_materialization() {
        // r[verify molten.filesystem_materialization.root]
        let policy = policy(ReplacementPolicy::ReplaceRegularFiles);
        let outside = crate::test_support::process_workspace("materialize_outside").expect("outside root");
        let target = crate::test_support::process_workspace("materialize_symlink_target").expect("target root");
        std::fs::write(outside.join("outside.bin"), b"outside").expect("outside fixture");
        std::os::unix::fs::symlink(&*outside, target.join("linked-parent")).expect("parent symlink");
        let parent_payload = [MaterializationPayload::new(
            "linked-parent/outside.bin",
            b"overwrite".to_vec(),
        )];
        assert!(materialize_path(&target, &policy, &parent_payload).is_err());
        assert_eq!(std::fs::read(outside.join("outside.bin")).expect("outside survives"), b"outside");

        std::os::unix::fs::symlink(outside.join("outside.bin"), target.join("leaf.bin")).expect("leaf symlink");
        let leaf_payload = [MaterializationPayload::new("leaf.bin", b"overwrite".to_vec())];
        assert!(materialize_path(&target, &policy, &leaf_payload).is_err());
        assert_eq!(std::fs::read(outside.join("outside.bin")).expect("outside survives"), b"outside");
    }

    #[test]
    fn source_root_detects_tampered_member() {
        let source_path = crate::test_support::process_workspace("materialize_source").expect("source root");
        let payloads = payloads();
        for payload in &payloads {
            let path = source_path.join(&payload.logical_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("source parent");
            }
            std::fs::write(path, &payload.bytes).expect("source member");
        }
        let policy = policy(ReplacementPolicy::NoReplace);
        let plan = plan_payloads(&policy, &payloads).expect("source plan");
        let source = SourceDirectoryRoot::open_existing(&source_path).expect("source capability");
        assert_eq!(source.read_payloads(&policy, &plan).expect("source payloads").len(), payloads.len());
        std::fs::write(source_path.join("a.txt"), b"tampered").expect("tamper source");
        assert!(source.read_payloads(&policy, &plan).is_err());
        std::fs::remove_file(source_path.join("nested/b.preserves")).expect("remove required source member");
        assert!(source.read_payloads(&policy, &plan).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn source_capability_survives_root_replacement_and_rejects_links() {
        let source_path = crate::test_support::process_workspace("materialize_source_anchor").expect("source root");
        std::fs::write(source_path.join("member"), b"anchored").expect("member fixture");
        let source = SourceDirectoryRoot::open_existing(&source_path).expect("source capability");
        let moved = source_path.with_extension("moved");
        std::fs::rename(&source_path, &moved).expect("replace source root");
        std::fs::create_dir(&*source_path).expect("replacement source root");
        std::fs::write(source_path.join("member"), b"substitute").expect("substitute member");
        let policy = policy(ReplacementPolicy::NoReplace);
        let member = MaterializationPath::parse("member", policy.max_path_bytes).expect("member path");
        assert_eq!(source.read_path(&member, policy.max_member_bytes).expect("anchored read"), b"anchored");

        std::os::unix::fs::symlink(moved.join("member"), moved.join("linked")).expect("source link");
        assert!(source.list_regular_files_recursive(&policy).is_err());
    }

    #[test]
    fn archive_round_trip_is_bounded_and_rejects_duplicates_and_special_entries() {
        // r[verify molten.filesystem_materialization.archive_members]
        let policy = policy(ReplacementPolicy::NoReplace);
        let payloads = payloads();
        let archive_bytes = write_archive(Vec::new(), &policy, &payloads).expect("write archive");
        let verified = verify_archive(std::io::Cursor::new(archive_bytes.clone()), &policy).expect("verify archive");
        assert_eq!(verified.plan, plan_payloads(&policy, &payloads).expect("expected plan"));
        let mut corrupt_header = archive_bytes;
        corrupt_header[0] ^= 1;
        assert!(verify_archive(std::io::Cursor::new(corrupt_header), &policy).is_err());

        let duplicate_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "same", b"one", tar::EntryType::Regular);
            append_test_entry(&mut builder, "same", b"two", tar::EntryType::Regular);
            builder.into_inner().expect("duplicate archive")
        };
        assert!(verify_archive(std::io::Cursor::new(duplicate_bytes), &policy).is_err());

        let symlink_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "link", b"target", tar::EntryType::Symlink);
            builder.into_inner().expect("symlink archive")
        };
        assert!(verify_archive(std::io::Cursor::new(symlink_bytes), &policy).is_err());

        let traversal_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_raw_test_entry(&mut builder, b"../escape", b"escape", tar::EntryType::Regular);
            builder.into_inner().expect("traversal archive")
        };
        assert!(verify_archive(std::io::Cursor::new(traversal_bytes), &policy).is_err());

        const TINY_MAX_MEMBERS: usize = 1;
        const TINY_MAX_MEMBER_BYTES: u64 = 4;
        const TINY_MAX_TOTAL_BYTES: u64 = 4;
        const TINY_MAX_PATH_BYTES: usize = 128;
        let tiny = MaterializationPolicy::bounded("tiny-archive-v1", ReplacementPolicy::NoReplace)
            .expect("tiny policy")
            .with_bounds(TINY_MAX_MEMBERS, TINY_MAX_MEMBER_BYTES, TINY_MAX_TOTAL_BYTES, TINY_MAX_PATH_BYTES)
            .expect("tiny bounds");
        let oversized_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "large", b"large", tar::EntryType::Regular);
            builder.into_inner().expect("oversized archive")
        };
        assert!(verify_archive(std::io::Cursor::new(oversized_bytes), &tiny).is_err());

        let too_many_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "one", b"1", tar::EntryType::Regular);
            append_test_entry(&mut builder, "two", b"2", tar::EntryType::Regular);
            builder.into_inner().expect("too-many archive")
        };
        assert!(verify_archive(std::io::Cursor::new(too_many_bytes), &tiny).is_err());
    }

    fn append_test_entry(builder: &mut tar::Builder<Vec<u8>>, path: &str, bytes: &[u8], entry_type: tar::EntryType) {
        let mut header = test_header(bytes, entry_type);
        builder.append_data(&mut header, path, std::io::Cursor::new(bytes)).expect("append test entry");
    }

    fn append_raw_test_entry(
        builder: &mut tar::Builder<Vec<u8>>,
        path: &[u8],
        bytes: &[u8],
        entry_type: tar::EntryType,
    ) {
        let mut header = test_header(bytes, entry_type);
        header.as_mut_bytes()[..path.len()].copy_from_slice(path);
        header.set_cksum();
        builder.append(&header, std::io::Cursor::new(bytes)).expect("append raw test entry");
    }

    fn test_header(bytes: &[u8], entry_type: tar::EntryType) -> tar::Header {
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(bytes.len()).expect("test entry size"));
        header.set_entry_type(entry_type);
        if entry_type.is_symlink() {
            header.set_link_name("target").expect("link name");
        }
        header.set_mode(ARCHIVE_READ_ONLY_MODE);
        header.set_cksum();
        header
    }
}
