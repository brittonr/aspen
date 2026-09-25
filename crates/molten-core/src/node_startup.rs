//! Exact portable input plans. These values grant no startup or execution authority.

pub const POLICY_SCHEMA: &str = "molten.node-startup-cohort.v2";
pub const BUNDLE_SCHEMA: &str = "molten.node-startup-bundle.v2";
pub const BUILD_INPUTS_SCHEMA: &str = "molten.node-build-inputs.v1";
pub const OCTET_REVISION: &str = "c9b06bcf565c51d4a77d210e61b69ae51db9df25";
pub const MAX_DESCRIPTOR_BYTES: usize = 32_768; // 32 KiB
pub const MAX_MEMBER_BYTES: u64 = 8_388_608; // 8 MiB
pub const MAX_BUNDLE_BYTES: u64 = 33_554_432; // 32 MiB
pub const MAX_SOURCE_FILES: usize = 32_768;
pub const MAX_BUILD_UNITS: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedCohort {
    pub schema: String,
    pub descriptor_blake3: String,
    pub cohort: Cohort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    BuildInputs,
}

pub const ROLES: [MemberRole; 11] = [
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
    MemberRole::BuildInputs,
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
            Self::BuildInputs => "build-inputs.json",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub role: MemberRole,
    pub blake3: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        if self.schema != POLICY_SCHEMA {
            return Err(Rejection::Policy);
        }
        if hashes.iter().any(|hash| !is_hex(hash, 64)) {
            return Err(Rejection::Policy);
        }
        if !is_hex(&c.source_revision, 40) {
            return Err(Rejection::Policy);
        }
        if c.octet_revision != OCTET_REVISION {
            return Err(Rejection::Policy);
        }
        if c.build_toolchain != "nightly-2026-05-26" {
            return Err(Rejection::Policy);
        }
        if c.octet_toolchain != "nightly-2026-03-21-x86_64-unknown-linux-gnu" {
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
        if bytes.len() as u64 != expected.bytes {
            return Err(Rejection::MemberIdentity);
        }
        if blake3::hash(bytes).to_hex().as_str() != expected.blake3 {
            return Err(Rejection::MemberIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub name: String,
    pub blake3: String,
    pub bytes: u64,
}

/// Normalized first-party and dependency compiler inputs from an independently
/// retained build. This record binds claimed coverage, not compiler execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildInputs {
    pub schema: String,
    pub executable_target: String,
    pub units: Vec<BuildUnit>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildUnit {
    pub package: String,
    pub target: String,
    pub source_paths: Vec<String>,
}

fn validate_build_input_header(inputs: &BuildInputs) -> Result<(), Rejection> {
    if inputs.schema != BUILD_INPUTS_SCHEMA {
        return Err(Rejection::SourceContext);
    }
    if inputs.executable_target != "molten-node" {
        return Err(Rejection::SourceContext);
    }
    if inputs.units.is_empty() {
        return Err(Rejection::SourceContext);
    }
    if inputs.units.len() > MAX_BUILD_UNITS {
        return Err(Rejection::SourceContext);
    }
    Ok(())
}

fn valid_build_unit_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    if label.len() > 256 {
        return false;
    }
    label.bytes().all(|byte| byte.is_ascii_graphic())
}

fn validate_build_unit_metadata(unit: &BuildUnit, previous: Option<(&str, &str)>) -> Result<(), Rejection> {
    if previous.is_some_and(|prior| prior >= (unit.package.as_str(), unit.target.as_str())) {
        return Err(Rejection::SourceContext);
    }
    if matches!(unit.package.as_str(), "molten" | "molten-core") {
        return Err(Rejection::SourceContext);
    }
    if !valid_build_unit_label(&unit.package) {
        return Err(Rejection::SourceContext);
    }
    if !valid_build_unit_label(&unit.target) {
        return Err(Rejection::SourceContext);
    }
    if unit.source_paths.is_empty() {
        return Err(Rejection::SourceContext);
    }
    if unit.source_paths.len() > MAX_SOURCE_FILES {
        return Err(Rejection::SourceContext);
    }
    Ok(())
}

#[derive(Default)]
struct RequiredBuildRoots {
    libraries: [bool; 3],
    binary: bool,
}

fn record_required_build_roots(unit: &BuildUnit, roots: &mut RequiredBuildRoots) {
    let paths = &unit.source_paths;
    match (unit.package.as_str(), unit.target.as_str()) {
        ("molten-node-core", "lib") => {
            roots.libraries[0] = paths.iter().any(|path| path == "crates/molten-node-core/src/lib.rs");
        }
        ("molten-node-host", "lib") => {
            roots.libraries[1] = paths.iter().any(|path| path == "crates/molten-node-host/src/lib.rs");
        }
        ("molten-node-runtime", "lib") => {
            roots.libraries[2] = paths.iter().any(|path| path == "crates/molten-node-runtime/src/lib.rs");
        }
        ("molten-node-runtime", "bin/molten-node") => {
            roots.binary = paths.iter().any(|path| path == "crates/molten-node-runtime/src/bin/molten-node.rs");
        }
        _ => {}
    }
}

fn validate_build_unit_paths<'a>(
    unit: &'a BuildUnit,
    files: &[SourceFile],
    source_paths: &mut Vec<&'a str>,
) -> Result<(), Rejection> {
    let mut previous_path: Option<&str> = None;
    for path in &unit.source_paths {
        if !path.ends_with(".rs") {
            return Err(Rejection::SourceContext);
        }
        if previous_path.is_some_and(|previous| previous >= path.as_str()) {
            return Err(Rejection::SourceContext);
        }
        if files.binary_search_by(|file| file.name.cmp(path)).is_err() {
            return Err(Rejection::SourceContext);
        }
        previous_path = Some(path);
        if source_paths.len() >= MAX_SOURCE_FILES {
            return Err(Rejection::SourceContext);
        }
        source_paths.push(path.as_str());
    }
    Ok(())
}

pub fn validate_build_inputs(files: &[SourceFile], inputs: &BuildInputs) -> Result<(), Rejection> {
    validate_build_input_header(inputs)?;
    let mut previous = None;
    let mut roots = RequiredBuildRoots::default();
    let mut source_paths = Vec::new();
    for unit in &inputs.units {
        validate_build_unit_metadata(unit, previous)?;
        previous = Some((unit.package.as_str(), unit.target.as_str()));
        record_required_build_roots(unit, &mut roots);
        validate_build_unit_paths(unit, files, &mut source_paths)?;
    }
    if !roots.binary {
        return Err(Rejection::SourceContext);
    }
    if roots.libraries.contains(&false) {
        return Err(Rejection::SourceContext);
    }
    source_paths.sort_unstable();
    source_paths.dedup();
    if !source_paths
        .into_iter()
        .eq(files.iter().filter(|file| file.name.ends_with(".rs")).map(|file| file.name.as_str()))
    {
        return Err(Rejection::SourceContext);
    }
    Ok(())
}

/// This inventory binds the approved source snapshot. It never authorizes source-file reads.
// r[impl molten.startup_evidence.inputs]
pub fn validate_source_inventory(plan: &EvidencePlan, files: &[SourceFile]) -> Result<(), Rejection> {
    if files.is_empty() || files.len() > MAX_SOURCE_FILES {
        return Err(Rejection::SourceInventory);
    }
    let mut prior: Option<&str> = None;
    for file in files {
        if file.name.len() > 512 {
            return Err(Rejection::SourceInventory);
        }
        if !is_hex(&file.blake3, 64) {
            return Err(Rejection::SourceInventory);
        }
        if file.bytes > MAX_MEMBER_BYTES {
            return Err(Rejection::SourceInventory);
        }
        if file.name.split('/').any(|segment| segment.is_empty() || segment == "." || segment == "..") {
            return Err(Rejection::SourceInventory);
        }
        if !file.name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-/".contains(&byte)) {
            return Err(Rejection::SourceInventory);
        }
        if prior.is_some_and(|name| name >= file.name.as_str()) {
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
    // These anchors are necessary, not sufficient: the complete compiled closure
    // must also be independently reviewed against build inputs before approval.
    for required in [
        "crates/molten-node-core/Cargo.toml",
        "crates/molten-node-core/src/lib.rs",
        "crates/molten-node-host/Cargo.toml",
        "crates/molten-node-host/src/lib.rs",
        "crates/molten-node-runtime/Cargo.toml",
        "crates/molten-node-runtime/src/lib.rs",
        "crates/molten-node-runtime/src/bin/molten-node.rs",
        "crates/molten-node-runtime/src/node/daemon.rs",
        "crates/molten-node-runtime/src/node/runtime.rs",
        "crates/molten-node-runtime/src/node/content.rs",
        "crates/molten-node-runtime/src/node/startup_evidence.rs",
        "crates/molten-node-runtime/src/node/parts/daemon/p018/body.rs",
        "crates/molten-node-runtime/src/node/parts/daemon/p019/body.rs",
        "crates/molten-node-runtime/src/source_gate.rs",
        "crates/molten-core/src/content_store_adapter/node_service.rs",
        "crates/molten-core/src/node_startup.rs",
        "src/octet/startup_snapshot.rs",
        "crates/molten-node-host/src/node/state.rs",
    ] {
        if !files.iter().any(|file| file.name == required) {
            return Err(Rejection::SourceContext);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "node_startup/tests.rs"]
mod tests;
