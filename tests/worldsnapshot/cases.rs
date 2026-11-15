#[test]
fn public_logical_restore_survives_distinct_process_restart_without_effect_dispatch() -> FixtureResult<()> {
    // r[verify molten.world_snapshot.runtime_spine_restore]
    let fixture = Fixture::new("public-logical-process-restart")?;
    let receipt = fixture.workspace.join("receipt.preserves");
    succeeded(&fixture.restore(&receipt)?); // child exits; no runtime object survives in this test
    let readback = fixture.workspace.join("readback.preserves");
    let output = command(&[
        "world-snapshot", "readback", "--state-root", text(&fixture.workspace.join("node"))?,
        "--out", text(&readback)?,
    ])?;
    succeeded(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("unresolved_effects=1 dispatched_effects=0"));
    let bytes = std::fs::read(&readback)?;
    assert_eq!(bytes, fixture.image.canonical_bytes()?);
    let recovered = molten::world_snapshot::LogicalRuntimeImage::from_bytes(&bytes)?;
    let mut restarted = molten::runtime::RuntimeState::from_snapshot(recovered.snapshot)?;
    let mut uninterrupted = molten::runtime::RuntimeState::from_snapshot(fixture.image.snapshot.clone())?;
    for step in [
        molten::runtime::RuntimeStep::Clock { actor: "producer".into() },
        molten::runtime::RuntimeStep::Random { actor: "producer".into(), upper: 97 },
    ] {
        assert_eq!(restarted.apply_step(&step), uninterrupted.apply_step(&step));
    }
    assert_eq!(restarted.snapshot(), uninterrupted.snapshot());
    molten::preserves_rail::strict_canonical_decode(&std::fs::read(receipt)?)?;
    Ok(())
}

#[test]
fn public_logical_restore_denies_corrupt_missing_and_revoked_state_before_activation() -> FixtureResult<()> {
    // r[verify molten.world_snapshot.runtime_spine_activation]
    for failure in ["corrupt", "missing", "revoked", "unknown-profile", "cohort"] {
        let fixture = Fixture::new("public-logical-process-denial")?;
        match failure {
            "corrupt" => fixture.root.write(
                &molten::node_state::RelativePath::parse("storage/world-snapshot/bundle/entropy/state.preserves")?,
                b"corrupt",
            )?,
            "missing" => fixture.root.remove_regular_file(
                &molten::node_state::RelativePath::parse("storage/world-snapshot/bundle/time/state.preserves")?,
            )?,
            "revoked" => {
                let mut current = fixture.current.clone();
                current.allowed = false;
                fixture.admit(&current)?;
            }
            _ => {
                let mut descriptor = fixture.descriptor.clone();
                if failure == "unknown-profile" {
                    descriptor.profile_ref = molten_core::world_commit::SnapshotProfileRef::new(reference("unknown profile"))
                        .map_err(|error| format!("profile: {error:?}"))?;
                } else {
                    descriptor.cohort.facts[0].identity = reference("caller-provided fake runtime build");
                }
                std::fs::write(
                    fixture.workspace.join("descriptor.preserves"),
                    molten::world_snapshot::canonical_snapshot_descriptor(&descriptor)?.bytes,
                )?;
            }
        }
        let receipt = fixture.workspace.join("denied-receipt.preserves");
        let output = fixture.restore(&receipt)?;
        assert!(!output.status.success(), "failure={failure}");
        assert!(!receipt.exists());
        assert!(!fixture.root.try_exists(&molten::node_state::RelativePath::parse("ledger/world-snapshot/runtime.redb")?)?);
    }
    Ok(())
}

#[test]
fn public_logical_restore_receipt_output_failure_does_not_claim_no_activation() -> FixtureResult<()> {
    let fixture = Fixture::new("public-logical-post-activation-output")?;
    let output = fixture.restore(&fixture.workspace.join("missing-parent/receipt.preserves"))?;
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("activation=completed canonical-receipt=stored receipt-output=failed"), "{error}");
    let output = command(&[
        "world-snapshot", "readback", "--state-root", text(&fixture.workspace.join("node"))?,
        "--out", text(&fixture.workspace.join("still-active.preserves"))?,
    ])?;
    succeeded(&output);
    Ok(())
}

#[test]
fn public_opaque_machine_restore_remains_denied_without_an_admitted_adapter() -> FixtureResult<()> {
    let fixture = Fixture::new("public-opaque-process-denial")?;
    let mut descriptor = fixture.descriptor.clone();
    descriptor.class = molten_core::world_snapshot::SnapshotClass::Opaque;
    descriptor.profile_ref = molten_core::world_commit::SnapshotProfileRef::new(reference("opaque profile"))
        .map_err(|error| format!("profile: {error:?}"))?;
    descriptor.cohort = molten_core::world_snapshot::SnapshotCohort {
        cohort_ref: molten_core::world_commit::SnapshotCohortRef::new(reference("opaque cohort"))
            .map_err(|error| format!("cohort: {error:?}"))?,
        facts: molten_core::world_snapshot::OPAQUE_COHORT_FACTS.iter().map(|kind| molten_core::world_snapshot::CohortFact {
            kind: *kind, identity: reference(kind.as_str()),
        }).collect(),
    };
    descriptor.components = molten_core::world_snapshot::OPAQUE_COMPONENTS.iter()
        .map(|kind| opaque_component(*kind)).collect::<FixtureResult<Vec<_>>>()?;
    std::fs::write(
        fixture.workspace.join("descriptor.preserves"),
        molten::world_snapshot::canonical_snapshot_descriptor(&descriptor)?.bytes,
    )?;
    let output = fixture.restore(&fixture.workspace.join("opaque-receipt.preserves"))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("opaque/VM restore denied"));
    assert!(!fixture.root.try_exists(&molten::node_state::RelativePath::parse("ledger/world-snapshot/runtime.redb")?)?);
    Ok(())
}

fn opaque_component(kind: molten_core::world_snapshot::SnapshotComponentKind) -> FixtureResult<molten_core::world_snapshot::SnapshotComponent> {
    let identity = reference(kind.as_str());
    Ok(molten_core::world_snapshot::SnapshotComponent {
        kind,
        identity: identity.clone(),
        root: kind.root_kind().map(|root| molten_core::world_commit::WorldRootRef::parse(root, identity))
            .transpose().map_err(|error| format!("root: {error:?}"))?,
        owner: if matches!(kind,
            molten_core::world_snapshot::SnapshotComponentKind::Artifact
                | molten_core::world_snapshot::SnapshotComponentKind::Schema
                | molten_core::world_snapshot::SnapshotComponentKind::RuntimeProfile
                | molten_core::world_snapshot::SnapshotComponentKind::Policy
        ) {
            molten_core::world_snapshot::ComponentOwner::Molten
        } else {
            molten_core::world_snapshot::ComponentOwner::ChaosControl
        },
    })
}
