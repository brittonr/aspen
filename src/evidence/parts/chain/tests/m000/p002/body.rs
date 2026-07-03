
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

    fn assert_bad_subjects(case: &MismatchCase<'_>) {
        let wrong_range_subjects = vec![ref_for("wrong-range-subject")];
        let checkpoint_context_refs = scope_context_refs(case.chain).expect("scope context");
        let predicate_checkpoint_checks = vec![ChainCheck::pass("checkpoint-range-coverage")];
        let tampered_predicate_value = chain_predicate_receipt_value(&ChainPredicateReceiptValueInput {
            predicate: CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: &wrong_range_subjects,
            input_refs: &case.verified.payload_refs,
            context_refs: &checkpoint_context_refs,
            checks: &predicate_checkpoint_checks,
        });
        let tampered_predicate_ref = crate::ledger::import_artifact(case.root, &tampered_predicate_value)
            .expect("import tampered predicate")
            .artifact_ref;
        let fake_verify_link_refs = vec![case.genesis.link_ref.clone()];
        let fake_verify_diagnostics = Vec::new();
        let fake_verify_predicate_refs = vec![tampered_predicate_ref.clone()];
        let fake_verify_receipt = ChainVerifyReceiptValueInput {
            decision: "pass",
            chain: case.chain,
            anchor_ref: Some(&case.genesis.link_ref),
            expected_head: Some(&case.genesis.link_ref),
            discovered_heads: &fake_verify_link_refs,
            verified_links: &fake_verify_link_refs,
            payload_refs: &case.verified.payload_refs,
            diagnostics: &fake_verify_diagnostics,
        };
        let fake_verify_value = chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
            receipt: fake_verify_receipt,
            predicate_receipt_refs: &fake_verify_predicate_refs,
            fork_policy: ChainForkPolicy::RejectUnexpectedForks,
        });
        let fake_verify_ref = crate::ledger::import_artifact(case.root, &fake_verify_value)
            .expect("import fake verify receipt")
            .artifact_ref;
        let tampered_predicate = accept_chain_checkpoint(case.root, &ChainCheckpointInput {
            chain: case.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: case.genesis.link_ref.clone(),
            head_ref: case.genesis.link_ref.clone(),
            verify_receipt_ref: fake_verify_ref,
            range_predicate_ref: tampered_predicate_ref,
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("tampered range predicate rejected");
        assert!(tampered_predicate.to_string().contains("subjects"));
    }

    fn assert_wrong_kind(case: &MismatchCase<'_>) {
        let wrong_predicate_ref = case
            .verified
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = crate::ledger::read_artifact(case.root, predicate_ref).expect("read predicate");
                parse_chain_predicate_receipt(&value).expect("parse predicate").predicate == SEGMENT_NO_GAP_PREDICATE
            })
            .expect("no-gap predicate")
            .clone();
        let wrong_predicate = accept_chain_checkpoint(case.root, &ChainCheckpointInput {
            chain: case.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: case.genesis.link_ref.clone(),
            head_ref: case.genesis.link_ref.clone(),
            verify_receipt_ref: case.verified.receipt_ref.clone(),
            range_predicate_ref: wrong_predicate_ref,
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("wrong range predicate rejected");
        assert!(wrong_predicate.to_string().contains("range predicate"));
    }

    fn sample_genesis(scope: &str, id: &str, epoch: &str, payload_label: &str) -> ChainLink {
        let input = sample_genesis_input(scope, id, epoch, payload_label);
        let link = parse_chain_link(&chain_link_value(&input)).expect("parse genesis");
        validate_genesis(&link).expect("validate genesis");
        link
    }

    fn sample_genesis_input(scope: &str, id: &str, epoch: &str, payload_label: &str) -> ChainLinkInput {
        ChainLinkInput::genesis(
            ChainScope::new(scope, id, epoch),
            sample_payload(payload_label),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("genesis-input"),
        )
    }

    fn sample_payload(label: &str) -> ChainPayload {
        ChainPayload::new("gate-receipt", ref_for(label), "molten.harness.gate-receipt.v1")
    }

    fn deterministic_link_refs(chain: &ChainScope, labels: &[String], salt: u64) -> Vec<String> {
        let mut previous = None;
        let mut refs = Vec::with_capacity(labels.len());
        for (index, label) in labels.iter().enumerate() {
            let input_ref = ref_for(&format!("hegel-{salt}-input-{index}"));
            let input = if let Some(previous) = &previous {
                ChainLinkInput::append(previous, sample_payload(label), Vec::new(), sample_producer(), input_ref)
            } else {
                ChainLinkInput::genesis(chain.clone(), sample_payload(label), Vec::new(), sample_producer(), input_ref)
            };
            let link = parse_chain_link(&chain_link_value(&input)).expect("parse deterministic link");
            refs.push(link.link_ref.clone());
            previous = Some(link);
        }
        refs
    }

    fn append_linear_chain(root: &Path, chain: &ChainScope, labels: &[String]) -> Vec<ChainLink> {
        let mut previous = None;
        let mut links = Vec::with_capacity(labels.len());
        for (index, label) in labels.iter().enumerate() {
            let input = if let Some(previous) = &previous {
                ChainLinkInput::append(
                    previous,
                    stored_payload(root, label),
                    Vec::new(),
                    sample_producer(),
                    ref_for(&format!("linear-input-{index}")),
                )
            } else {
                ChainLinkInput::genesis(
                    chain.clone(),
                    stored_payload(root, label),
                    Vec::new(),
                    sample_producer(),
                    ref_for(&format!("linear-input-{index}")),
                )
            };
            let value = chain_link_value(&input);
            append_chain_link(root, &value).expect("append linear chain link");
            let link = parse_chain_link(&value).expect("parse linear chain link");
            previous = Some(link.clone());
            links.push(link);
        }
        links
    }

    fn stored_payload(root: &Path, label: &str) -> ChainPayload {
        let artifact = record("test-payload", vec![string(label)]);
        let imported = crate::ledger::import_artifact(root, &artifact).expect("import payload");
        ChainPayload::new("test-payload", imported.artifact_ref, "molten.test.payload.v1")
    }

    fn import_genesis_link(root: &Path, chain: ChainScope, payload_label: &str) -> ChainLink {
        let input = ChainLinkInput::genesis(
            chain,
            stored_payload(root, payload_label),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        );
        import_raw_link(root, &input)
    }
