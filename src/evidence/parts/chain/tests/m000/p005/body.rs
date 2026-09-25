
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
