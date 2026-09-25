
    #[test]
    fn chain_anchor_and_checkpoint_indexes_track_accepted_heads_and_freshness() {
        let root = temp_dir("chain-checkpoint");
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

        let policy_ref = ref_for("checkpoint-policy");
        let membership_ref = ref_for("membership");
        let anchor = publish_chain_anchor(
            &root,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify checkpoint range");
        assert_eq!(verified.decision, "pass");
        let checkpoint = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![membership_ref],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");

        let index = build_chain_index(&root).expect("index checkpoints");
        assert_eq!(index.anchors_for_chain(&chain), vec![anchor.anchor_ref.clone()]);
        assert_eq!(index.anchor_links_for_chain(&chain), vec![genesis.link_ref.clone()]);
        assert_eq!(index.checkpoints_for_chain(&chain), vec![checkpoint.checkpoint_ref.clone()]);
        assert_eq!(index.checkpoint_heads_for_chain(&chain), vec![second.link_ref.clone()]);
        assert_eq!(checkpoint.range_predicate_ref, checkpoint_range_predicate(&root, &verified));
        validate_chain_checkpoint_freshness(&root, &chain, &checkpoint.checkpoint_ref, Some(&second.link_ref))
            .expect("fresh checkpoint");

        let third_value = chain_link_value(&ChainLinkInput::append(
            &second,
            stored_payload(&root, "payload-c"),
            Vec::new(),
            sample_producer(),
            ref_for("append-third"),
        ));
        append_chain_link(&root, &third_value).expect("append third");
        let stale = validate_chain_checkpoint_freshness(&root, &chain, &checkpoint.checkpoint_ref, None)
            .expect_err("checkpoint becomes stale after head advances");
        assert!(stale.to_string().contains("stale"));
    }

    #[test]
    fn chain_checkpoint_rejects_mismatched_verify_receipt() {
        let root = temp_dir("chain-checkpoint-mismatch");
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
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&genesis.link_ref))
            .expect("verify genesis range");
        let wrong_head = ref_for("wrong-head");
        let error = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: wrong_head,
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("mismatched checkpoint rejected");
        assert!(error.to_string().contains("head"));

        let missing_predicate = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: genesis.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: ref_for("missing-range-predicate"),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("missing range predicate rejected");
        assert!(missing_predicate.to_string().contains("predicate"));

        let case = MismatchCase {
            root: &root,
            chain: &chain,
            genesis: &genesis,
            verified: &verified,
        };
        assert_bad_subjects(&case);
        assert_wrong_kind(&case);
    }

    #[test]
    fn chain_checkpoint_acceptance_rejects_stale_head() {
        let root = temp_dir("chain-checkpoint-stale-accept");
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
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&genesis.link_ref))
            .expect("verify initial range");
        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        append_chain_link(&root, &second_value).expect("append second");

        let error = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: genesis.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("stale checkpoint rejected at acceptance");
        assert!(error.to_string().contains("current chain head"));
    }

    #[test]
    fn chain_checkpoint_acceptance_requires_monotonic_prior() {
        let root = temp_dir("chain-checkpoint-prior");
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
        let first_verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&genesis.link_ref))
            .expect("verify first checkpoint range");
        let first = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: genesis.link_ref.clone(),
            verify_receipt_ref: first_verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &first_verified),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept first checkpoint");

        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse second");
        append_chain_link(&root, &second_value).expect("append second");
        let second_verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify second checkpoint range");
        let second_range_predicate = checkpoint_range_predicate(&root, &second_verified);

        let missing_prior = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: second_verified.receipt_ref.clone(),
            range_predicate_ref: second_range_predicate.clone(),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("successive checkpoint requires prior");
        assert!(missing_prior.to_string().contains("prior checkpoint"));

        let accepted = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain,
            prior_checkpoint_ref: Some(first.checkpoint_ref.clone()),
            anchor_link_ref: genesis.link_ref,
            head_ref: second.link_ref,
            verify_receipt_ref: second_verified.receipt_ref,
            range_predicate_ref: second_range_predicate,
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("successive checkpoint with prior accepted");
        assert_eq!(accepted.prior_checkpoint_ref.as_deref(), Some(first.checkpoint_ref.as_str()));
    }

    struct MismatchCase<'a> {
        root: &'a Path,
        chain: &'a ChainScope,
        genesis: &'a ChainLink,
        verified: &'a ChainVerify,
    }
