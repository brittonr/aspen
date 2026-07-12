    type TestCase = hegel::TestCase;

    use super::*;

    fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
        crate::preserves_rail::canonical_bytes(value)
    }

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    #[test]
    fn chain_link_identity_is_canonical_and_stable() {
        let input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        let first = chain_link_value(&input);
        let second = chain_link_value(&input);
        assert_eq!(canonical_bytes(&first).expect("first bytes"), canonical_bytes(&second).expect("second bytes"));
        assert_eq!(chain_link_ref(&first).expect("first ref"), chain_link_ref(&second).expect("second ref"));

        let parsed = parse_chain_link(&first).expect("parse chain link");
        assert_eq!(parsed.link_ref, chain_link_ref(&first).expect("chain link ref"));
        validate_genesis(&parsed).expect("valid genesis");
    }

    #[test]
    fn chain_link_preserves_payload_ref_without_rewriting_payload() {
        let payload_value = parse_text("<gate-receipt-placeholder \"ok\">").expect("parse payload");
        let payload_ref = canonical_hash(&payload_value).expect("payload ref");
        let input = ChainLinkInput::genesis(
            ChainScope::new("evidence-ledger", "node-a", "epoch-1"),
            ChainPayload::new("gate-receipt", payload_ref.clone(), "molten.harness.gate-receipt.v1"),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("genesis-input"),
        );

        let link_value = chain_link_value(&input);
        let parsed = parse_chain_link(&link_value).expect("parse chain link");
        validate_genesis(&parsed).expect("valid genesis");
        assert_eq!(parsed.payload.artifact_ref, payload_ref);
        assert_ne!(parsed.link_ref, payload_ref);
        assert_eq!(canonical_hash(&payload_value).expect("payload ref after link"), payload_ref);
    }

    #[test]
    fn append_validation_binds_previous_ref_sequence_and_scope() {
        let previous = sample_genesis("evidence-ledger", "node-a", "epoch-1", "payload-a");
        let good_input = ChainLinkInput::append(
            &previous,
            sample_payload("payload-b"),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("append-input"),
        );
        let good = parse_chain_link(&chain_link_value(&good_input)).expect("parse append");
        validate_append(&previous, &good).expect("valid append");

        let mut wrong_previous_input = good_input.clone();
        wrong_previous_input.previous_link_ref = Some(ref_for("wrong-previous"));
        let wrong_previous = parse_chain_link(&chain_link_value(&wrong_previous_input)).expect("parse wrong previous");
        let error = validate_append(&previous, &wrong_previous).expect_err("wrong previous ref rejected");
        assert!(error.to_string().contains("previous link ref mismatch"));

        let mut gap_input = good_input.clone();
        gap_input.sequence += 1;
        let gap = parse_chain_link(&chain_link_value(&gap_input)).expect("parse sequence gap");
        let error = validate_append(&previous, &gap).expect_err("sequence gap rejected");
        assert!(error.to_string().contains("previous + 1"));

        let mut wrong_scope_input = good_input;
        wrong_scope_input.chain.id = "node-b".to_string();
        let wrong_scope = parse_chain_link(&chain_link_value(&wrong_scope_input)).expect("parse wrong scope");
        let error = validate_append(&previous, &wrong_scope).expect_err("wrong scope rejected");
        assert!(error.to_string().contains("same chain scope"));
    }

    #[test]
    fn genesis_validation_rejects_previous_links_and_nonzero_sequences() {
        let mut input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        input.previous_link_ref = Some(ref_for("unexpected-previous"));
        let with_previous = parse_chain_link(&chain_link_value(&input)).expect("parse genesis with previous");
        let error = validate_genesis(&with_previous).expect_err("previous rejected");
        assert!(error.to_string().contains("must not name a previous"));

        let mut input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        input.sequence = 1;
        let nonzero = parse_chain_link(&chain_link_value(&input)).expect("parse nonzero genesis");
        let error = validate_genesis(&nonzero).expect_err("nonzero rejected");
        assert!(error.to_string().contains("sequence must be 0"));
    }

    #[test]
    fn chain_hashing_is_scoped_not_global_ordering() {
        let left = sample_genesis("evidence-ledger", "node-a", "epoch-1", "payload-left");
        let right = sample_genesis("artifact-lineage", "catalog-a", "epoch-1", "payload-right");
        validate_genesis(&left).expect("left genesis valid");
        validate_genesis(&right).expect("right genesis valid");
        assert_eq!(left.sequence, 0);
        assert_eq!(right.sequence, 0);
        assert_ne!(left.chain, right.chain);

        let mut cross_scope_input = ChainLinkInput::append(
            &left,
            sample_payload("payload-cross"),
            Vec::new(),
            sample_producer(),
            ref_for("cross-scope-input"),
        );
        cross_scope_input.chain = right.chain.clone();
        let cross_scope = parse_chain_link(&chain_link_value(&cross_scope_input)).expect("parse cross-scope append");
        let error = validate_append(&left, &cross_scope).expect_err("cross-scope append rejected");
        assert!(error.to_string().contains("same chain scope"));
    }

    #[test]
    fn chain_ledger_append_stores_links_indexes_heads_and_receipts() {
        let root = temp_dir("chain-append");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_input = ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        );
        let genesis_value = chain_link_value(&genesis_input);
        let genesis_link = parse_chain_link(&genesis_value).expect("parse genesis");
        let genesis_append = append_chain_link(&root, &genesis_value).expect("append genesis");
        assert_eq!(genesis_append.head_before, None);
        assert_eq!(genesis_append.head_after, genesis_link.link_ref);
        assert_eq!(genesis_append.payload_ref, genesis_link.payload.artifact_ref);
        let append_receipt = genesis_append
            .receipt_value
            .collect_simple_record("chain-append-receipt-v1", Some(9))
            .expect("append receipt shape");
        let append_predicates = record_ref_sequence(&append_receipt[7], "predicates").expect("append predicates");
        assert_eq!(append_predicates, vec![genesis_append.predicate_receipt_ref.clone()]);
        let append_predicate = parse_chain_predicate_receipt(
            &crate::ledger::read_artifact(&root, &genesis_append.predicate_receipt_ref)
                .expect("append predicate receipt"),
        )
        .expect("parse append predicate");
        assert_eq!(append_predicate.predicate, GENESIS_VALID_PREDICATE);
        assert_eq!(
            crate::ledger::read_artifact(&root, &genesis_append.link_ref).expect("stored genesis"),
            genesis_value
        );
        assert_eq!(
            crate::ledger::read_artifact(&root, &genesis_append.receipt_ref).expect("stored append receipt"),
            genesis_append.receipt_value
        );

        let index = build_chain_index(&root).expect("build index after genesis");
        assert_eq!(index.heads_for_chain(&chain), vec![genesis_append.link_ref.clone()]);
        assert_eq!(index.links_for_payload(&genesis_link.payload.artifact_ref), vec![genesis_append.link_ref.clone()]);

        let second_input = ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        );
        let second_value = chain_link_value(&second_input);
        let second_link = parse_chain_link(&second_value).expect("parse second link");
        let second_append = append_chain_link(&root, &second_value).expect("append second link");
        assert_eq!(second_append.head_before, Some(genesis_append.link_ref.clone()));
        assert_eq!(second_append.head_after, second_link.link_ref);

        let index = build_chain_index(&root).expect("build index after append");
        assert_eq!(index.heads_for_chain(&chain), vec![second_append.link_ref.clone()]);
        assert_eq!(index.children_for_parent(&genesis_append.link_ref), vec![second_append.link_ref.clone()]);
        assert_eq!(index.links_for_sequence(&chain, 0), vec![genesis_append.link_ref]);
        assert_eq!(index.links_for_sequence(&chain, 1), vec![second_append.link_ref]);
    }

    #[test]
    fn chain_ledger_append_rejects_missing_payload_sequence_gaps_and_forks() {
        let missing_root = temp_dir("chain-missing-payload");
        let missing_payload_link =
            chain_link_value(&sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "missing-payload"));
        let error = append_chain_link(&missing_root, &missing_payload_link).expect_err("missing payload rejected");
        assert!(error.to_string().contains("payload"));

        let root = temp_dir("chain-rejections");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain,
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis_link = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");

        let mut gap_input = ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-gap"),
            Vec::new(),
            sample_producer(),
            ref_for("gap-input"),
        );
        gap_input.sequence += 1;
        let gap_error = append_chain_link(&root, &chain_link_value(&gap_input)).expect_err("sequence gap rejected");
        assert!(gap_error.to_string().contains("previous + 1"));

        let first_child_value = chain_link_value(&ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-b"),
        ));
        append_chain_link(&root, &first_child_value).expect("append first child");
        let fork_value = chain_link_value(&ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-c"),
            Vec::new(),
            sample_producer(),
            ref_for("append-c"),
        ));
        let fork_error = append_chain_link(&root, &fork_value).expect_err("fork rejected");
        assert!(fork_error.to_string().contains("unexpected fork"));
    }

    #[test]
    fn chain_verify_receipt_passes_for_anchor_to_head_segment() {
        let root = temp_dir("chain-verify-pass");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");
        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse second");
        append_chain_link(&root, &second_value).expect("append second");

        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify segment");
        assert_eq!(verified.decision, "pass");
        assert!(verified.diagnostics.is_empty());
        assert_eq!(verified.verified_links, vec![genesis.link_ref, second.link_ref]);
        let verify_receipt = verified
            .receipt_value
            .collect_simple_record("chain-verify-receipt-v1", Some(11))
            .expect("verify receipt shape");
        assert_eq!(
            record_ref_sequence(&verify_receipt[8], "predicates").expect("verify predicates"),
            verified.predicate_receipt_refs
        );
        let predicate_names = predicate_names(&root, &verified.predicate_receipt_refs);
        assert!(predicate_names.contains(&SEGMENT_NO_GAP_PREDICATE.to_string()));
        assert!(predicate_names.contains(&SEGMENT_NO_FORK_PREDICATE.to_string()));
        assert!(predicate_names.contains(&DESCENDS_FROM_ANCHOR_PREDICATE.to_string()));
        assert!(predicate_names.contains(&CHECKPOINT_COVERS_RANGE_PREDICATE.to_string()));
        assert_eq!(
            crate::ledger::read_artifact(&root, &verified.receipt_ref).expect("stored verify receipt"),
            verified.receipt_value
        );
    }

    #[test]
    fn chain_verify_receipt_reports_fork_gap_stale_head_and_missing_payload() {
        assert_fork_diagnostic();
        assert_gap_diagnostic();
        assert_stale_head_diagnostic();
        assert_missing_payload_diagnostic();
    }
