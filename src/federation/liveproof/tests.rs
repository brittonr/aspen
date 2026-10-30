// r[verify molten.dag_sync.fabric_admission]
// r[verify molten.federated_pull_runtime.receiver_pull]
// r[verify molten.federated_pull_runtime.crypto]
// r[verify molten.federated_pull_runtime.final_validation]
#[test]
#[ignore = "requires owner-approved DAG conformance evidence: MOLTEN_DAG_CONFORMANCE_REF"]
fn live_loopback_commits_real_receiver_registry_and_detached_evidence_denies_tampering() {
    let workspace = crate::test_support::ProcessWorkspace::new("federation-live-import").unwrap();
    super::loopback(&input(&workspace)).unwrap();
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace.join("receiver")).unwrap();
    let identity = root.identity().unwrap();
    let adapter = super::setup::profiles::crypto(&identity).unwrap();
    let exchange = crate::node_state::DirectoryRoot::open_existing(&workspace).unwrap().receipts().unwrap();
    let provider = super::setup::exchange::handoff(&exchange, "provider").unwrap();
    let receiver = super::setup::exchange::handoff(&exchange, "receiver").unwrap();
    let context =
        super::setup::exchange::context(&provider, &receiver.public_key, &receiver.signer_ref, receiver.generation);
    let receipts = root.receipts().unwrap();
    let bytes = receipts
        .read(
            &crate::node_state::RelativePath::parse("detached-bundle.preserves").unwrap(),
            super::receipt::MAXIMUM_BYTES,
        )
        .unwrap();
    let accepted = super::receipt::verify(&adapter, &context, &bytes).unwrap();
    assert_eq!(
        crate::ledger::read_artifact_with_root(&root.ledger_store().unwrap(), &accepted.artifact_ref).unwrap(),
        super::setup::profiles::artifact()
    );
    let detached = crate::test_support::ProcessWorkspace::new("federation-public-only-offline-verifier").unwrap();
    let detached_root = crate::node_state::DirectoryRoot::open(&detached).unwrap();
    let public = detached_root.receipts().unwrap();
    public
        .write(&crate::node_state::RelativePath::parse("detached-bundle.preserves").unwrap(), &bytes)
        .unwrap();
    let public_context = receipts
        .read(&crate::node_state::RelativePath::parse("public-context.preserves").unwrap(), 8192)
        .unwrap();
    public
        .write(&crate::node_state::RelativePath::parse("public-context.preserves").unwrap(), &public_context)
        .unwrap();
    let result = super::verify_detached(&detached, &accepted.context_ref).unwrap();
    let result = result.collect_simple_record("federation-live-proof-v1", Some(7)).unwrap();
    assert_eq!(result[3].as_string().as_deref(), Some(accepted.artifact_ref.as_str()));
    assert_registry_absent(&detached);
    assert_tampered_bundles_rejected(&adapter, &context, &bytes);
    let mut wrong_context = context;
    wrong_context.policy_ref = super::setup::transport::reference("unadmitted-offline-policy");
    assert!(super::receipt::verify(&adapter, &wrong_context, &bytes).is_err());
    assert!(
        super::verify_detached(&workspace.join("receiver"), &super::setup::transport::reference("untrusted-context"))
            .is_err()
    );
}
fn assert_tampered_bundles_rejected(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
    context: &super::receipt::PublicContext,
    bytes: &[u8],
) {
    let original = crate::preserves_rail::parse_canonical_bytes(bytes).unwrap();
    let outer = original.collect_simple_record("federation-detached-bundle-v1", Some(2)).unwrap();
    let body = crate::preserves_rail::value_to_iovalue(&outer[0]);
    let seal = crate::preserves_rail::value_to_iovalue(&outer[1]);
    for (index, label) in [
        (9, "altered-content"),
        (8, "altered-import-receipt"),
        (6, "altered-content-receipt"),
        (10, "altered-supervised-admission"),
    ] {
        let altered =
            replace(&body, "federation-detached-evidence-v1", index, crate::preserves_rail::record(label, Vec::new()));
        let changed = crate::preserves_rail::record("federation-detached-bundle-v1", vec![altered, seal.clone()]);
        assert!(
            super::receipt::verify(adapter, context, &crate::preserves_rail::canonical_bytes(&changed).unwrap())
                .is_err()
        );
    }
    let signed = seal.collect_simple_record("federation-wire-v1", Some(6)).unwrap();
    let signature = signed[5].collect_sequence().unwrap();
    let mut signature = signature.iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    signature[0] = crate::preserves_rail::u64_value(signature[0].as_u64().unwrap().unwrap() ^ 1);
    let bad_seal = replace(&seal, "federation-wire-v1", 5, crate::preserves_rail::sequence(signature));
    let bad = crate::preserves_rail::record("federation-detached-bundle-v1", vec![body, bad_seal]);
    assert!(super::receipt::verify(adapter, context, &crate::preserves_rail::canonical_bytes(&bad).unwrap()).is_err());
}

// r[verify molten.federated_pull_runtime.receiver_pull]
// r[verify molten.federated_pull_runtime.crypto]
#[test]
#[ignore = "requires owner-approved DAG conformance evidence: MOLTEN_DAG_CONFORMANCE_REF"]
fn actual_malicious_responses_cannot_materialize_receiver_registry_artifacts() {
    for fault in [
        "corrupt-signature",
        "wrong-key",
        "wrong-domain",
        "corrupt-content",
        "unsolicited-push",
    ] {
        let workspace = crate::test_support::ProcessWorkspace::new(&format!("federation-live-{fault}")).unwrap();
        super::loopback_denial(&input(&workspace), fault).unwrap();
        assert_registry_absent(&workspace.join("receiver"));
    }
}

// r[verify molten.federated_pull_runtime.receiver_pull]
#[test]
#[ignore = "requires owner-approved DAG conformance evidence and MOLTEN_FIXTURE_BINARY"]
fn distinct_os_processes_reject_actual_malicious_signed_responses_before_registry_mutation() {
    let binary =
        std::env::var_os("MOLTEN_FIXTURE_BINARY").expect("set MOLTEN_FIXTURE_BINARY to the actual molten CLI binary");
    for fault in [
        "corrupt-signature",
        "wrong-key",
        "wrong-domain",
        "corrupt-content",
        "unsolicited-push",
    ] {
        let workspace = crate::test_support::ProcessWorkspace::new(&format!("federation-process-{fault}")).unwrap();
        super::distinct_process_denial(&input(&workspace), std::path::Path::new(&binary), fault).unwrap();
        assert_registry_absent(&workspace.join("receiver"));
    }
}

fn assert_registry_absent(receiver: &std::path::Path) {
    let root = crate::node_state::DirectoryRoot::open_existing(receiver).unwrap();
    let registry = root.ledger_store().unwrap();
    let head = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact()).unwrap();
    let path = crate::local_store::Locator::parse(&format!(
        "content/blake3_{}.bin",
        crate::preserves_rail::content_ref_hex(&head).unwrap()
    ))
    .unwrap();
    assert!(!registry.root().try_exists(&path).unwrap());
    assert!(crate::ledger::read_artifact_with_root(&registry, &head).is_err());
}

// r[verify molten.federated_pull_runtime.receiver_pull]
#[test]
#[ignore = "requires owner-approved DAG conformance evidence and MOLTEN_FIXTURE_BINARY"]
fn distinct_os_processes_commit_signed_admitted_receiver_artifact() {
    let workspace = crate::test_support::ProcessWorkspace::new("federation-distinct-process-import").unwrap();
    let binary =
        std::env::var_os("MOLTEN_FIXTURE_BINARY").expect("set MOLTEN_FIXTURE_BINARY to the actual molten CLI binary");
    super::distinct_process(&input(&workspace), std::path::Path::new(&binary)).unwrap();
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace.join("receiver")).unwrap();
    let head = crate::preserves_rail::canonical_hash(&super::setup::profiles::artifact()).unwrap();
    assert_eq!(
        crate::ledger::read_artifact_with_root(&root.ledger_store().unwrap(), &head).unwrap(),
        super::setup::profiles::artifact()
    );
}
fn replace(
    value: &preserves::IOValue,
    label: &'static str,
    index: usize,
    replacement: preserves::IOValue,
) -> preserves::IOValue {
    let fields = value.collect_simple_record(label, None).unwrap();
    let mut fields = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    fields[index] = replacement;
    crate::preserves_rail::record(label, fields)
}
fn input(root: &std::path::Path) -> super::RunInput {
    super::RunInput {
        run_directory: root.to_path_buf(),
        dag_conformance_ref: std::env::var("MOLTEN_DAG_CONFORMANCE_REF")
            .expect("set MOLTEN_DAG_CONFORMANCE_REF to independently reviewed current owner DAG conformance evidence"),
    }
}
