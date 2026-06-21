#[test]
fn cli_chain_publish_fetch_commands_work() {
    let dir = temp_dir("chain-cli");
    let fixture = chain_publish_fixture(&dir);
    let bundle_ref = publish_chain_segment(&dir, &fixture);
    fetch_chain_segment(&dir, fixture, bundle_ref);
}

struct ChainPublishFixture {
    ledger: PathBuf,
    destination: PathBuf,
    iroh_store: PathBuf,
    chain: molten::evidence_chain::ChainScope,
    head_ref: String,
}

fn chain_publish_fixture(dir: &Path) -> ChainPublishFixture {
    let ledger = dir.join("ledger-source");
    let destination = dir.join("ledger-destination");
    let iroh_store = dir.join("chain-iroh");
    let chain = molten::evidence_chain::ChainScope::new("cli-chain", "artifact", "epoch");
    let payload_ref = import_chain_payload(&ledger);
    let link_ref = append_genesis_chain_link(&ledger, &chain, payload_ref);
    ChainPublishFixture {
        ledger,
        destination,
        iroh_store,
        chain,
        head_ref: link_ref,
    }
}

fn import_chain_payload(ledger: &Path) -> String {
    let payload_value = molten::preserves_rail::record(
        "cli-chain-payload",
        vec![molten::preserves_rail::string("ok")],
    );
    ledger::import_artifact(ledger, &payload_value)
        .expect("import chain payload")
        .artifact_ref
}

fn append_genesis_chain_link(ledger: &Path, chain: &molten::evidence_chain::ChainScope, payload_ref: String) -> String {
    let input = molten::evidence_chain::ChainLinkInput::genesis(
        chain.clone(),
        molten::evidence_chain::ChainPayload::new("cli-chain-payload", payload_ref, "molten.test.payload.v1"),
        Vec::new(),
        molten::evidence_chain::ChainProducer::new("node:cli", test_ref("producer-key")),
        test_ref("genesis-input"),
    );
    let link_value = molten::evidence_chain::chain_link_value(&input);
    let link = molten::evidence_chain::parse_chain_link(&link_value).expect("parse chain link");
    molten::evidence_chain::append_chain_link(ledger, &link_value).expect("append chain link");
    link.link_ref
}

fn publish_chain_segment(dir: &Path, fixture: &ChainPublishFixture) -> String {
    run_chain_command(ChainCommand::Publish {
        ledger: fixture.ledger.clone(),
        iroh_store: fixture.iroh_store.clone(),
        scope: fixture.chain.scope.clone(),
        id: fixture.chain.id.clone(),
        epoch: fixture.chain.epoch.clone(),
        anchor: None,
        head: Some(fixture.head_ref.clone()),
        node: "node:cli".to_string(),
        fork_policy: "reject-unexpected-forks".to_string(),
        receipt_out: Some(dir.join("chain-publish.preserves")),
    })
    .expect("publish chain segment");
    only_blob_ref(&fixture.iroh_store)
}

fn fetch_chain_segment(dir: &Path, fixture: ChainPublishFixture, bundle_ref: String) {
    run_chain_command(ChainCommand::Fetch {
        ticket: format!("iroh-local-chain:{bundle_ref}"),
        ledger: fixture.destination.clone(),
        iroh_store: fixture.iroh_store,
        expected_bundle_ref: Some(bundle_ref),
        peer: "peer:cli".to_string(),
        fork_policy: "reject-unexpected-forks".to_string(),
        receipt_out: Some(dir.join("chain-fetch.preserves")),
    })
    .expect("fetch chain segment");
    let entries = ledger::list_artifacts(&fixture.destination).expect("list destination ledger");
    assert!(entries.iter().any(|entry| entry.artifact_kind == "chain-link"));
    assert!(entries
        .iter()
        .any(|entry| entry.artifact_kind == "iroh-chain-exchange-receipt"));
}
