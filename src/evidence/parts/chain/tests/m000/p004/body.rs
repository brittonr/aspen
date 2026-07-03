
    #[test]
    fn chain_append_replay_existing_current_head_is_idempotent() {
        let root = temp_dir("chain-idempotent-append");
        let chain = ChainScope::new("evidence-ledger", "idempotent-node", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse genesis");
        let genesis_append = append_chain_link(&root, &genesis_value).expect("append genesis");
        let genesis_replay = append_chain_link(&root, &genesis_value).expect("replay current genesis");
        assert_eq!(genesis_replay, genesis_append);
        assert_eq!(build_chain_index(&root).expect("index after replay").heads_for_chain(&chain), vec![genesis.link_ref.clone()]);

        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse second");
        let second_append = append_chain_link(&root, &second_value).expect("append second");
        let second_replay = append_chain_link(&root, &second_value).expect("replay current second");
        assert_eq!(second_replay, second_append);
        assert_eq!(second_replay.head_before.as_deref(), Some(genesis.link_ref.as_str()));
        assert_eq!(build_chain_index(&root).expect("index after second replay").heads_for_chain(&chain), vec![second.link_ref]);

        let stale_replay = append_chain_link(&root, &genesis_value).expect_err("stale historical replay rejected");
        assert!(stale_replay.to_string().contains("already present but is not the current chain head"));
    }

    #[test]
    fn chain_verify_denies_duplicate_sequence_conflicts_and_tampered_payload_refs() {
        let root = temp_dir("chain-duplicate-sequence-proof");
        let chain = ChainScope::new("evidence-ledger", "duplicate-sequence", "epoch-1");
        let genesis = import_genesis_link(&root, chain.clone(), "payload-root");
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
        import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-right"),
                Vec::new(),
                sample_producer(),
                ref_for("append-right"),
            ),
        );
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&left.link_ref))
            .expect("verify duplicate sequence conflict");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "fork"));
        assert!(has_diagnostic(&verified, "sequence-conflict"));

        let tamper_root = temp_dir("chain-tampered-payload-proof");
        let tamper_chain = ChainScope::new("evidence-ledger", "tampered-payload", "epoch-1");
        let tampered_value = chain_link_value(&sample_genesis_input(
            &tamper_chain.scope,
            &tamper_chain.id,
            &tamper_chain.epoch,
            "missing-payload",
        ));
        let tampered_link = parse_chain_link(&tampered_value).expect("parse tampered payload link");
        crate::ledger::import_artifact(&tamper_root, &tampered_value).expect("raw import tampered payload link");
        let tampered = verify_chain_segment(&tamper_root, &tamper_chain, None, Some(&tampered_link.link_ref))
            .expect("verify tampered payload ref");
        assert_eq!(tampered.decision, "fail");
        assert!(has_diagnostic(&tampered, "missing-payload"));
    }

    #[test]
    fn checkpoint_anchor_and_signed_receipt_refs_protect_reachable_chain_evidence_from_gc() {
        let fixture = checkpoint_fixture("chain-checkpoint-retention-proof");
        let unrelated = crate::ledger::import_artifact(
            &fixture.root,
            &record("unanchored-test-artifact", vec![string("remove-me")]),
        )
        .expect("import unanchored artifact");
        let reachable = fixture.reachable_refs();
        pin_refs(&fixture.root, &reachable);
        assert_refs_available(&fixture.root, &reachable);

        let retention_evidence = delete_evidence_for(
            &fixture.root,
            "unanchored-chain-gc",
            &unrelated.artifact_ref,
            &unrelated.artifact_kind,
            crate::retention::CLASS_PUBLIC_ARTIFACT,
        );
        let apply_ref = delete_apply_ref_for(
            &fixture.root,
            &unrelated.artifact_ref,
            &unrelated.artifact_kind,
            crate::retention::CLASS_PUBLIC_ARTIFACT,
            &retention_evidence,
        );
        let gc = crate::ledger::gc(&fixture.root, crate::ledger::GcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: std::slice::from_ref(&apply_ref),
        })
        .expect("ledger gc unanchored artifact");
        assert_eq!(gc.decision, "pass");
        assert!(gc.removed_refs.contains(&unrelated.artifact_ref));
        assert!(crate::ledger::read_artifact(&fixture.root, &unrelated.artifact_ref).is_err());
        assert_refs_available(&fixture.root, &reachable);
        verify_signed_chain_receipt(
            &crate::ledger::read_artifact(&fixture.root, &fixture.signed_append_ref).expect("signed append retained"),
            "root",
            "key",
        )
        .expect("verify retained signed append receipt");
        verify_signed_chain_receipt(
            &crate::ledger::read_artifact(&fixture.root, &fixture.signed_verify_ref).expect("signed verify retained"),
            "root",
            "key",
        )
        .expect("verify retained signed verify receipt");
    }

    struct CheckpointFixture {
        root: PathBuf,
        anchor_ref: String,
        checkpoint_ref: String,
        genesis_link_ref: String,
        second_link_ref: String,
        genesis_payload_ref: String,
        second_payload_ref: String,
        genesis_append_receipt_ref: String,
        second_append_receipt_ref: String,
        genesis_predicate_ref: String,
        second_predicate_ref: String,
        verify_receipt_ref: String,
        verify_predicate_refs: Vec<String>,
        signed_append_ref: String,
        signed_verify_ref: String,
    }

    impl CheckpointFixture {
        fn reachable_refs(&self) -> Vec<String> {
            let mut refs = vec![
                self.anchor_ref.clone(),
                self.checkpoint_ref.clone(),
                self.genesis_link_ref.clone(),
                self.second_link_ref.clone(),
                self.genesis_payload_ref.clone(),
                self.second_payload_ref.clone(),
                self.genesis_append_receipt_ref.clone(),
                self.second_append_receipt_ref.clone(),
                self.genesis_predicate_ref.clone(),
                self.second_predicate_ref.clone(),
                self.verify_receipt_ref.clone(),
                self.signed_append_ref.clone(),
                self.signed_verify_ref.clone(),
            ];
            refs.extend(self.verify_predicate_refs.clone());
            refs.sort();
            refs.dedup();
            refs
        }
    }

    fn checkpoint_fixture(name: &str) -> CheckpointFixture {
        let root = temp_dir(name);
        let chain = ChainScope::new("evidence-ledger", name, "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse checkpoint genesis");
        let genesis_append = append_chain_link(&root, &genesis_value).expect("append checkpoint genesis");
        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse checkpoint second");
        let second_append = append_chain_link(&root, &second_value).expect("append checkpoint second");
        let policy_ref = ref_for("checkpoint-policy");
        let anchor = publish_chain_anchor(
            &root,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish checkpoint anchor");
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify checkpoint range");
        let checkpoint = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain,
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![ref_for("membership")],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");
        let signed_append = sign_chain_receipt(
            &second_append.receipt_value,
            "node:local",
            "root",
            "key",
            std::slice::from_ref(&genesis_append.receipt_ref),
        )
        .expect("sign append receipt");
        let signed_append_ref = crate::ledger::import_artifact(&root, &signed_append)
            .expect("import signed append")
            .artifact_ref;
        let signed_verify = sign_chain_receipt(
            &verified.receipt_value,
            "node:local",
            "root",
            "key",
            std::slice::from_ref(&signed_append_ref),
        )
        .expect("sign verify receipt");
        let signed_verify_ref = crate::ledger::import_artifact(&root, &signed_verify)
            .expect("import signed verify")
            .artifact_ref;

        CheckpointFixture {
            root,
            anchor_ref: anchor.anchor_ref,
            checkpoint_ref: checkpoint.checkpoint_ref,
            genesis_link_ref: genesis.link_ref,
            second_link_ref: second.link_ref,
            genesis_payload_ref: genesis_append.payload_ref,
            second_payload_ref: second_append.payload_ref,
            genesis_append_receipt_ref: genesis_append.receipt_ref,
            second_append_receipt_ref: second_append.receipt_ref,
            genesis_predicate_ref: genesis_append.predicate_receipt_ref,
            second_predicate_ref: second_append.predicate_receipt_ref,
            verify_receipt_ref: verified.receipt_ref,
            verify_predicate_refs: verified.predicate_receipt_refs,
            signed_append_ref,
            signed_verify_ref,
        }
    }

    fn pin_refs(root: &Path, refs: &[String]) {
        for artifact_ref in refs {
            crate::ledger::pin_artifact(root, artifact_ref).expect("pin reachable chain artifact");
        }
    }

    fn assert_refs_available(root: &Path, refs: &[String]) {
        for artifact_ref in refs {
            crate::ledger::read_artifact(root, artifact_ref).expect("reachable chain artifact remains available");
        }
    }

    fn delete_evidence_for(
        root: &Path,
        label: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
    ) -> crate::retention::DestructiveEvidence {
        let requester_ref = ref_for(&format!("{label}-requester"));
        crate::retention::DestructiveEvidence {
            requester_ref: Some(requester_ref.clone()),
            policy_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_POLICY,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            authority_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_AUTHORITY,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            evidence_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: vec![store_delete_admission(
                root,
                crate::retention::ADMISSION_KIND_REFERENCE_INDEX,
                label,
                &requester_ref,
                object_ref,
                object_kind,
                retention_class,
            )],
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn store_delete_admission(
        root: &Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
    ) -> String {
        let bound_refs = vec![object_ref.to_string()];
        let diagnostics = vec![format!("{label}-{kind}")];
        crate::retention::store_evidence_admission(root, &crate::retention::EvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            bound_refs: &bound_refs,
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &diagnostics,
        })
        .expect("store delete evidence admission")
        .admission_ref
    }

    fn delete_apply_ref_for(
        root: &Path,
        object_ref: &str,
        object_kind: &str,
        retention_class: &str,
        evidence: &crate::retention::DestructiveEvidence,
    ) -> String {
        let plan = crate::retention::store_gc_plan(crate::retention::GcPlanInput {
            root,
            subsystem: "ledger-gc",
            object_ref,
            object_kind,
            retention_class,
            action: crate::retention::ACTION_DELETE,
            evidence,
        })
        .expect("store unanchored GC plan");
        crate::retention::apply_gc_plan(crate::retention::GcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply unanchored GC plan")
        .apply_ref
    }
