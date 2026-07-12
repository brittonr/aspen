
    fn import_raw_link(root: &Path, input: &ChainLinkInput) -> ChainLink {
        let value = chain_link_value(input);
        let link = parse_chain_link(&value).expect("parse raw link");
        crate::ledger::import_artifact(root, &value).expect("raw import link");
        link
    }

    fn has_diagnostic(verify: &ChainVerify, kind: &str) -> bool {
        verify.diagnostics.iter().any(|diagnostic| diagnostic.kind == kind)
    }

    fn checkpoint_range_predicate(root: &Path, verify: &ChainVerify) -> String {
        verify
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = crate::ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                parse_chain_predicate_receipt(&value).expect("parse predicate receipt").predicate
                    == CHECKPOINT_COVERS_RANGE_PREDICATE
            })
            .cloned()
            .expect("checkpoint range predicate ref")
    }

    fn predicate_names(root: &Path, predicate_refs: &[String]) -> Vec<String> {
        predicate_refs
            .iter()
            .map(|predicate_ref| {
                let value = crate::ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                parse_chain_predicate_receipt(&value).expect("parse predicate receipt").predicate
            })
            .collect()
    }

    fn checkpoint_checks() -> Vec<ChainCheck> {
        vec![
            ChainCheck::pass("raft-control-plane-command"),
            ChainCheck::pass("verified-range"),
            ChainCheck::pass("checkpoint-freshness"),
        ]
    }

    fn sample_producer() -> ChainProducer {
        ChainProducer::new("node:local", ref_for("producer-key"))
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> crate::test_support::ProcessWorkspace {
        crate::test_support::process_workspace(name).expect("create isolated evidence workspace")
    }
