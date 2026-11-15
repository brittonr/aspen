#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-snapshot CLI visibly composes closed operator DTOs and canonical artifacts"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the public world-snapshot spelling remains searchable at the command boundary"
)]

use std::path::Path;
use std::path::PathBuf;

use molten::error::Failure;
use molten::error::Result;
use molten::world_snapshot::canonical_snapshot_clone_plan;
use molten::world_snapshot::canonical_snapshot_compatibility;
use molten::world_snapshot::parse_canonical_snapshot_descriptor;
use molten_core::world_snapshot::CloneChild;
use molten_core::world_snapshot::ClonePlanRequest;
use molten_core::world_snapshot::CompatibilityVerdict;
use molten_core::world_snapshot::MAX_CLONE_CHILDREN;
use molten_core::world_snapshot::OverlayIdentity;
use molten_core::world_snapshot::SnapshotDescriptor;
use molten_core::world_snapshot::plan_restore;
use molten_core::world_snapshot::validate_snapshot;

const OVERLAY_DOMAIN: &str = "onixresearch.molten.world-snapshot.operator-overlay.v1";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldSnapshotCommand {
    /// Capture the closed committed-turn runtime-spine image (no authority grant).
    Capture {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        trusted_signer: String,
        #[arg(long)]
        commit_ref: String,
        #[arg(long)]
        descriptor_out: PathBuf,
    },
    /// Reconstruct and read back activated state without dispatching effects.
    Readback {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Inspect {
        #[arg(long)]
        descriptor: PathBuf,
    },
    Compatibility {
        #[arg(long)]
        descriptor: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
    RestorePlan {
        #[arg(long)]
        descriptor: PathBuf,
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        current_admission: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    ClonePlan {
        #[arg(long)]
        descriptor: PathBuf,
        #[arg(long)]
        children: u32,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Restore {
        #[arg(long)]
        descriptor: PathBuf,
        #[arg(long)]
        destination: PathBuf,
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        trusted_signer: String,
        #[arg(long)]
        receipt_out: PathBuf,
    },
}

pub(crate) fn run_world_snapshot_command(command: WorldSnapshotCommand) -> Result<()> {
    match command {
        WorldSnapshotCommand::Capture {
            image,
            state_root,
            trusted_signer,
            commit_ref,
            descriptor_out,
        } => capture(&image, &state_root, &trusted_signer, &commit_ref, &descriptor_out),
        WorldSnapshotCommand::Readback { state_root, out } => readback(&state_root, &out),
        WorldSnapshotCommand::Inspect { descriptor } => inspect(&descriptor),
        WorldSnapshotCommand::Compatibility {
            descriptor,
            destination,
        } => compatibility(&descriptor, &destination),
        WorldSnapshotCommand::RestorePlan {
            descriptor,
            destination,
            current_admission,
            out,
        } => restore_plan(&descriptor, &destination, current_admission, out.as_deref()),
        WorldSnapshotCommand::ClonePlan {
            descriptor,
            children,
            out,
        } => clone_plan(&descriptor, children, out.as_deref()),
        WorldSnapshotCommand::Restore {
            descriptor,
            destination,
            state_root,
            trusted_signer,
            receipt_out,
        } => restore(&descriptor, &destination, &state_root, &trusted_signer, &receipt_out),
    }
}

fn inspect(path: &Path) -> Result<()> {
    let (descriptor, canonical) = load_descriptor(path)?;
    println!("descriptor_ref={}", canonical.artifact_ref);
    println!("class={}", descriptor.class.as_str());
    println!("commit_ref={}", descriptor.commit_ref);
    println!("profile_ref={}", descriptor.profile_ref);
    println!("cohort_ref={}", descriptor.cohort.cohort_ref);
    println!("component_count={}", descriptor.components.len());
    println!("contains_live_handle={}", descriptor.contains_live_handle);
    Ok(())
}

fn compatibility(source: &Path, destination: &Path) -> Result<()> {
    let (descriptor, _) = load_descriptor(source)?;
    let (target, _) = load_descriptor(destination)?;
    let report = validate_snapshot(&descriptor, &target.cohort);
    let canonical = canonical_snapshot_compatibility(&report)?;
    println!("compatibility_ref={}", canonical.artifact_ref);
    println!("verdict={}", report.verdict.as_str());
    for issue in &report.issues {
        println!("issue={issue:?}");
    }
    if report.verdict != CompatibilityVerdict::Compatible {
        return Err(Failure::invalid_harness("world snapshot compatibility denied"));
    }
    Ok(())
}

fn restore_plan(source: &Path, destination: &Path, current_admission: bool, out: Option<&Path>) -> Result<()> {
    let (descriptor, _) = load_descriptor(source)?;
    let (target, _) = load_descriptor(destination)?;
    let plan = plan_restore(&descriptor, &target.cohort, current_admission).map_err(|report| {
        Failure::invalid_harness(format!("world snapshot restore planning denied: {:?}", report.issues))
    })?;
    let canonical = molten::world_snapshot::canonical_snapshot_restore_plan(&plan)?;
    write_optional(out, &canonical.bytes)?;
    println!("restore_plan_ref={}", canonical.artifact_ref);
    println!("step_count={}", plan.steps.len());
    for step in plan.steps {
        println!("step={}", step.as_str());
    }
    Ok(())
}

fn clone_plan(source: &Path, children: u32, out: Option<&Path>) -> Result<()> {
    let (descriptor, _) = load_descriptor(source)?;
    let child_count =
        usize::try_from(children).map_err(|_| Failure::invalid_harness("world snapshot child count exceeds usize"))?;
    if child_count == 0 || child_count > MAX_CLONE_CHILDREN {
        return Err(Failure::invalid_harness("world snapshot child count is outside the reviewed bound"));
    }
    let request = ClonePlanRequest {
        parent_ref: descriptor.commit_ref.clone(),
        children: (0..children).map(|index| clone_child(&descriptor, index)).collect::<Result<Vec<_>>>()?,
    };
    let canonical = canonical_snapshot_clone_plan(&request)?;
    write_optional(out, &canonical.bytes)?;
    println!("clone_plan_ref={}", canonical.artifact_ref);
    println!("child_count={children}");
    Ok(())
}

fn restore(
    source: &Path,
    destination: &Path,
    state_root: &Path,
    trusted_signer: &str,
    receipt_out: &Path,
) -> Result<()> {
    let (descriptor, canonical_descriptor) = load_descriptor(source)?;
    let (target, _) = load_descriptor(destination)?;
    let adapter = molten::world_snapshot::LocalLogicalSnapshot::open(state_root, parse_signer(trusted_signer)?)?;
    let outcome = molten::world_snapshot::restore_local_logical_snapshot(adapter, &descriptor, &target.cohort)?;
    std::fs::write(receipt_out, &outcome.restore.receipt.bytes).map_err(|error| Failure::invalid_harness(format!(
        "activation=completed canonical-receipt=stored receipt-output=failed cleanup=retained-for-reconciliation: {error}"
    )))?;
    println!("decision=restored");
    println!("profile={}", molten::world_snapshot::LOCAL_LOGICAL_RUNTIME_PROFILE);
    println!("descriptor_ref={}", canonical_descriptor.artifact_ref);
    println!("receipt_ref={}", outcome.restore.receipt.artifact_ref);
    println!("runtime_state_ref={}", outcome.runtime.snapshot().snapshot_ref()?);
    println!("observation_count={}", outcome.restore.observations.len());
    Ok(())
}

fn parse_signer(trusted_signer: &str) -> Result<ed25519_dalek::VerifyingKey> {
    let bytes = hex::decode(trusted_signer)
        .map_err(|_| Failure::invalid_harness("trusted signer must be 64 hex characters"))?;
    let bytes: [u8; 32] =
        bytes.try_into().map_err(|_| Failure::invalid_harness("trusted signer must be 64 hex characters"))?;
    ed25519_dalek::VerifyingKey::from_bytes(&bytes)
        .map_err(|error| Failure::invalid_harness(format!("trusted signer: {error}")))
}

fn capture(image: &Path, state_root: &Path, signer: &str, commit_ref: &str, out: &Path) -> Result<()> {
    let bytes = read_bounded(image, molten::world_snapshot::MAX_LOCAL_LOGICAL_IMAGE_BYTES)?;
    let image = molten::world_snapshot::LogicalRuntimeImage::from_bytes(&bytes)?;
    let commit = molten_core::world_commit::WorldCommitRef::new(commit_ref.to_string())
        .map_err(|error| Failure::invalid_harness(format!("snapshot commit: {error:?}")))?;
    let adapter = molten::world_snapshot::LocalLogicalSnapshot::open(state_root, parse_signer(signer)?)?;
    let descriptor = adapter.prepare_bundle(&image, commit)?;
    let canonical = molten::world_snapshot::canonical_snapshot_descriptor(&descriptor)?;
    std::fs::write(out, &canonical.bytes).map_err(Failure::from)?;
    println!("profile={}", molten::world_snapshot::LOCAL_LOGICAL_RUNTIME_PROFILE);
    println!("descriptor_ref={}", canonical.artifact_ref);
    println!("destination_ref={}", adapter.destination_ref());
    Ok(())
}

fn readback(state_root: &Path, out: &Path) -> Result<()> {
    let root = molten::node_state::DirectoryRoot::open_existing(state_root)?;
    let readback = molten::world_snapshot::read_local_logical_runtime(&root)?;
    std::fs::write(out, &readback.canonical_image).map_err(Failure::from)?;
    println!("descriptor_ref={}", readback.descriptor_ref);
    println!("activation_ref={}", readback.activation_ref);
    println!("runtime_state_ref={}", readback.runtime.snapshot().snapshot_ref()?);
    println!("unresolved_effects={} dispatched_effects=0", readback.unresolved_effects.len());
    Ok(())
}

fn load_descriptor(path: &Path) -> Result<(SnapshotDescriptor, molten::world_snapshot::CanonicalSnapshotArtifact)> {
    let bytes = read_bounded(path, molten_core::world_snapshot::MAX_SNAPSHOT_CANONICAL_BYTES as u64)?;
    parse_canonical_snapshot_descriptor(&bytes)
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(Failure::from)?;
    if file.metadata().map_err(Failure::from)?.len() > maximum {
        return Err(Failure::invalid_harness("snapshot operator input exceeds byte bound"));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes).map_err(Failure::from)?;
    if bytes.len() as u64 > maximum {
        return Err(Failure::invalid_harness("snapshot operator input grew beyond byte bound"));
    }
    Ok(bytes)
}

fn clone_child(descriptor: &SnapshotDescriptor, index: u32) -> Result<CloneChild> {
    Ok(CloneChild {
        parent_ref: descriptor.commit_ref.clone(),
        memory_overlay: overlay(descriptor, index, "memory")?,
        device_overlay: overlay(descriptor, index, "device")?,
        disk_overlay: overlay(descriptor, index, "disk")?,
        endpoint_overlay: overlay(descriptor, index, "endpoint")?,
    })
}

fn overlay(descriptor: &SnapshotDescriptor, index: u32, surface: &str) -> Result<OverlayIdentity> {
    let mut hasher = blake3::Hasher::new_derive_key(OVERLAY_DOMAIN);
    update_text(&mut hasher, descriptor.commit_ref.as_str())?;
    hasher.update(&index.to_le_bytes());
    update_text(&mut hasher, surface)?;
    Ok(OverlayIdentity(format!("blake3:{}", hasher.finalize().to_hex())))
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length =
        u64::try_from(value.len()).map_err(|_| Failure::invalid_harness("world snapshot CLI value exceeds u64"))?;
    hasher.update(&length.to_le_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn write_optional(path: Option<&Path>, bytes: &[u8]) -> Result<()> {
    if let Some(path) = path {
        std::fs::write(path, bytes).map_err(Failure::from)?;
    }
    Ok(())
}
