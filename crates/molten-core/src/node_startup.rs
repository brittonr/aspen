//! Exact portable input plans. These values grant no startup or execution authority.
use serde::{Deserialize, Serialize};

pub const POLICY_SCHEMA: &str = "molten.node-startup-cohort.v1";
pub const BUNDLE_SCHEMA: &str = "molten.node-startup-bundle.v1";
pub const OCTET_REVISION: &str = "c9b06bcf565c51d4a77d210e61b69ae51db9df25";
pub const MAX_DESCRIPTOR_BYTES: usize = 32 * 1024;
pub const MAX_MEMBER_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_BUNDLE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_SOURCE_FILES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    pub source_revision: String,
    pub source_inventory_blake3: String,
    pub executable_blake3: String,
    pub build_rustc_blake3: String,
    pub build_toolchain: String,
    pub octet_revision: String,
    pub octet_cli_blake3: String,
    pub octet_driver_blake3: String,
    pub octet_lints_blake3: String,
    pub octet_rustc_blake3: String,
    pub octet_toolchain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedCohort {
    pub schema: String,
    pub descriptor_blake3: String,
    pub cohort: Cohort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemberRole {
    CargoManifest,
    DylintConfig,
    CargoLock,
    FlakeLock,
    RustToolchain,
    SourceInventory,
    Command,
    Status,
    Summary,
    ObjectCorpus,
}

pub const ROLES: [MemberRole; 10] = [
    MemberRole::CargoManifest,
    MemberRole::DylintConfig,
    MemberRole::CargoLock,
    MemberRole::FlakeLock,
    MemberRole::RustToolchain,
    MemberRole::SourceInventory,
    MemberRole::Command,
    MemberRole::Status,
    MemberRole::Summary,
    MemberRole::ObjectCorpus,
];

impl MemberRole {
    pub fn filename(self) -> &'static str {
        match self {
            Self::CargoManifest => "Cargo.toml",
            Self::DylintConfig => "dylint.toml",
            Self::CargoLock => "Cargo.lock",
            Self::FlakeLock => "flake.lock",
            Self::RustToolchain => "rust-toolchain.toml",
            Self::SourceInventory => "source-inventory.json",
            Self::Command => "command.txt",
            Self::Status => "status.json",
            Self::Summary => "summary.txt",
            Self::ObjectCorpus => "object-corpus-receipt.json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub role: MemberRole,
    pub blake3: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub schema: String,
    pub cohort: Cohort,
    pub members: Vec<Member>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Policy,
    DescriptorIdentity,
    DescriptorShape,
    Cohort,
    Executable,
    MemberInventory,
    Bounds,
    MemberIdentity,
    SourceInventory,
    SourceContext,
}

/// A finite read plan, not a verified gate or permission to start a node.
#[derive(Debug)]
pub struct EvidencePlan {
    cohort: Cohort,
    descriptor_blake3: String,
    members: Vec<Member>,
}

pub fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl TrustedCohort {
    pub fn validate(&self) -> Result<(), Rejection> {
        let c = &self.cohort;
        let hashes = [
            &self.descriptor_blake3,
            &c.source_inventory_blake3,
            &c.executable_blake3,
            &c.build_rustc_blake3,
            &c.octet_cli_blake3,
            &c.octet_driver_blake3,
            &c.octet_lints_blake3,
            &c.octet_rustc_blake3,
        ];
        if self.schema != POLICY_SCHEMA
            || hashes.iter().any(|s| !is_hex(s, 64))
            || !is_hex(&c.source_revision, 40)
            || c.octet_revision != OCTET_REVISION
            || c.build_toolchain != "nightly-2026-05-26"
            || c.octet_toolchain != "nightly-2026-03-21-x86_64-unknown-linux-gnu"
        {
            return Err(Rejection::Policy);
        }
        Ok(())
    }

    // Called before JSON decoding and before any member read.
    pub fn check_descriptor_bytes(&self, bytes: &[u8]) -> Result<(), Rejection> {
        self.validate()?;
        if bytes.is_empty() || bytes.len() > MAX_DESCRIPTOR_BYTES {
            return Err(Rejection::Bounds);
        }
        if blake3::hash(bytes).to_hex().as_str() != self.descriptor_blake3 {
            return Err(Rejection::DescriptorIdentity);
        }
        Ok(())
    }
}

// r[impl molten.startup_evidence.inputs]
impl EvidencePlan {
    pub fn admit(policy: &TrustedCohort, descriptor: Descriptor, executable: &str) -> Result<Self, Rejection> {
        policy.validate()?;
        if descriptor.schema != BUNDLE_SCHEMA {
            return Err(Rejection::DescriptorShape);
        }
        if descriptor.cohort != policy.cohort {
            return Err(Rejection::Cohort);
        }
        if executable != policy.cohort.executable_blake3 {
            return Err(Rejection::Executable);
        }
        if descriptor.members.len() != ROLES.len() {
            return Err(Rejection::MemberInventory);
        }
        let mut total = 0_u64;
        for (member, role) in descriptor.members.iter().zip(ROLES) {
            if member.role != role || !is_hex(&member.blake3, 64) {
                return Err(Rejection::MemberInventory);
            }
            if member.bytes == 0 || member.bytes > MAX_MEMBER_BYTES {
                return Err(Rejection::Bounds);
            }
            total = total.checked_add(member.bytes).ok_or(Rejection::Bounds)?;
        }
        if total > MAX_BUNDLE_BYTES {
            return Err(Rejection::Bounds);
        }
        if descriptor.members[5].blake3 != policy.cohort.source_inventory_blake3 {
            return Err(Rejection::SourceInventory);
        }
        Ok(Self {
            cohort: descriptor.cohort,
            descriptor_blake3: policy.descriptor_blake3.clone(),
            members: descriptor.members,
        })
    }

    pub fn cohort(&self) -> &Cohort {
        &self.cohort
    }
    pub fn descriptor_blake3(&self) -> &str {
        &self.descriptor_blake3
    }
    pub fn members(&self) -> &[Member] {
        &self.members
    }
    pub fn verify_member(&self, index: usize, bytes: &[u8]) -> Result<(), Rejection> {
        let expected = self.members.get(index).ok_or(Rejection::MemberInventory)?;
        if bytes.len() as u64 != expected.bytes || blake3::hash(bytes).to_hex().as_str() != expected.blake3 {
            return Err(Rejection::MemberIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub name: String,
    pub blake3: String,
    pub bytes: u64,
}

/// This inventory binds the approved source snapshot. It never authorizes source-file reads.
// r[impl molten.startup_evidence.inputs]
pub fn validate_source_inventory(plan: &EvidencePlan, files: &[SourceFile]) -> Result<(), Rejection> {
    if files.is_empty() || files.len() > MAX_SOURCE_FILES {
        return Err(Rejection::SourceInventory);
    }
    let mut prior: Option<&str> = None;
    for file in files {
        if file.name.len() > 512
            || !is_hex(&file.blake3, 64)
            || file.bytes > MAX_MEMBER_BYTES
            || file.name.split('/').any(|s| s.is_empty() || s == "." || s == "..")
            || !file.name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            || prior.is_some_and(|name| name >= file.name.as_str())
        {
            return Err(Rejection::SourceInventory);
        }
        prior = Some(&file.name);
    }
    for member in &plan.members[..5] {
        if !files.iter().any(|file| {
            file.name == member.role.filename() && file.blake3 == member.blake3 && file.bytes == member.bytes
        }) {
            return Err(Rejection::SourceContext);
        }
    }
    // Include both the historical gate scopes and the new source owners, not just facade names.
    for required in [
        "src/job/dag.rs",
        "src/main.rs",
        "src/node/daemon.rs",
        "src/node/runtime.rs",
        "src/octet/gate.rs",
        "src/upgrades/mod.rs",
        "src/node/content.rs",
        "src/node/parts/daemon/p018/body.rs",
        "src/node/parts/daemon/p019/body.rs",
        "crates/molten-core/src/content_store_adapter/node_service.rs",
        "crates/molten-core/src/node_startup.rs",
        "src/octet/startup_snapshot.rs",
        "src/node/startup_evidence.rs",
    ] {
        if !files.iter().any(|file| file.name == required) {
            return Err(Rejection::SourceContext);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
