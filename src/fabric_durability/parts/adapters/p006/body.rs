
// Snapshot payload publication through the pinned Durable File Publication
// contract.
//
// The payload is staged, synchronized, published under its content-addressed
// name without replacement, and its parent directory is synchronized before
// any snapshot metadata commits. A parent synchronization failure leaves the
// payload visible with unknown durability: the adapter commits no metadata and
// quarantines until a reopen reconciles. Startup and reopen check every
// snapshot payload's BLAKE3 identity before the adapter serves.

use cap_fs_ext::OpenOptionsFollowExt;
use durable_file_publication::shell::PublicationIo;
use durable_file_publication::shell::StageNameSource;

const SNAPSHOT_FINAL_MODE: u32 = 0o600;
const SNAPSHOT_STAGE_ATTEMPTS: usize = 8;

/// Wraps the publication I/O over the opened snapshot directory.
pub type PublicationIoFactory = std::sync::Arc<
    dyn Fn(durable_file_publication::shell::LinuxDirectory) -> Box<dyn PublicationIo> + Send + Sync,
>;

/// How the adapter publishes snapshot payloads.
#[derive(Clone)]
pub enum PublicationSource {
    /// The Linux adapter over the snapshot directory.
    Linux,
    /// A supplied wrapper around the Linux adapter.
    Wrapped(PublicationIoFactory),
}

struct DynPublicationIo(Box<dyn PublicationIo>);

impl PublicationIo for DynPublicationIo {
    fn supports_no_replace(&self) -> bool {
        self.0.supports_no_replace()
    }

    fn supports_parent_sync(&self) -> bool {
        self.0.supports_parent_sync()
    }

    fn create_stage(
        &mut self,
        leaf: &str,
        private_mode: u32,
    ) -> Result<durable_file_publication::shell::StageCreateDisposition, durable_file_publication::shell::MechanicalIoFailure>
    {
        self.0.create_stage(leaf, private_mode)
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.0.write_all(bytes)
    }

    fn apply_permissions(&mut self, mode: u32) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.0.apply_permissions(mode)
    }

    fn sync_payload(&mut self) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.0.sync_payload()
    }

    fn rename_stage(
        &mut self,
        stage_leaf: &str,
        destination_leaf: &str,
        replacement: durable_file_publication::core::ReplacementMode,
    ) -> Result<durable_file_publication::shell::RenameDisposition, durable_file_publication::shell::MechanicalIoFailure>
    {
        self.0.rename_stage(stage_leaf, destination_leaf, replacement)
    }

    fn sync_parent(&mut self) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.0.sync_parent()
    }

    fn cleanup_stage(&mut self, stage_leaf: &str) -> Result<(), durable_file_publication::shell::MechanicalIoFailure> {
        self.0.cleanup_stage(stage_leaf)
    }
}

struct SnapshotStageNames {
    stem: String,
    attempt: u64,
}

impl StageNameSource for SnapshotStageNames {
    fn next_stage_leaf(&mut self) -> String {
        self.attempt = self.attempt.saturating_add(1);
        format!(".stage-{}-{}", self.stem, self.attempt)
    }
}

/// Publish one snapshot payload under its content-addressed name, returning
/// how publication ended and, when nothing committed, the primitive failure.
// r[impl molten.fabric_durability.snapshot_publication]
fn publish_snapshot_payload(
    root: &crate::local_store::DurableStoreRoot,
    source: &PublicationSource,
    content_ref: &str,
    bytes: &[u8],
) -> crate::error::Result<(SnapshotPublicationObservation, Option<String>)> {
    let stem = snapshot_file_stem(content_ref)?;
    root.root().create_dir_all(&crate::local_store::Locator::parse(SNAPSHOT_DIRECTORY)?)?;
    // A readable descriptor, opened relative to the store capability without
    // following links: directory synchronization needs one.
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let directory = root.root().try_clone_dir()?.open_with(SNAPSHOT_DIRECTORY, &options).map_err(adapter_error)?.into_std();
    let linux = durable_file_publication::shell::LinuxDirectory::from_open_directory(directory).map_err(adapter_error)?;
    let mut io = match source {
        PublicationSource::Linux => DynPublicationIo(Box::new(linux)),
        PublicationSource::Wrapped(factory) => DynPublicationIo(factory(linux)),
    };
    let request = durable_file_publication::core::PublicationRequest {
        destination_leaf: format!("{stem}.bin"),
        payload_bytes: bytes.len(),
        maximum_payload_bytes: bytes.len().max(1),
        collision_attempt_limit: SNAPSHOT_STAGE_ATTEMPTS,
        final_mode: SNAPSHOT_FINAL_MODE,
        replacement: durable_file_publication::core::ReplacementMode::NoReplace,
        durability: durable_file_publication::core::DurabilityMode::DurabilityRequired,
    };
    let mut names = SnapshotStageNames {
        stem: stem.to_string(),
        attempt: 0,
    };
    let classification = durable_file_publication::shell::publish_one_file(&mut io, &mut names, &request, bytes)
        .map_err(|error| adapter_error(format!("snapshot publication request invalid: {error:?}")))?;
    Ok(match classification.disposition {
        durable_file_publication::core::PublicationDisposition::NotCommitted { failure, .. } => {
            (SnapshotPublicationObservation::NotCommitted, Some(format!("{failure:?}")))
        }
        durable_file_publication::core::PublicationDisposition::DestinationExists { .. } => {
            (SnapshotPublicationObservation::DestinationExists, None)
        }
        durable_file_publication::core::PublicationDisposition::CommittedAndParentSynchronized => {
            (SnapshotPublicationObservation::CommittedAndParentSynchronized, None)
        }
        durable_file_publication::core::PublicationDisposition::CommittedVisibilityOnly
        | durable_file_publication::core::PublicationDisposition::CommittedDurabilityUnknown => {
            (SnapshotPublicationObservation::CommittedDurabilityUnknown, None)
        }
    })
}

/// Whether the published payload of `content_ref` exists with that identity.
fn snapshot_payload_matches(root: &crate::local_store::DurableStoreRoot, content_ref: &str) -> bool {
    let Ok(stem) = snapshot_file_stem(content_ref) else {
        return false;
    };
    let Ok(locator) = crate::local_store::Locator::parse(&format!("{SNAPSHOT_DIRECTORY}/{stem}.bin")) else {
        return false;
    };
    root.root().read(&locator).is_ok_and(|bytes| blake3_ref(&bytes) == content_ref)
}

/// Whether every snapshot the state names has a payload with its identity.
fn snapshot_payloads_match(root: &crate::local_store::DurableStoreRoot, state: &DurableState) -> bool {
    state.snapshots.values().all(|snapshot| snapshot_payload_matches(root, &snapshot.content_ref))
}

impl RedbDurableStateAdapter {
    /// Publish the payload, then commit its metadata only after the payload
    /// and its parent are synchronized.
    // r[impl molten.fabric_durability.snapshot_publication]
    fn publish_then_commit_snapshot(
        &mut self,
        request: &SnapshotRequest,
        bytes: &[u8],
        transition: DurableTransition,
    ) -> crate::error::Result<TransitionEnvelope> {
        let (observation, failure) =
            publish_snapshot_payload(&self.root, &self.publication, &request.content_ref, bytes)?;
        match snapshot_metadata_decision(observation) {
            SnapshotMetadataDecision::CommitMetadata => {}
            SnapshotMetadataDecision::VerifyExistingThenCommit => {
                if !snapshot_payload_matches(&self.root, &request.content_ref) {
                    self.health = health_after(self.health, PersistenceClassification::CorruptOrInconsistent);
                    return Err(crate::error::Failure::invalid_harness(
                        "existing snapshot payload does not match its content identity",
                    ));
                }
            }
            SnapshotMetadataDecision::Abandon => {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "snapshot payload publication did not commit: {}",
                    failure.unwrap_or_default()
                )));
            }
            SnapshotMetadataDecision::QuarantineUntilReconciled => {
                self.health = health_after(self.health, PersistenceClassification::OutcomeUnknown);
                return Err(crate::error::Failure::invalid_harness(
                    "snapshot payload committed with unknown durability; metadata withheld until reconciliation",
                ));
            }
        }
        let attempt = persist_snapshot_metadata(&self.database, request, &transition.next);
        self.settle(request.durability, attempt, transition)
    }
}
