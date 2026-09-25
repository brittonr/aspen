
#[derive(Default)]
struct FixtureStore {
    commits: BTreeMap<String, Vec<u8>>,
    roots: BTreeMap<String, Vec<u8>>,
}

impl WorldCommitPublicationPort for FixtureStore {
    fn publish_commit(
        &mut self,
        _commit_ref: &WorldCommitRef,
        _canonical_bytes: &[u8],
    ) -> std::result::Result<PublicationOutcome, WorldCommitPortError> {
        Err(WorldCommitPortError::new("fixture", "publication disabled"))
    }

    fn read_commit(&self, commit_ref: &WorldCommitRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.commits
            .get(commit_ref.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("fixture", "missing commit"))
    }
}

impl WorldImmutableObjectPort for FixtureStore {
    fn contains_root(&self, root: &WorldRootRef) -> std::result::Result<bool, WorldCommitPortError> {
        Ok(self.roots.contains_key(root.as_str()))
    }

    fn persist_root(
        &mut self,
        _root: &WorldRootRef,
        _canonical_bytes: &[u8],
    ) -> std::result::Result<(), WorldCommitPortError> {
        Err(WorldCommitPortError::new("fixture", "persistence disabled"))
    }

    fn read_root(&self, root: &WorldRootRef) -> std::result::Result<Vec<u8>, WorldCommitPortError> {
        self.roots
            .get(root.as_str())
            .cloned()
            .ok_or_else(|| WorldCommitPortError::new("fixture", "missing root"))
    }
}

fn fixture_store() -> (FixtureStore, CanonicalWorldCommit) {
    let mut store = FixtureStore::default();
    let roots = SnapshotProfileKind::Logical
        .required_roots()
        .iter()
        .map(|kind| {
            let value =
                crate::preserves_rail::record("world-distribution-root-fixture", vec![crate::preserves_rail::string(
                    kind.as_str(),
                )]);
            let bytes = crate::preserves_rail::canonical_bytes(&value).expect("root bytes");
            let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
            let root = WorldRootRef::parse(*kind, reference).expect("root ref");
            store.roots.insert(root.as_str().to_string(), bytes);
            root
        })
        .collect::<Vec<_>>();
    let core = WorldCommitCore {
        version: WorldCommitVersion::V1,
        profile: SnapshotProfile {
            kind: SnapshotProfileKind::Logical,
            profile_ref: SnapshotProfileRef::new(reference("logical-profile")).expect("profile ref"),
            cohort_ref: None,
        },
        parents: Vec::new(),
        roots,
        completeness: CompletenessClaim::for_profile(SnapshotProfileKind::Logical),
    };
    let commit = canonical_world_commit(&core, &world_bounds()).expect("canonical commit");
    store.commits.insert(commit.commit_ref.as_str().to_string(), commit.bytes.clone());
    (store, commit)
}

struct ReplicationTransport {
    corrupt: bool,
}

impl TransportPort for ReplicationTransport {
    fn fetch(&mut self, action: &Action) -> Result<TransferOutcome> {
        Ok(TransferOutcome::Received(TransferEnvelope {
            transfer_ref: reference(if self.corrupt { "corrupt-transfer" } else { "transfer" }),
            transport_verification_ref: reference("transport-verification"),
            operation_id: action.operation_id.clone(),
            content_ref: action.content_ref.clone(),
            manifest_ref: reference("transport-manifest"),
            source_peer: action.source_peer.clone().unwrap_or_else(|| reference("source-peer")),
            target_peer: action.target_peer.clone(),
            generation: CURRENT_GENERATION,
            membership_epoch: CURRENT_GENERATION,
            placement_epoch: CURRENT_GENERATION,
            encoded_bytes: action.encoded_bytes,
            protected: action.preserve_protected_form,
        }))
    }
}

struct ReplicationContent {
    corrupt: bool,
}

impl ContentPort for ReplicationContent {
    fn inventory(&mut self, _manifest: &Manifest) -> Result<Inventory> {
        Ok(Inventory::default())
    }

    fn verify(&mut self, action: &Action, envelope: &TransferEnvelope) -> Result<VerificationObservation> {
        Ok(VerificationObservation {
            verification_ref: reference("content-verification"),
            operation_id: action.operation_id.clone(),
            replica: Replica {
                content_ref: action.content_ref.clone(),
                peer_id: action.target_peer.clone(),
                fault_domain: action.fault_domain.clone(),
                generation: CURRENT_GENERATION,
                membership_epoch: CURRENT_GENERATION,
                placement_epoch: CURRENT_GENERATION,
                present: true,
                identity_verified: !self.corrupt,
                pinned: true,
                protected: envelope.protected,
                manifest_ref: envelope.manifest_ref.clone(),
                cleanup_clearance_ref: None,
            },
            identity_verified: !self.corrupt,
            authorization_admitted: true,
        })
    }

    fn cleanup(&mut self, _action: &Action, _admission: &CleanupObservation) -> Result<String> {
        Ok(reference("cleanup"))
    }
}

struct DagAuthority;

impl DagAuthorityPort for DagAuthority {
    fn observe_authority(&mut self, plan: &DagSyncPlan) -> Result<DagAuthorityObservation> {
        Ok(DagAuthorityObservation {
            authority_ref: reference("dag-authority"),
            plan_ref: plan.plan_ref.clone(),
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            admitted: true,
        })
    }
}

struct DagResources;

impl DagResourcePort for DagResources {
    fn reserve(&mut self, plan: &DagSyncPlan) -> Result<DagResourceObservation> {
        Ok(DagResourceObservation {
            reservation_ref: reference("dag-resources"),
            plan_ref: plan.plan_ref.clone(),
            admitted: true,
        })
    }
}

struct Progress {
    loaded: Option<DagSyncProgress>,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagProgressPort for Progress {
    fn load(&mut self, _epoch_ref: &DagEpochRef) -> Result<Option<DagSyncProgress>> {
        Ok(self.loaded.clone())
    }

    fn store(&mut self, progress: &DagSyncProgress) -> Result<String> {
        self.loaded = Some(progress.clone());
        self.events.borrow_mut().push("progress");
        Ok(reference("durable-progress"))
    }
}

struct Observations;

impl DagObservationPort for Observations {
    fn publish_response(&mut self, _response: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        Ok(())
    }

    fn publish_progress(&mut self, _progress: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        Ok(())
    }
}

struct DagReceipts {
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl DagReceiptPort for DagReceipts {
    fn publish_receipt(&mut self, _receipt: &crate::dag_sync::CanonicalDagRecord) -> Result<()> {
        self.events.borrow_mut().push("dag-receipt");
        Ok(())
    }
}

struct WorldReceipts {
    events: Rc<RefCell<Vec<&'static str>>>,
    count: usize,
}

impl WorldDistributionReceiptPort for WorldReceipts {
    fn publish_world_distribution_receipt(&mut self, _receipt: &CanonicalWorldDistributionRecord) -> Result<()> {
        self.events.borrow_mut().push("world-receipt");
        self.count = self.count.saturating_add(1);
        Ok(())
    }
}

struct ClaimTransport {
    carriers: Vec<WorldClaimCarrier>,
}

impl WorldClaimTransportPort for ClaimTransport {
    fn receive_claims(&mut self, _maximum: usize) -> Result<Vec<WorldClaimCarrier>> {
        Ok(self.carriers.clone())
    }
}

struct ClaimAuthentication;

impl WorldClaimAuthenticationPort for ClaimAuthentication {
    fn authenticate_claim(&mut self, carrier: &WorldClaimCarrier) -> Result<WorldHeadAuthenticationObservation> {
        Ok(WorldHeadAuthenticationObservation {
            statement_ref: WorldHeadStatementRef::new(reference("claim-statement")).expect("statement ref"),
            decision_ref: WorldHeadAuthenticationDecisionRef::new(reference("claim-authentication"))
                .expect("authentication ref"),
            passed: true,
            purpose_matches: true,
            policy_matches: true,
            signers: vec![WorldHeadSignerObservation {
                key_identity_ref: reference("claim-key"),
                role: WorldHeadSignerRole::Maintainer,
                authenticated: true,
                current: true,
                revoked: false,
                authority_admitted: carrier.claim.successor_generation == NEXT_GENERATION,
            }],
        })
    }
}

struct ClaimAuthority {
    admitted: bool,
}

impl WorldClaimAuthorityPort for ClaimAuthority {
    fn observe_claim_authority(&mut self, _carrier: &WorldClaimCarrier) -> Result<WorldClaimAuthorityFacts> {
        Ok(WorldClaimAuthorityFacts {
            authority: WorldHeadAuthorityObservation {
                authority_ref: WorldHeadAuthorityRef::new(reference("claim-authority")).expect("authority ref"),
                policy_ref: claim_policy_ref(),
                admitted: self.admitted,
                observed_generation: CURRENT_GENERATION,
            },
            currentness: WorldHeadCurrentnessObservation {
                durable_generation_observed: true,
                independent_ref: None,
            },
            evidence_ref: reference("claim-authority-evidence"),
        })
    }
}

fn claim_carrier() -> WorldClaimCarrier {
    WorldClaimCarrier {
        peer_ref: reference("claim-peer"),
        claim_ref: WorldHeadClaimRef::new(reference("claim-ref")).expect("claim ref"),
        claim: WorldHeadClaim {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            expected_head: Some(commit_ref("claim-root")),
            successor_head: commit_ref("claim-successor"),
            expected_generation: CURRENT_GENERATION,
            successor_generation: NEXT_GENERATION,
            purpose: WorldHeadPurpose::Advance,
            policy_ref: claim_policy_ref(),
            source_heads: Vec::new(),
        },
        encoded_bytes: ROOT_BYTES,
    }
}
