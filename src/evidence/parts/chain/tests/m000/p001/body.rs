
    fn assert_fork_diagnostic() {
        let root = temp_dir("chain-verify-fork");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, scope.clone(), "payload-a");
        import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-b"),
                Vec::new(),
                sample_producer(),
                ref_for("append-b"),
            ),
        );
        import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-c"),
                Vec::new(),
                sample_producer(),
                ref_for("append-c"),
            ),
        );
        let verified = verify_chain_segment(&root, &scope, None, None).expect("verify forked chain");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "fork"));
    }

    fn assert_gap_diagnostic() {
        let root = temp_dir("chain-verify-gap");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, scope.clone(), "payload-a");
        let mut input = ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-gap"),
            Vec::new(),
            sample_producer(),
            ref_for("gap-input"),
        );
        input.sequence += 1;
        let link = import_raw_link(&root, &input);
        let verified = verify_chain_segment(&root, &scope, None, Some(&link.link_ref)).expect("verify gap chain");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "gap"));
    }

    fn assert_stale_head_diagnostic() {
        let root = temp_dir("chain-verify-stale");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            scope.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse stale genesis");
        append_chain_link(&root, &genesis_value).expect("append stale genesis");
        let child_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        append_chain_link(&root, &child_value).expect("append stale child");
        let verified = verify_chain_segment(&root, &scope, None, Some(&genesis.link_ref)).expect("verify stale head");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "stale-head"));
    }

    fn assert_missing_payload_diagnostic() {
        let root = temp_dir("chain-verify-missing-payload");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let value = chain_link_value(&sample_genesis_input(&scope.scope, &scope.id, &scope.epoch, "missing-payload"));
        let link = parse_chain_link(&value).expect("parse missing payload link");
        crate::ledger::import_artifact(&root, &value).expect("raw import missing payload link");
        let verified = verify_chain_segment(&root, &scope, None, Some(&link.link_ref)).expect("verify missing payload");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "missing-payload"));
    }

    #[test]
    fn signed_chain_receipts_can_be_link_payloads_without_changing_subject_hashes() {
        let root = temp_dir("chain-signed-receipts");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let append = append_chain_link(&root, &genesis_value).expect("append genesis");
        let append_subject_ref = canonical_hash(&append.receipt_value).expect("append receipt subject ref");
        assert_eq!(append_subject_ref, append.receipt_ref);

        let signed_append =
            sign_chain_receipt(&append.receipt_value, "node:local", "root", "key", &[]).expect("sign append receipt");
        let verified_signed_append =
            verify_signed_chain_receipt(&signed_append, "root", "key").expect("verify signed append receipt");
        assert_eq!(verified_signed_append.subject_ref, append.receipt_ref);
        assert_eq!(
            canonical_hash(&append.receipt_value).expect("append receipt ref after signing"),
            append.receipt_ref
        );

        let signed_append_ref = canonical_hash(&signed_append).expect("signed append receipt ref");
        crate::ledger::import_artifact(&root, &signed_append).expect("import signed append receipt");
        let signed_chain = ChainScope::new("evidence-ledger", "signed-receipts", "epoch-1");
        let signed_payload_link_value = chain_link_value(&ChainLinkInput::genesis(
            signed_chain.clone(),
            signed_receipt_payload(signed_append_ref.clone()),
            Vec::new(),
            sample_producer(),
            ref_for("signed-receipt-link"),
        ));
        let linked = append_chain_link(&root, &signed_payload_link_value).expect("append signed receipt payload link");
        let linked_link = parse_chain_link(&signed_payload_link_value).expect("parse signed payload link");
        assert_eq!(linked_link.payload.artifact_ref, signed_append_ref);

        let verified_segment = verify_chain_segment(&root, &signed_chain, None, Some(&linked.link_ref))
            .expect("verify signed receipt chain segment");
        assert_eq!(verified_segment.decision, "pass");
        let signed_verify = sign_chain_receipt(
            &verified_segment.receipt_value,
            "node:local",
            "root",
            "key",
            std::slice::from_ref(&signed_append_ref),
        )
        .expect("sign verify receipt");
        let verified_signed_verify =
            verify_signed_chain_receipt(&signed_verify, "root", "key").expect("verify signed verify receipt");
        assert_eq!(verified_signed_verify.subject_ref, verified_segment.receipt_ref);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_chain_segment_append_no_gap_fork_and_anchor_properties(tc: TestCase) {
        let length = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(5));
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let labels = (0..length).map(|index| format!("hegel-{salt}-payload-{index}")).collect::<Vec<_>>();
        let chain = ChainScope::new("evidence-ledger", format!("hegel-node-{salt}"), "epoch-1");

        let deterministic_first = deterministic_link_refs(&chain, &labels, salt);
        let deterministic_second = deterministic_link_refs(&chain, &labels, salt);
        assert_eq!(deterministic_first, deterministic_second);

        let root = temp_dir("chain-hegel-linear");
        let linear = append_linear_chain(&root, &chain, &labels);
        assert_eq!(linear.len(), length);
        let linear_head = linear.last().expect("non-empty linear chain");
        let no_gap = verify_chain_segment(&root, &chain, Some(&linear[0].link_ref), Some(&linear_head.link_ref))
            .expect("verify generated no-gap chain");
        assert_eq!(no_gap.decision, "pass");
        assert!(no_gap.diagnostics.is_empty());

        let fork_root = temp_dir("chain-hegel-fork");
        let fork_chain = ChainScope::new("evidence-ledger", format!("hegel-fork-node-{salt}"), "epoch-1");
        let fork_genesis = import_genesis_link(&fork_root, fork_chain.clone(), &format!("hegel-{salt}-fork-root"));
        let left = import_raw_link(
            &fork_root,
            &ChainLinkInput::append(
                &fork_genesis,
                stored_payload(&fork_root, &format!("hegel-{salt}-fork-left")),
                Vec::new(),
                sample_producer(),
                ref_for(&format!("hegel-{salt}-fork-left-input")),
            ),
        );
        let right = import_raw_link(
            &fork_root,
            &ChainLinkInput::append(
                &fork_genesis,
                stored_payload(&fork_root, &format!("hegel-{salt}-fork-right")),
                Vec::new(),
                sample_producer(),
                ref_for(&format!("hegel-{salt}-fork-right-input")),
            ),
        );
        let production_fork =
            verify_chain_segment(&fork_root, &fork_chain, Some(&fork_genesis.link_ref), Some(&left.link_ref))
                .expect("production fork verification");
        assert_eq!(production_fork.decision, "fail");
        assert!(has_diagnostic(&production_fork, "fork"));

        let diagnostic_fork = verify_chain_segment_with_policy(
            &fork_root,
            &fork_chain,
            Some(&fork_genesis.link_ref),
            Some(&left.link_ref),
            ChainForkPolicy::RetainForkEvidence,
        )
        .expect("diagnostic fork verification");
        assert_eq!(diagnostic_fork.decision, "pass");
        assert!(has_diagnostic(&diagnostic_fork, "fork"));
        assert_eq!(diagnostic_fork.verified_links, vec![fork_genesis.link_ref.clone(), left.link_ref.clone()]);

        let non_descending = verify_chain_segment(&fork_root, &fork_chain, Some(&left.link_ref), Some(&right.link_ref))
            .expect("non-descending anchor verification");
        assert_eq!(non_descending.decision, "fail");
        assert!(has_diagnostic(&non_descending, "anchor-descent"));
    }

    #[test]
    fn diagnostic_fork_policy_retains_evidence_while_verifying_selected_head() {
        let root = temp_dir("chain-diagnostic-fork");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, chain.clone(), "payload-a");
        let left = import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-left"),
                Vec::new(),
                sample_producer(),
                ref_for("append-left"),
            ),
        );
        let right = import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-right"),
                Vec::new(),
                sample_producer(),
                ref_for("append-right"),
            ),
        );

        let production = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&left.link_ref))
            .expect("production verify fork");
        assert_eq!(production.decision, "fail");
        assert!(has_diagnostic(&production, "fork"));

        let diagnostic = verify_chain_segment_with_policy(
            &root,
            &chain,
            Some(&genesis.link_ref),
            Some(&left.link_ref),
            ChainForkPolicy::RetainForkEvidence,
        )
        .expect("diagnostic verify fork");
        assert_eq!(diagnostic.decision, "pass");
        assert!(has_diagnostic(&diagnostic, "fork"));
        assert_eq!(diagnostic.verified_links, vec![genesis.link_ref.clone(), left.link_ref.clone()]);

        let index = build_chain_index(&root).expect("index fork evidence");
        let evidence_refs = index.fork_evidence_for_parent(&genesis.link_ref);
        assert!(!evidence_refs.is_empty());
        let evidence = index.fork_evidence_by_ref.get(&evidence_refs[0]).expect("indexed fork evidence");
        assert_eq!(evidence.parent_ref.as_deref(), Some(genesis.link_ref.as_str()));
        assert_eq!(evidence.selected_head.as_deref(), Some(left.link_ref.as_str()));
        assert_eq!(evidence.profile, ChainForkPolicy::RetainForkEvidence.profile());
        assert_eq!(evidence.decision, ChainForkPolicy::RetainForkEvidence.decision_for_fork());
        assert!(evidence.child_refs.contains(&left.link_ref));
        assert!(evidence.child_refs.contains(&right.link_ref));
    }
