fn envelope() -> molten::error::Result<molten::content_replication::TransferEnvelope> {
    let manifest = super::manifest();
    let action = super::action(&manifest, false);
    Ok(molten::content_replication::TransferEnvelope {
        transfer_ref: super::digest('a'),
        transport_verification_ref: super::digest('b'),
        operation_id: action.operation_id,
        content_ref: action.content_ref,
        manifest_ref: manifest.contents[0].manifest_ref.clone(),
        source_peer: action
            .source_peer
            .ok_or_else(|| molten::error::MoltenError::invalid_harness("planned transfer has no source peer"))?,
        target_peer: action.target_peer,
        generation: manifest.generation,
        membership_epoch: manifest.membership_epoch,
        placement_epoch: manifest.placement_epoch,
        encoded_bytes: action.encoded_bytes,
        protected: action.preserve_protected_form,
    })
}

fn assert_denial(outcome: molten::content_replication::TransferOutcome, diagnostic: &str) {
    assert_eq!(super::received(outcome), Err(molten::error::MoltenError::InvalidHarness(diagnostic.to_string())),);
}

#[test]
fn received_preserves_the_complete_envelope() -> molten::error::Result<()> {
    let expected = envelope()?;
    let outcome = molten::content_replication::TransferOutcome::Received(expected.clone());
    assert_eq!(super::received(outcome), Ok(expected));
    Ok(())
}

#[test]
fn cancelled_returns_its_variant_and_detail() {
    assert_denial(
        molten::content_replication::TransferOutcome::Cancelled("receiver stopped".to_string()),
        "unexpected transfer outcome: Cancelled(\"receiver stopped\")",
    );
}

#[test]
fn uncertain_preserves_escaped_detail() {
    assert_denial(
        molten::content_replication::TransferOutcome::Uncertain("receipt \"missing\"\ntry later".to_string()),
        "unexpected transfer outcome: Uncertain(\"receipt \\\"missing\\\"\\ntry later\")",
    );
}

#[test]
fn unavailable_preserves_empty_detail() {
    assert_denial(
        molten::content_replication::TransferOutcome::Unavailable(String::new()),
        "unexpected transfer outcome: Unavailable(\"\")",
    );
}

#[test]
fn timed_out_preserves_unicode_detail() {
    assert_denial(
        molten::content_replication::TransferOutcome::TimedOut("récepteur unavailable".to_string()),
        "unexpected transfer outcome: TimedOut(\"récepteur unavailable\")",
    );
}

#[test]
fn setup_error_returns_without_panicking() {
    let manifest = super::manifest();
    let action = super::action(&manifest, false);
    assert_eq!(
        super::run_action("", &manifest, &action),
        Err(molten::error::MoltenError::Io("test workspace logical label cannot be empty".to_string())),
    );
}

#[test]
fn open_error_returns_without_panicking() {
    let mut manifest = super::manifest();
    let action = super::action(&manifest, false);
    manifest.contents[0].content_ref = super::digest('c');
    let expected_manifest = manifest.clone();
    let expected_action = action.clone();
    assert_eq!(
        super::run_action("content_open_rejection", &manifest, &action),
        Err(molten::error::MoltenError::InvalidHarness(
            "multiprocess replication payload does not match its content identity".to_string(),
        )),
    );
    assert_eq!(manifest, expected_manifest);
    assert_eq!(action, expected_action);
}

#[test]
fn fetch_error_returns_without_panicking() {
    let manifest = super::manifest();
    let mut action = super::action(&manifest, false);
    action.operation_id = "not-a-content-reference".to_string();
    let expected_manifest = manifest.clone();
    let expected_action = action.clone();
    assert_eq!(
        super::run_action("content_fetch_rejection", &manifest, &action),
        Err(molten::error::MoltenError::InvalidHarness(
            "multiprocess operation is not a BLAKE3 reference".to_string(),
        )),
    );
    assert_eq!(manifest, expected_manifest);
    assert_eq!(action, expected_action);
}

#[test]
fn offline_verification_error_returns_without_panicking() -> molten::error::Result<()> {
    let workspace = super::test_support::process_workspace("content_offline_rejection")?;
    let run_directory = workspace.join("missing-run");
    let expected_error = std::fs::read_dir(&run_directory)
        .err()
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("missing run directory unexpectedly exists"))?;
    let expected = molten::error::MoltenError::from(expected_error);
    assert!(matches!(&expected, molten::error::MoltenError::Io(_)));
    let envelope = envelope()?;
    let unchanged = envelope.clone();
    assert_eq!(super::verify_envelope(&run_directory, &envelope), Err(expected));
    assert_eq!(envelope, unchanged);
    assert!(!run_directory.exists());
    Ok(())
}

#[test]
fn invalid_operation_has_no_run_directory_or_completed_call() -> molten::error::Result<()> {
    use molten::content_replication::TransportPort;

    let manifest = super::manifest();
    let mut action = super::action(&manifest, false);
    action.operation_id = "not-a-content-reference".to_string();
    let workspace = super::test_support::process_workspace("content_no_process")?;
    let run_root = workspace.join("run");
    let mut adapter = molten::content_replication::DistinctProcessTransferAdapter::open(
        &manifest,
        run_root.clone(),
        std::path::PathBuf::from(env!("CARGO_BIN_EXE_molten")),
        molten::cluster_harness::DEFAULT_DISTINCT_PROCESS_TIMEOUT_MS,
        std::collections::BTreeMap::from([(manifest.contents[0].content_ref.clone(), super::PAYLOAD.to_vec())]),
    )?;
    assert_eq!(
        adapter.fetch(&action),
        Err(molten::error::MoltenError::InvalidHarness(
            "multiprocess operation is not a BLAKE3 reference".to_string(),
        )),
    );
    assert_eq!(adapter.call_count(), 0);
    assert!(!run_root.exists());
    Ok(())
}
