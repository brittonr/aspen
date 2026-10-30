pub(super) struct Fixture {
    pub workspace: crate::test_support::Workspace,
    pub root: molten_node_host::node_state::DirectoryRoot,
}

impl Fixture {
    pub fn new(label: &str) -> Self {
        let workspace = crate::test_support::Workspace::new(label).unwrap();
        let state = workspace.state().unwrap();
        let root = molten_node_host::node_state::DirectoryRoot::from_dir(state.dir().try_clone().unwrap());
        root.create_layout().unwrap();
        Self { workspace, root }
    }

    pub fn open(&self) -> crate::world_head::LocalWorldHeadStore {
        crate::world_head::LocalWorldHeadStore::open(
            &self.root.namespace(molten_node_host::node_state::NamespaceKind::Storage).unwrap(),
        )
        .unwrap()
    }

    pub fn path(&self) -> std::path::PathBuf {
        let state = self.workspace.state().unwrap();
        self.workspace.process_bridge().plan(&state).unwrap().path().join("storage/world-heads.redb")
    }

    /// Seeds a preexisting state fixture, not an authenticated or authorized mutation.
    pub fn seed(&self, claim: &release_channel_index::model::Claim, revision: u64) {
        let mut head_store = self.open();
        let _ = crate::world_witness::Database::new(&mut head_store).unwrap();
        let highest = highest(claim, revision);
        let head = molten_core::world_head::WorldHeadState {
            branch_id: highest.subject.branch_id.clone(),
            branch_class: molten_core::world_head::WorldBranchClass::Local,
            head: molten_core::world_commit::WorldCommitRef::new(format!(
                "blake3:{}",
                blake3::hash(&claim.digest).to_hex()
            ))
            .unwrap(),
            generation: claim.generation,
            policy_ref: molten_core::world_head::WorldHeadPolicyRef::new(format!("blake3:{}", "a".repeat(64))).unwrap(),
        };
        let (_, bytes) = crate::world_head::canonical_world_head_state(&head).unwrap();
        let highest_bytes = crate::world_witness::canonical_highest(&highest).unwrap();
        let write = head_store.database().begin_write().unwrap();
        write
            .open_table(redb::TableDefinition::<&str, &[u8]>::new("world_heads_v1"))
            .unwrap()
            .insert(head.branch_id.as_str(), bytes.as_slice())
            .unwrap();
        write
            .open_table(crate::world_witness::store::HIGHEST)
            .unwrap()
            .insert(head.branch_id.as_str(), highest_bytes.as_slice())
            .unwrap();
        write.commit().unwrap();
    }

    pub fn retained(&self) -> molten_core::world_witness::HighestWitnessState {
        let mut head_store = self.open();
        match crate::world_witness::Database::new(&mut head_store).unwrap().retained(&branch()).unwrap() {
            molten_core::world_witness::RetainedWitnessState::Present(highest) => highest,
            other => panic!("expected fixture highest state, got {other:?}"),
        }
    }
}

pub(super) fn branch() -> molten_core::world_head::WorldBranchId {
    molten_core::world_head::WorldBranchId::new("main").unwrap()
}

fn highest(
    claim: &release_channel_index::model::Claim,
    revision: u64,
) -> molten_core::world_witness::HighestWitnessState {
    molten_core::world_witness::HighestWitnessState {
        subject: molten_core::world_witness::WitnessSubject {
            branch_id: branch(),
            generation: claim.generation,
            claim_ref: molten_core::world_head::WorldHeadClaimRef::new(format!(
                "blake3:{}",
                claim.digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
            ))
            .unwrap(),
        },
        checkpoint: molten_core::world_witness::SignedCheckpoint {
            provider_id: "fixture-provider".into(),
            checkpoint_id: format!("fixture-checkpoint-{revision}"),
            revision,
            signature_verified: false,
            observed_at: 100,
        },
        consistency_predecessor: None,
        quorum: std::collections::BTreeSet::from(["fixture-provider".into()]),
    }
}
