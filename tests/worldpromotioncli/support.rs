use super::*;

pub(super) fn run_reconcile(root: &std::path::Path) -> String {
    run_success(&["reconcile", "--state-root", root.to_str().expect("path")])
}

/// A previous process claimed the only reservation and stored an attempt, then
/// stopped before recording the dispatcher result (a lost completion).
pub(super) fn interrupt_attempt(storage: &molten_node_host::node_state::Scope) -> (String, WorldReleaseAttemptRef) {
    let attempt_ref = WorldReleaseAttemptRef::new(reference("interrupted-attempt")).expect("attempt ref");
    let mut store = LocalWorldPromotionStore::open(storage).expect("promotion store");
    let reservation_ref = store.list_reservations().expect("reservations")[0].reservation_ref.clone();
    let mut reservation = store.claim_reservation(&reservation_ref).expect("claim").expect("reservation");
    reservation.state = WorldReleaseState::Attempting;
    store.update_reservation(&reservation).expect("attempting reservation");
    store
        .store_attempt(&WorldAttemptRecord {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref.clone(),
            state: WorldReleaseState::Attempting,
            observation_ref: None,
            outcome: WorldAttemptOutcome::Unknown,
            external_completion_proven: false,
        })
        .expect("attempting attempt");
    (reservation_ref.to_string(), attempt_ref)
}

pub(super) struct Promoted {
    pub(super) temporary: tempfile::TempDir,
    pub(super) root: DirectoryRoot,
    pub(super) request_path: std::path::PathBuf,
    pub(super) authority_path: std::path::PathBuf,
    pub(super) closure_path: std::path::PathBuf,
    pub(super) signer_hex: String,
}

impl Promoted {
    pub(super) fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = DirectoryRoot::open(temporary.path()).expect("state root");
        root.create_layout().expect("layout");
        let policy_ref = WorldHeadPolicyRef::new(reference("policy")).expect("policy");
        seed(&root, &policy_ref);
        let (signer_hex, request_path, authority_path, closure_path) = fixtures(temporary.path(), &policy_ref);
        run_promote(temporary.path(), &request_path, &authority_path, &closure_path, &signer_hex);
        Self {
            temporary,
            root,
            request_path,
            authority_path,
            closure_path,
            signer_hex,
        }
    }

    pub(super) fn path(&self) -> &str {
        self.temporary.path().to_str().expect("path")
    }

    /// The promotion grant re-signed as an operator grant for `generation`.
    pub(super) fn operator_grant(&self, name: &str, generation: u64, admitted: bool) -> std::path::PathBuf {
        let path = self.temporary.path().join(name);
        signed::write_operator_authority(&self.authority_path, &path, generation, admitted);
        path
    }

    pub(super) fn store(&self) -> LocalWorldPromotionStore {
        LocalWorldPromotionStore::open(&self.root.namespace(NamespaceKind::Storage).expect("storage")).expect("store")
    }
}
