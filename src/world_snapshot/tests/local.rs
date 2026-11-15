include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/world_snapshot/tests/local/fixture.rs"));

#[test]
fn local_logical_runtime_restores_owner_state_and_continues_deterministically() {
    // r[verify molten.world_snapshot.runtime_spine_restore]
    let fixture = Fixture::new("logical-runtime-state");
    let restored = fixture.restore().expect("restore");
    assert!(!restored.restore.observations.is_empty());
    let mut readback = read_local_logical_runtime(&fixture.root).expect("read back recreated runtime");
    assert_eq!(readback.canonical_image, fixture.image.canonical_bytes().expect("canonical image"));
    assert_eq!(readback.runtime.snapshot(), fixture.image.snapshot);
    assert_eq!(restored.runtime.snapshot(), fixture.image.snapshot);
    assert_eq!(readback.unresolved_effects, fixture.image.unresolved_effects);
    let mut uninterrupted = RuntimeState::from_snapshot(fixture.image.snapshot.clone()).expect("uninterrupted");
    for step in [
        RuntimeStep::Clock {
            actor: "producer".into(),
        },
        RuntimeStep::Random {
            actor: "producer".into(),
            upper: 100,
        },
    ] {
        assert_eq!(readback.runtime.apply_step(&step), uninterrupted.apply_step(&step));
    }
    assert_eq!(readback.runtime.snapshot(), uninterrupted.snapshot());
    assert_eq!(
        fixture.restore().expect("same activation is idempotent").restore.receipt.artifact_ref,
        restored.restore.receipt.artifact_ref
    );
}

#[test]
fn local_logical_runtime_denies_untrusted_revoked_expired_and_corrupt_inputs() {
    // r[verify molten.world_snapshot.runtime_spine_activation]
    let fixture = Fixture::new("logical-runtime-denials");
    let mut changed = fixture.current.clone();
    changed.allowed = false;
    fixture.admit(&changed, &fixture.signer);
    assert!(fixture.restore().expect_err("revoked").to_string().contains("activation=inactive"));
    changed = fixture.current.clone();
    changed.expires_unix_seconds = 0;
    fixture.admit(&changed, &fixture.signer);
    assert!(fixture.restore().is_err());
    fixture.admit(&fixture.current, &SigningKey::from_bytes(&[9; 32]));
    assert!(fixture.restore().is_err());
    fixture.admit(&fixture.current, &fixture.signer);
    fixture.write("storage/world-snapshot/bundle/entropy/state.preserves", b"corrupt");
    assert!(fixture.restore().expect_err("corrupt").to_string().contains("activation=inactive"));
    assert!(
        !fixture
            .root
            .try_exists(&RelativePath::parse("ledger/world-snapshot/runtime.redb").expect("path"))
            .expect("exists")
    );
}

#[test]
fn local_logical_runtime_does_not_accept_supplied_destination_or_unknown_profile() {
    let fixture = Fixture::new("logical-runtime-cohort");
    let mut supplied = fixture.descriptor.cohort.clone();
    supplied.facts[0].identity = reference("caller-made-runtime-build");
    assert!(restore_local_logical_snapshot(fixture.adapter(), &fixture.descriptor, &supplied).is_err());
    let mut unsupported = fixture.descriptor.clone();
    unsupported.profile_ref =
        molten_core::world_commit::SnapshotProfileRef::new(reference("unsupported-profile")).expect("profile");
    assert!(restore_local_logical_snapshot(fixture.adapter(), &unsupported, &fixture.descriptor.cohort).is_err());
}

struct ChangingAdmission<'a> {
    fixture: &'a Fixture,
    inner: LocalLogicalSnapshot,
    calls: usize,
    revoke: bool,
}
impl CurrentSnapshotAdmissionPort for ChangingAdmission<'_> {
    fn observe_current(
        &mut self,
        descriptor: &SnapshotDescriptor,
        descriptor_ref: &str,
        destination: &SnapshotCohort,
    ) -> crate::error::Result<SnapshotAdmissionObservation> {
        self.calls += 1;
        if self.calls == 2 {
            let mut current = self.fixture.current.clone();
            current.generation += 1;
            if self.revoke {
                current.allowed = false;
                current.revocation_ref = reference("revoked");
            }
            self.fixture.admit(&current, &self.fixture.signer);
        }
        self.inner.observe_current(descriptor, descriptor_ref, destination)
    }
}

#[test]
fn local_logical_runtime_fences_generation_and_revocation_during_reconstruction() {
    // r[verify molten.world_snapshot.runtime_spine_activation]
    for revoke in [false, true] {
        let fixture = Fixture::new("logical-runtime-currentness-fence");
        let adapter = fixture.adapter();
        {
            let mut state = adapter.state.borrow_mut();
            state.descriptor = Some(fixture.descriptor.clone());
            state.descriptor_ref = canonical_snapshot_descriptor(&fixture.descriptor).expect("descriptor").artifact_ref;
        }
        let mut materialization = adapter.clone();
        let mut admission = ChangingAdmission {
            fixture: &fixture,
            inner: adapter.clone(),
            calls: 0,
            revoke,
        };
        let mut handles = adapter.clone();
        let mut runtime = adapter.clone();
        let mut receipts = adapter.clone();
        restore_logical_snapshot(&fixture.descriptor, &fixture.descriptor.cohort, LogicalSnapshotPorts {
            materialization: &mut materialization,
            admission: &mut admission,
            handles: &mut handles,
            runtime: &mut runtime,
            receipts: &mut receipts,
        })
        .expect_err("generation drift must fence activation");
        let state = adapter.state.borrow();
        assert!(!state.activated);
        assert!(!state.activation_attempted);
        assert_eq!(state.restored.len(), 7);
        assert!(state.store.is_some(), "partial state remains for reconciliation");
    }
}

#[test]
fn local_logical_runtime_retains_completed_activation_when_receipt_storage_fails() {
    // r[verify molten.world_snapshot.runtime_spine_activation]
    let fixture = Fixture::new("logical-runtime-receipt-failure");
    fixture.write("storage/world-snapshot/receipts", b"non-directory blocks receipt publication");
    let error = fixture.restore().expect_err("receipt failure").to_string();
    assert!(error.contains("activation=completed receipt-publication=failed"), "{error}");
    let recovered = read_local_logical_runtime(&fixture.root).expect("activation remains observable");
    assert_eq!(recovered.runtime.snapshot(), fixture.image.snapshot);
}

#[test]
fn runtime_snapshot_owner_rejects_zero_rng_crossed_scopes_and_malformed_value_refs() {
    let mut invalid = RuntimeState::new(7).snapshot();
    invalid.rng_state = 0;
    assert!(RuntimeState::from_snapshot(invalid).is_err());
    let fixture = Fixture::new("logical-runtime-owner-invariants");
    let mut invalid = fixture.image.snapshot.clone();
    invalid.facets.get_mut("facet:root").expect("facet").actor = "other-owner".into();
    assert!(RuntimeState::from_snapshot(invalid).is_err());
    let bytes = crate::preserves_rail::canonical_bytes(&fixture.image.snapshot.to_value()).expect("snapshot bytes");
    assert_eq!(
        crate::runtime::RuntimeSnapshot::from_canonical_bytes(&bytes).expect("decode"),
        fixture.image.snapshot
    );
    let mut crossed = bytes;
    let body_ref = fixture.image.snapshot.messages.iter().next().expect("message").body.value_ref().as_bytes();
    let position = crossed.windows(body_ref.len()).position(|window| window == body_ref).expect("body reference");
    crossed[position + 7] = if crossed[position + 7] == b'0' { b'1' } else { b'0' };
    crate::preserves_rail::strict_canonical_decode(&crossed).expect("still canonical wire bytes");
    assert!(crate::runtime::RuntimeSnapshot::from_canonical_bytes(&crossed).is_err());
    let mut cycle = fixture.image.snapshot.clone();
    cycle.facets.get_mut("facet:root").expect("facet").parent = Some("facet:root".into());
    assert!(RuntimeState::from_snapshot(cycle).is_err());
}

#[test]
fn local_logical_runtime_denies_hash_verified_but_semantically_crossed_components() {
    let mut fixture = Fixture::new("logical-runtime-semantic-inventory");
    let bytes =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("runtime-spine-component-v1", vec![
            crate::preserves_rail::string("time"),
            crate::preserves_rail::u64_value(999),
        ]))
        .expect("canonical crossed component");
    fixture.write("storage/world-snapshot/bundle/time/state.preserves", &bytes);
    let component = fixture
        .descriptor
        .components
        .iter_mut()
        .find(|component| component.kind == SnapshotComponentKind::Time)
        .expect("time component");
    component.identity = crate::preserves_rail::content_ref_from_bytes(&bytes);
    component.root = Some(
        molten_core::world_commit::WorldRootRef::parse(
            component.kind.root_kind().expect("root kind"),
            component.identity.clone(),
        )
        .expect("root"),
    );
    fixture.admit(&fixture.current, &fixture.signer);
    let error = fixture.restore().expect_err("semantic inventory must agree before handles").to_string();
    assert!(error.contains("component semantics"), "{error}");
    assert!(
        !fixture
            .root
            .try_exists(&RelativePath::parse("ledger/world-snapshot/runtime.redb").expect("path"))
            .expect("no handles")
    );
}

#[test]
fn local_logical_runtime_refuses_replacing_a_different_active_world() {
    let mut fixture = Fixture::new("logical-runtime-active-world-refusal");
    fixture.restore().expect("first active world");
    let original = read_local_logical_runtime(&fixture.root).expect("active runtime").descriptor_ref;
    fixture.descriptor.commit_ref = WorldCommitRef::new(reference("different world commit")).expect("commit");
    fixture.admit(&fixture.current, &fixture.signer);
    let error = fixture.restore().expect_err("replacement lacks separate authority").to_string();
    assert!(error.contains("activation=inactive"), "{error}");
    assert!(error.contains("different active world"), "{error}");
    assert_eq!(read_local_logical_runtime(&fixture.root).expect("original remains active").descriptor_ref, original);
}

#[test]
fn local_logical_runtime_rejects_retained_admission_generation_rollback() {
    // r[verify molten.world_snapshot.runtime_spine_activation]
    let fixture = Fixture::new("logical-runtime-generation-rollback");
    fixture.restore().expect("initial activation");
    let mut advanced = fixture.current.clone();
    advanced.generation += 1;
    fixture.admit(&advanced, &fixture.signer);
    fixture.restore().expect("higher admission generation");
    let active = read_local_logical_runtime(&fixture.root).expect("advanced activation").activation_ref;

    fixture.admit(&fixture.current, &fixture.signer);
    let error = fixture.restore().expect_err("old signed admission must not replace newer activation").to_string();
    assert!(error.contains("activation=inactive"), "{error}");
    assert!(error.contains("generation"), "{error}");
    assert_eq!(read_local_logical_runtime(&fixture.root).expect("retained activation").activation_ref, active);

    let mut crossed = advanced.clone();
    crossed.capability_ref = reference("different same-generation capability");
    fixture.admit(&crossed, &fixture.signer);
    assert!(fixture.restore().is_err());
    assert_eq!(read_local_logical_runtime(&fixture.root).expect("unchanged activation").activation_ref, active);

    fixture.admit(&advanced, &fixture.signer);
    fixture.restore().expect("exact retained admission is idempotent");
    assert_eq!(read_local_logical_runtime(&fixture.root).expect("idempotent activation").activation_ref, active);
}

#[test]
fn runtime_snapshot_owner_denies_exhausted_resume_counters() {
    for exhausted_time in [true, false] {
        let mut snapshot = RuntimeState::new(7).snapshot();
        if exhausted_time {
            snapshot.logical_time = u64::MAX;
        } else {
            snapshot.effect_sequence = u64::MAX;
        }
        assert!(RuntimeState::from_snapshot(snapshot).is_err());
    }
}
