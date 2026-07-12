    use super::*;
    use std::fs;

    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    #[test]
    fn local_iroh_publish_fetch_verifies_bundle_refs() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("iroh");
        let root = CapabilityExchangeRoot::open(&root_path).expect("open exchange capability root");
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "iroh" 1
              <budget-v1 "molten.harness.budget.v1" <limits 8 2 32 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "assert" #f "ready">]>
              [<assert "a" "ready">]>"#,
        )
        .expect("parse suite");
        let run = crate::harness::run_suite_value(&suite).expect("run suite");
        let bundle = crate::harness::sealed_repro_bundle_value_with_command(&run.report_value, &["molten".to_string()])
            .expect("seal bundle");
        let published = publish_bundle_with_root(&root, &bundle, "node:local").expect("publish bundle");
        let fetched = fetch_bundle_with_root(&FetchBundleInput {
            root: &root,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:local",
            out: None,
            ledger_root: None,
        })
        .expect("fetch bundle");
        assert_eq!(published.bundle_ref, fetched.bundle_ref);
        let error = fetch_bundle_with_root(&FetchBundleInput {
            root: &root,
            ticket: &published.ticket,
            expected_bundle_ref: Some("blake3:deadbeef"),
            peer: "peer:local",
            out: None,
            ledger_root: None,
        })
        .expect_err("wrong advertised ref fails");
        assert!(error.to_string().contains("expected blake3:deadbeef"));
    }

    #[test]
    fn local_iroh_chain_segment_publish_fetch_imports_verified_artifacts() {
        let source = temp_dir("chain-source");
        let destination = temp_dir("chain-destination");
        let iroh = temp_dir("chain-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-a", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let second = append_test_link(&source, &chain, Some(&genesis), "payload-b");
        let policy_ref = ref_for("checkpoint-policy");
        crate::evidence_chain::publish_chain_anchor(
            &source,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = crate::evidence_chain::verify_chain_segment(
            &source,
            &chain,
            Some(&genesis.link_ref),
            Some(&second.link_ref),
        )
        .expect("verify range");
        crate::evidence_chain::accept_chain_checkpoint(&source, &crate::evidence_chain::ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&source, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![ref_for("membership")],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");

        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: Some(&genesis.link_ref),
            expected_head: Some(&second.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        let fetched = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("fetch chain segment");
        assert_eq!(published.bundle_ref, fetched.bundle_ref);
        let destination_index = crate::evidence_chain::build_chain_index(&destination).expect("destination index");
        assert_eq!(destination_index.heads_for_chain(&chain), vec![second.link_ref.clone()]);
        assert_eq!(destination_index.anchor_links_for_chain(&chain), vec![genesis.link_ref.clone()]);
        assert_eq!(destination_index.checkpoint_heads_for_chain(&chain), vec![second.link_ref.clone()]);
    }

    #[test]
    fn fetched_chain_segment_rejects_missing_checkpoint_predicate_artifact() {
        let source = temp_dir("chain-missing-predicate-source");
        let destination = temp_dir("chain-missing-predicate-destination");
        let iroh = temp_dir("chain-missing-predicate-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-missing-predicate", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let second = append_test_link(&source, &chain, Some(&genesis), "payload-b");
        let policy_ref = ref_for("checkpoint-policy");
        crate::evidence_chain::publish_chain_anchor(
            &source,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = crate::evidence_chain::verify_chain_segment(
            &source,
            &chain,
            Some(&genesis.link_ref),
            Some(&second.link_ref),
        )
        .expect("verify range");
        crate::evidence_chain::accept_chain_checkpoint(&source, &crate::evidence_chain::ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref,
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&source, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![ref_for("membership")],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&second.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        let bundle_bytes = fs::read(blob_path(&iroh, &published.bundle_ref).expect("blob path")).expect("read bundle");
        let bundle = parse_canonical_bytes(&bundle_bytes).expect("parse bundle");
        let tampered = remove_bundle_artifacts_of_kind(&bundle, "chain-predicate-receipt");
        fs::write(
            blob_path(&iroh, &published.bundle_ref).expect("blob path"),
            canonical_bytes(&tampered).expect("canonical tampered bundle"),
        )
        .expect("write tampered bundle");
        let error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("missing predicate rejected");
        assert!(error.to_string().contains("predicate"));
    }

    #[test]
    fn fetched_chain_segment_rejects_tampered_bundle_bytes() {
        let source = temp_dir("chain-tamper-source");
        let destination = temp_dir("chain-tamper-destination");
        let iroh = temp_dir("chain-tamper-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-tamper", "epoch-1");
        let genesis = append_test_link(&source, &chain, None, "payload-a");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&genesis.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect("publish chain segment");
        fs::write(blob_path(&iroh, &published.bundle_ref).expect("blob path"), b"tampered").expect("tamper blob");
        let error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("tampered bundle rejected");
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn fetched_forked_chain_segment_requires_diagnostic_policy() {
        let source = temp_dir("chain-fork-source");
        let iroh = temp_dir("chain-fork-iroh");
        let chain = crate::evidence_chain::ChainScope::new("test-chain", "artifact-fork", "epoch-1");
        let genesis = import_test_link(&source, &chain, None, "payload-a");
        let first_child = import_test_link(&source, &chain, Some(&genesis), "payload-b");
        let _second_child = import_test_link(&source, &chain, Some(&genesis), "payload-c");
        let published = publish_chain_segment(&PublishChainSegmentInput {
            iroh_root: &iroh,
            ledger_root: &source,
            chain: &chain,
            anchor_ref: None,
            expected_head: Some(&first_child.link_ref),
            node: "node:source",
            fork_policy: crate::evidence_chain::ChainForkPolicy::RetainForkEvidence,
        })
        .expect("publish retained-fork segment");
        let production_destination = temp_dir("chain-fork-prod-destination");
        let production_error = fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &production_destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        })
        .expect_err("production policy rejects fetched forks");
        assert!(production_error.to_string().contains("fork diagnostics"));
        let diagnostic_destination = temp_dir("chain-fork-diagnostic-destination");
        fetch_chain_segment(&FetchChainSegmentInput {
            iroh_root: &iroh,
            ticket: &published.ticket,
            expected_bundle_ref: Some(&published.bundle_ref),
            peer: "peer:source",
            ledger_root: &diagnostic_destination,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RetainForkEvidence,
        })
        .expect("diagnostic policy retains fetched forks");
        let diagnostic_index =
            crate::evidence_chain::build_chain_index(&diagnostic_destination).expect("diagnostic index");
        assert!(diagnostic_index.heads_for_chain(&chain).contains(&first_child.link_ref));
        assert!(!diagnostic_index.fork_evidence_for_chain(&chain).is_empty());
    }

    fn remove_bundle_artifacts_of_kind(bundle: &IoValue, removed_kind: &str) -> IoValue {
        let fields = bundle.collect_simple_record("chain-segment-bundle-v1", Some(8)).expect("chain segment bundle");
        let artifacts_field = value_to_iovalue(&fields[4]);
        let artifacts = artifacts_field.collect_simple_record("artifacts", Some(1)).expect("artifacts field");
        let artifact_values = artifacts[0].collect_sequence().expect("artifact sequence");
        let filtered = artifact_values
            .iter()
            .filter_map(|artifact| {
                let artifact = value_to_iovalue(artifact);
                let artifact_record = artifact.collect_simple_record("artifact", Some(3)).expect("artifact record");
                let kind = required_string(&artifact_record[0], "artifact kind").expect("artifact kind");
                (kind != removed_kind).then_some(artifact)
            })
            .collect::<Vec<_>>();
        record("chain-segment-bundle-v1", vec![
            value_to_iovalue(&fields[0]),
            value_to_iovalue(&fields[1]),
            value_to_iovalue(&fields[2]),
            value_to_iovalue(&fields[3]),
            record("artifacts", vec![sequence(filtered)]),
            value_to_iovalue(&fields[5]),
            value_to_iovalue(&fields[6]),
            value_to_iovalue(&fields[7]),
        ])
    }

    fn append_test_link(
        root: &Path,
        chain: &crate::evidence_chain::ChainScope,
        previous: Option<&crate::evidence_chain::ChainLink>,
        payload_label: &str,
    ) -> crate::evidence_chain::ChainLink {
        let value = test_link_value(root, chain, previous, payload_label);
        let link = crate::evidence_chain::parse_chain_link(&value).expect("parse link");
        crate::evidence_chain::append_chain_link(root, &value).expect("append link");
        link
    }
