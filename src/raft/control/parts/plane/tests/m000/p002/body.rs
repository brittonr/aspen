
    /// A read behind the commit index is stale, and a group manifest naming an unsupported state machine is refused.
    fn assert_stale_read_and_unsupported_state_machine_denied(runtime: &ControlRegistryRuntime) {
        let stale_read = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(RAFT_TEST_SEQUENCE_STEP),
            read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
            namespace: "receipt-index".to_string(),
            name: "duplicate-scope".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("stale read");
        assert_eq!(stale_read.decision, RAFT_DECISION_DENY);
        assert!(stale_read.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale read-index")));

        let wrong_state_machine_manifest = raft_group_manifest_value(&RaftGroupManifestInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            members: vec![test_ref("member-a")],
            state_machine: "unsupported-state-machine".to_string(),
            command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
            read_mode: READ_MODE_READ_INDEX.to_string(),
            snapshot_policy_ref: test_ref("snapshot-policy"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("wrong state machine manifest");
        let wrong_state_machine = new_control_registry_runtime(&wrong_state_machine_manifest).expect_err("state machine denial");
        assert!(wrong_state_machine.to_string().contains("unsupported raft state machine"));
    }

    #[test]
    fn raft_control_registry_snapshot_restore_equivalence_and_negative_evidence() {
        // r[verify molten.consensus_state_machine_proof.snapshot_restore_equivalence]
        let runtime = run_control_registry_fixture().expect("runtime");
        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value.clone(),
            log_entries: Vec::new(),
        })
        .expect("recover");
        assert_eq!(recovery.decision, RAFT_DECISION_PASS);
        assert_eq!(snapshot.state.state_ref, runtime.state.state_ref);
        assert_eq!(recovery.restored_state_ref.as_deref(), Some(snapshot.state.state_ref.as_str()));
        validate_content_ref(&snapshot.snapshot_ref).expect("snapshot ref");
        validate_content_ref(&recovery.receipt_ref).expect("recovery receipt ref");

        let tampered_snapshot = replace_snapshot_field(
            &snapshot.value,
            SNAPSHOT_CONTENT_REF_FIELD_INDEX,
            record("content-ref", vec![string(test_ref("tampered-content"))]),
        );
        let tampered = parse_raft_snapshot(&tampered_snapshot).expect_err("tampered snapshot denial");
        assert!(tampered.to_string().contains("raft snapshot state/content ref mismatch"));

        let missing_state_snapshot = replace_snapshot_field(
            &snapshot.value,
            SNAPSHOT_STATE_FIELD_INDEX,
            record("state", vec![record("none", Vec::new())]),
        );
        assert!(parse_raft_snapshot(&missing_state_snapshot).is_err());

        let mismatched_group = recover_control_registry(&RaftRecoveryInput {
            group_ref: test_ref("wrong-recovery-group"),
            snapshot: snapshot.value,
            log_entries: Vec::new(),
        })
        .expect("mismatched group recovery");
        assert_eq!(mismatched_group.decision, RAFT_DECISION_DENY);
        assert!(mismatched_group
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("snapshot group does not match recovery group")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_bounded_registry_logs_are_deterministic_and_control_only(tc: hegel::TestCase) {
        let command_count = usize::try_from(tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(4)))
            .expect("command count");
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut left = new_control_registry_runtime(&manifest).expect("left runtime");
        let mut right = new_control_registry_runtime(&manifest).expect("right runtime");
        for index in 0..command_count {
            let target = test_ref(&format!("target-{index}"));
            let command = control_registry_command_value(&ControlRegistryCommandInput {
                operation: "set-receipt-index".to_string(),
                namespace: "receipt-index".to_string(),
                name: format!("scope-{index}"),
                target_ref: Some(target),
            })
            .expect("generated command");
            let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
                group_ref: left.manifest.manifest_ref.clone(),
                client_session: "client:property".to_string(),
                sequence: u64::try_from(index + 1).expect("sequence"),
                command,
                authority_refs: auth(),
                policy_refs: left.manifest.policy_refs.clone(),
                resource_refs: left.manifest.resource_refs.clone(),
                evidence_refs: vec![test_ref("evidence")],
            })
            .expect("generated envelope");
            let left_result = propose_control_registry_command(&mut left, &envelope).expect("left proposal");
            let right_result = propose_control_registry_command(&mut right, &envelope).expect("right proposal");
            assert_eq!(left_result.decision, "pass");
            assert_eq!(left_result.registry_receipt.receipt_ref, right_result.registry_receipt.receipt_ref);
        }
        assert_eq!(left.state.state_ref, right.state.state_ref);
        assert_eq!(left.log_entries.len(), command_count);
        for entry in &left.log_entries {
            let envelope = parse_raft_command_envelope(&entry.command).expect("entry command envelope");
            assert!(parse_control_registry_command(&envelope.command).is_ok());
        }
    }

    #[test]
    fn cluster_consensus_config_selects_manifest_profile_and_denies_unknowns() {
        // r[verify molten.consensus.cluster_config_selection]
        let input = cluster_config_manifest_input();
        let raft_config = ClusterConsensusConfig {
            algorithm_profile: CONSENSUS_PROFILE_RAFT.to_string(),
            profile_version: Some(CONSENSUS_PROFILE_VERSION_RAFT.to_string()),
            placement_ref: Some(test_ref("cluster-raft-placement")),
            required_evidence_refs: vec![test_ref("cluster-raft-evidence")],
        };
        let raft_manifest_value = raft_group_manifest_value_with_cluster_config(&input, &raft_config)
            .expect("configured raft manifest");
        let raft_manifest = parse_raft_group_manifest(&raft_manifest_value).expect("configured raft parse");
        assert_eq!(raft_manifest.algorithm_profile, CONSENSUS_PROFILE_RAFT);
        assert_eq!(raft_manifest.admitted_profile_version, CONSENSUS_PROFILE_VERSION_RAFT);
        assert_eq!(raft_manifest.placement_ref, raft_config.placement_ref);
        assert_eq!(raft_manifest.required_evidence_refs, raft_config.required_evidence_refs);
        let runtime = new_control_registry_runtime(&raft_manifest_value).expect("configured raft runtime");
        assert_eq!(runtime.manifest.algorithm_profile, CONSENSUS_PROFILE_RAFT);

        let leaderless_config = ClusterConsensusConfig {
            algorithm_profile: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            profile_version: Some(CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL.to_string()),
            placement_ref: Some(test_ref("cluster-leaderless-placement")),
            required_evidence_refs: vec![test_ref("leaderless-proof"), test_ref("leaderless-simulation")],
        };
        let leaderless_manifest_value = raft_group_manifest_value_with_cluster_config(&input, &leaderless_config)
            .expect("configured leaderless manifest");
        let leaderless_manifest = parse_raft_group_manifest(&leaderless_manifest_value).expect("leaderless parse");
        assert_eq!(leaderless_manifest.algorithm_profile, CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL);
        assert_eq!(leaderless_manifest.production_status, PRODUCTION_STATUS_EXPERIMENTAL);
        let leaderless_runtime = new_control_registry_production_runtime(&leaderless_manifest_value)
            .expect_err("leaderless denied");
        assert!(leaderless_runtime.to_string().contains("not admitted for production runtime"));

        let unknown_config = ClusterConsensusConfig {
            algorithm_profile: "raftt".to_string(),
            ..ClusterConsensusConfig::default()
        };
        let unknown_error = raft_group_manifest_value_with_cluster_config(&input, &unknown_config)
            .expect_err("unknown consensus profile denied");
        assert!(unknown_error.to_string().contains("unsupported consensus algorithm profile raftt"));
    }

    #[test]
    fn consensus_profiles_reads_and_non_claims_are_fail_closed() {
        // r[verify molten.consensus.algorithm_profile_manifest]
        // r[verify molten.consensus.leaderless_profile_boundary]
        // r[verify molten.consensus.read_consistency_modes]
        // r[verify molten.consensus.non_claim_boundaries]
        let manifest_value = control_registry_fixture_manifest_value().expect("manifest");
        let manifest = parse_raft_group_manifest(&manifest_value).expect("parse manifest");
        assert_eq!(manifest.algorithm_profile, CONSENSUS_PROFILE_RAFT);
        assert_eq!(manifest.production_status, PRODUCTION_STATUS_MODEL_ONLY);
        assert_eq!(manifest.read_consistency_support.len(), EXPECTED_READ_CONSISTENCY_MODE_COUNT);
        assert!(manifest.placement_ref.is_some());

        let runtime = run_control_registry_fixture().expect("runtime");
        let local_stale = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(RAFT_TEST_SEQUENCE_STEP),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("local stale read");
        assert_eq!(local_stale.decision, RAFT_DECISION_PASS);
        assert_eq!(local_stale.read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert!(to_text(&local_stale.value).expect("read text").contains("local-stale-non-authoritative"));

        let leaderless_profile = leaderless_experimental_algorithm_profile_input(
            vec![test_ref("membership-policy")],
            Some(test_ref("placement")),
            vec![test_ref("proof"), test_ref("simulation")],
        );
        let leaderless_manifest = raft_group_manifest_value_with_profile(&RaftGroupManifestInput {
            group_id: DEFAULT_GROUP_ID.to_string(),
            members: vec![test_ref("member-a"), test_ref("member-b"), test_ref("member-c")],
            state_machine: CONTROL_REGISTRY_STATE_MACHINE.to_string(),
            command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
            read_mode: READ_MODE_READ_INDEX.to_string(),
            snapshot_policy_ref: test_ref("snapshot-policy"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
        }, &leaderless_profile)
        .expect("leaderless manifest");
        let leaderless_runtime = new_control_registry_production_runtime(&leaderless_manifest)
            .expect_err("leaderless denied");
        assert!(leaderless_runtime.to_string().contains("not admitted for production runtime"));

        let claim = consensus_claim_boundary_receipt(&ConsensusClaimBoundaryInput {
            group_ref: manifest.manifest_ref.clone(),
            claim: "byzantine-tolerance".to_string(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("claim boundary");
        assert_eq!(claim.decision, RAFT_DECISION_DENY);
        assert!(claim.diagnostics.join(";").contains("Byzantine"));
        assert_eq!(crate::ledger::artifact_kind(&claim.value), "consensus-non-claim-receipt");
    }
