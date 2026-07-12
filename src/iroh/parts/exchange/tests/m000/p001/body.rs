
    fn import_test_link(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> crate::evidence_chain::ChainLink {
        let value = test_link_value(root, chain, previous, payload_label);
        let link = crate::evidence_chain::parse_chain_link(&value).expect("parse link");
        crate::ledger::import_artifact(root, &value).expect("import raw link");
        link
    }

    fn test_link_value(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> IoValue {
        let payload = stored_payload(root, payload_label);
        match previous {
            Some(previous) => crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::append(
                previous,
                payload,
                Vec::new(),
                sample_producer(),
                ref_for(&format!("append-input-{payload_label}")),
            )),
            None => crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::genesis(
                chain.clone(),
                payload,
                Vec::new(),
                sample_producer(),
                ref_for(&format!("genesis-input-{payload_label}")),
            )),
        }
    }

    fn stored_payload(root: &Path, label: &str) -> crate::evidence_chain::ChainPayload {
        let artifact = record("test-payload", vec![string(label)]);
        let imported = crate::ledger::import_artifact(root, &artifact).expect("import payload");
        crate::evidence_chain::ChainPayload::new("test-payload", imported.artifact_ref, "molten.test.payload.v1")
    }

    fn checkpoint_range_predicate(root: &Path, verify: &crate::evidence_chain::ChainVerify) -> String {
        verify
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = crate::ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                crate::evidence_chain::parse_chain_predicate_receipt(&value)
                    .expect("parse predicate receipt")
                    .predicate
                    == crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE
            })
            .cloned()
            .expect("checkpoint range predicate ref")
    }

    fn checkpoint_checks() -> Vec<crate::evidence_chain::ChainCheck> {
        vec![
            crate::evidence_chain::ChainCheck::pass("raft-control-plane-command"),
            crate::evidence_chain::ChainCheck::pass("verified-range"),
            crate::evidence_chain::ChainCheck::pass("checkpoint-freshness"),
        ]
    }

    fn sample_producer() -> crate::evidence_chain::ChainProducer {
        crate::evidence_chain::ChainProducer::new("node:local", ref_for("producer-key"))
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> crate::test_support::ProcessWorkspace {
        crate::test_support::process_workspace(name).expect("create isolated exchange workspace")
    }
