//! The node state roots the world producers capture, restore and replay in,
//! and the runtime-spine snapshot admission they publish.

const RUNTIME_STORE: &str = "ledger/world-snapshot/runtime.redb";
const ADMISSION_EXPIRY: u64 = u64::MAX;

/// One node state root holding the runtime-spine destination layout.
pub(super) struct NodeRoot {
    pub root: crate::node_state::DirectoryRoot,
    pub signer: ed25519_dalek::SigningKey,
}

impl NodeRoot {
    pub fn open(path: &std::path::Path, label: &str) -> crate::error::Result<Self> {
        let root = crate::node_state::DirectoryRoot::open(path)?;
        root.create_layout()?;
        for name in ["config.preserves", "identity.preserves"] {
            let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
                "aspen-qualification-node-v1",
                vec![
                    crate::preserves_rail::string(label),
                    crate::preserves_rail::string(name),
                ],
            ))?;
            root.write(&crate::node_state::RelativePath::parse(name)?, &bytes)?;
        }
        Ok(Self {
            root,
            signer: super::runtime::fixture_key("runtime-spine restore authority"),
        })
    }

    pub fn adapter(&self) -> crate::error::Result<crate::world_snapshot::LocalLogicalSnapshot> {
        crate::world_snapshot::LocalLogicalSnapshot::from_root(self.root.clone(), self.signer.verifying_key())
    }

    /// Publish the independently provisioned current admission and its signed
    /// grant. `granted` is what the grant signs; `current` is the live document.
    pub fn admit(&self, grant: SnapshotGrant<'_>) -> crate::error::Result<()> {
        let bytes = crate::world_snapshot::sign_local_admission(crate::world_snapshot::LocalAdmissionInput {
            signer: grant.signer,
            descriptor_ref: grant.descriptor_ref,
            profile_ref: grant.descriptor.profile_ref.as_str(),
            cohort_ref: grant.descriptor.cohort.cohort_ref.as_str(),
            destination_ref: self.adapter()?.destination_ref(),
            current: grant.granted.clone(),
        })?;
        let current = serde_json::to_vec(grant.current)
            .map_err(|error| super::invalid(format!("current admission encoding: {error}")))?;
        self.root
            .write(&crate::node_state::RelativePath::parse("storage/world-snapshot/current.json")?, &current)?;
        self.root
            .write(&crate::node_state::RelativePath::parse("storage/world-snapshot/admission.json")?, &bytes)
    }

    pub fn runtime_store_exists(&self) -> crate::error::Result<bool> {
        self.root.try_exists(&crate::node_state::RelativePath::parse(RUNTIME_STORE)?)
    }
}

pub(super) struct SnapshotGrant<'a> {
    pub signer: &'a ed25519_dalek::SigningKey,
    pub descriptor: &'a molten_core::world_snapshot::SnapshotDescriptor,
    pub descriptor_ref: &'a str,
    pub granted: &'a crate::world_snapshot::LocalCurrentAdmission,
    pub current: &'a crate::world_snapshot::LocalCurrentAdmission,
}

pub(super) fn descriptor_ref(
    descriptor: &molten_core::world_snapshot::SnapshotDescriptor,
) -> crate::error::Result<String> {
    Ok(crate::world_snapshot::canonical_snapshot_descriptor(descriptor)?.artifact_ref)
}

/// The current runtime-spine admission at one generation and revocation epoch.
pub(super) fn admission_document(
    generation: u64,
    revocation_label: &str,
    allowed: bool,
) -> crate::world_snapshot::LocalCurrentAdmission {
    crate::world_snapshot::LocalCurrentAdmission {
        policy_ref: super::runtime::policy_ref(),
        capability_ref: super::runtime::reference("aspen qualification runtime-spine restore capability"),
        revocation_ref: super::runtime::reference(revocation_label),
        resource_ref: super::runtime::reference("aspen qualification bounded runtime resources"),
        adapter_ref: super::runtime::reference("aspen qualification runtime-spine redb adapter"),
        generation,
        allowed,
        expires_unix_seconds: ADMISSION_EXPIRY,
    }
}
