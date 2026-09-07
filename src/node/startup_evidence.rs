//! Capability adapter for portable source evidence. Verification is read-only;
//! lifecycle admission remains blocked until a real execution/build cohort is approved.
use std::io::Read;
use std::path::Path;

use cap_fs_ext::FollowSymlinks;
use cap_fs_ext::OpenOptionsFollowExt;
use cap_fs_ext::OpenOptionsSyncExt;
use cap_std::fs::Dir;
use cap_std::fs::OpenOptions;
use molten_core::node_startup::Descriptor;
use molten_core::node_startup::EvidencePlan;
use molten_core::node_startup::MAX_DESCRIPTOR_BYTES;
use molten_core::node_startup::TrustedCohort;

use crate::error::MoltenError;
use crate::error::Result;

type IoValue = preserves::IOValue;

/// A verified strict gate value. Verification alone does not authorize startup.
#[derive(Debug, Clone)]
pub struct AdmittedSourceGate {
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, serde::Serialize)]
pub struct VerificationReport {
    schema: &'static str,
    disposition: &'static str,
    descriptor_blake3: String,
    executable_blake3: String,
    source_inventory_blake3: String,
    strict_gate_ref: String,
    verified_members: usize,
    execution_established: bool,
    startup_authorized: bool,
}

// r[impl molten.startup_evidence.scope]
pub fn verify(policy_path: &Path, bundle_path: &Path) -> Result<VerificationReport> {
    let (root, plan) = load_plan(policy_path, bundle_path)?;
    verify_members(&root, &plan)
}

fn load_plan(policy_path: &Path, bundle_path: &Path) -> Result<(Dir, EvidencePlan)> {
    if !policy_path.is_absolute() || !bundle_path.is_absolute() {
        return Err(deny("paths-must-be-absolute"));
    }
    let parent = policy_path.parent().ok_or_else(|| deny("policy-path"))?;
    let leaf = policy_path.file_name().ok_or_else(|| deny("policy-path"))?;
    let policy_root = Dir::open_ambient_dir(parent, cap_std::ambient_authority())?;
    let policy_bytes = read_regular(&policy_root, Path::new(leaf), MAX_DESCRIPTOR_BYTES as u64)?;
    let policy: TrustedCohort = serde_json::from_slice(&policy_bytes).map_err(|_| deny("policy-json"))?;
    policy.validate().map_err(|_| deny("policy"))?;
    let root = Dir::open_ambient_dir(bundle_path, cap_std::ambient_authority())?;
    let descriptor_bytes = read_regular(&root, Path::new("bundle.json"), MAX_DESCRIPTOR_BYTES as u64)?;
    policy.check_descriptor_bytes(&descriptor_bytes).map_err(|_| deny("descriptor-identity"))?;
    let descriptor: Descriptor = serde_json::from_slice(&descriptor_bytes).map_err(|_| deny("descriptor-json"))?;
    let executable = measure_current_executable()?;
    let plan = EvidencePlan::admit(&policy, descriptor, &executable)
        .map_err(|error| MoltenError::invalid_harness(format!("startup-evidence-plan: {error:?}")))?;
    Ok((root, plan))
}

// r[impl molten.startup_evidence.scope]
pub fn admit_startup_source_gate(policy_path: &Path, bundle_path: &Path) -> Result<AdmittedSourceGate> {
    let (root, plan) = load_plan(policy_path, bundle_path)?;
    let _verified = verified_source_gate(&root, &plan)?;
    // No admitted production cohort exists yet. Caller-supplied policy and clean
    // snapshot counts cannot establish actual execution or source-to-binary binding.
    Err(deny("real-cohort-not-approved"))
}

fn verified_source_gate(root: &Dir, plan: &EvidencePlan) -> Result<AdmittedSourceGate> {
    let members = read_plan_members(root, plan)?;
    let gate = crate::quality::startup_snapshot::evaluate(crate::quality::startup_snapshot::Snapshot {
        plan,
        members: &members,
    })?;
    if gate.decision != "pass" {
        return Err(deny("strict-gate-denied"));
    }
    Ok(AdmittedSourceGate {
        receipt_ref: gate.receipt_ref,
        receipt_value: gate.receipt_value,
    })
}

fn read_plan_members(root: &Dir, plan: &EvidencePlan) -> Result<Vec<Vec<u8>>> {
    let mut members = Vec::with_capacity(plan.members().len());
    for (index, member) in plan.members().iter().enumerate() {
        let bytes = read_regular(root, Path::new(member.role.filename()), member.bytes)?;
        plan.verify_member(index, &bytes).map_err(|_| deny("member-identity"))?;
        members.push(bytes);
    }
    Ok(members)
}

fn verify_members(root: &Dir, plan: &EvidencePlan) -> Result<VerificationReport> {
    let gate = verified_source_gate(root, plan)?;
    Ok(VerificationReport {
        schema: "molten.node-startup-verification.v1",
        disposition: "verification-only",
        descriptor_blake3: plan.descriptor_blake3().into(),
        executable_blake3: plan.cohort().executable_blake3.clone(),
        source_inventory_blake3: plan.cohort().source_inventory_blake3.clone(),
        strict_gate_ref: gate.receipt_ref,
        verified_members: plan.members().len(),
        execution_established: false,
        startup_authorized: false,
    })
}

fn read_regular(root: &Dir, name: &Path, limit: u64) -> Result<Vec<u8>> {
    // Root is an explicit operator read grant; all descendant names are fixed leaves.
    let observed = root.symlink_metadata(name)?;
    if !observed.is_file() || observed.len() > limit {
        return Err(deny("not-bounded-regular-file"));
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let mut file = root.open_with(name, &options)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > limit {
        return Err(deny("not-bounded-regular-file"));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != before.len() || after.len() != before.len() || after.modified()? != before.modified()? {
        return Err(deny("file-changed"));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn measure_current_executable() -> Result<String> {
    // /proc/self/exe pins this running image even if its original pathname was replaced.
    let file = std::fs::File::open("/proc/self/exe")?;
    let limit = 512 * 1024 * 1024;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > limit {
        return Err(deny("executable-bounds"));
    }
    let mut reader = file.take(limit + 1);
    let mut hash = blake3::Hasher::new();
    let mut bytes = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = reader.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
        total += count as u64;
    }
    if total != meta.len() {
        return Err(deny("executable-changed"));
    }
    Ok(hash.finalize().to_hex().to_string())
}

#[cfg(not(target_os = "linux"))]
fn measure_current_executable() -> Result<String> {
    Err(deny("platform-unsupported"))
}

fn deny(code: &str) -> MoltenError {
    MoltenError::invalid_harness(format!("startup-evidence-{code}"))
}

#[cfg(test)]
#[path = "startup_evidence/tests.rs"]
mod tests;
