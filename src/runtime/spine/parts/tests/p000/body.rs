    fn authority_input(resource: &str, ability: &str, policy_allows: bool) -> super::BasaltUcanAuthorityInput {
        super::BasaltUcanAuthorityInput {
            contract_id: "contract:send".to_string(),
            resource: resource.to_string(),
            ability: ability.to_string(),
            holder_ref: test_ref("holder"),
            session_ref: test_ref("session"),
            context_ref: test_ref("context"),
            request_ref: test_ref("request"),
            basalt_policy_ref: test_ref("policy"),
            basalt_policy_source_ref: test_ref("policy-source"),
            basalt_policy_export_ref: test_ref("policy-export"),
            proofset_ref: test_ref("proofset"),
            ucan_verification_receipt_refs: vec![test_ref("ucan-verification")],
            verified_grants: vec![super::VerifiedBasaltGrant {
                grant_ref: test_ref("grant"),
                verification_receipt_ref: test_ref("ucan-verification"),
                holder_ref: test_ref("holder"),
                session_ref: test_ref("session"),
                context_ref: test_ref("context"),
                resource: resource.to_string(),
                ability: ability.to_string(),
                scope: "topic".to_string(),
            }],
            policy_allows,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn envelope(capability: &str) -> crate::runtime::Envelope {
        crate::runtime::Envelope::new(crate::runtime::EnvelopeInput {
            sender: crate::runtime::ActorId::parse("actor:policy").expect("sender"),
            subject: crate::runtime::RuntimeValue::string("policy.subject").expect("subject"),
            body: crate::runtime::RuntimeValue::string("body").expect("body"),
            blob_refs: Vec::new(),
            capabilities: vec![crate::runtime::Capability::parse(capability).expect("capability")],
            evidence_refs: Vec::new(),
        })
        .expect("envelope")
    }

    #[test]
    fn contract_selection_records_static_nickel_and_reviewed_steel() {
        let nickel = super::nickel_contract_decision("static-policy", b"{ allowed = true }");
        let steel = super::steel_contract_decision("dynamic-review", b"(lambda (x) x)");
        assert_eq!(nickel.backend, super::ContractBackend::NickelStatic);
        assert_eq!(steel.backend, super::ContractBackend::SteelReviewed);
        assert_ne!(nickel.contract_ref, steel.contract_ref);
    }

    #[test]
    fn basalt_request_requires_verified_authority() {
        let request = super::BasaltRequest {
            contract_id: "contract:send".to_string(),
            resource: "subject:ready".to_string(),
            ability: "send".to_string(),
            ucan_ref: crate::preserves_rail::content_ref_from_bytes(b"ucan"),
        };
        let error =
            super::evaluate_basalt_request(&request, "subject:ready", "send").expect_err("bare UCAN ref denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
        assert!(error.to_string().contains("bare ucan_ref"));
        let error =
            super::evaluate_basalt_request(&request, "subject:other", "send").expect_err("wrong resource denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
    }

    #[test]
    fn basalt_ucan_authority_admits_verified_grant_and_denies_mismatches() {
        let input = authority_input("subject:ready", "send", true);
        let receipt = super::evaluate_basalt_ucan_authority(&input).expect("authority receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(receipt.diagnostics.is_empty());
        assert_eq!(receipt.receipt_ref, crate::preserves_rail::canonical_hash(&receipt.value).expect("hash"));

        let mut denied = input.clone();
        denied.policy_allows = false;
        denied.verified_grants[0].resource = "subject:other".to_string();
        let receipt = super::evaluate_basalt_ucan_authority(&denied).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("Basalt policy denied")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource mismatch")));
    }

    #[test]
    fn policy_gate_records_pass_and_deny_receipts() {
        let pass =
            super::policy_gate_receipt(&envelope("send:policy.subject"), "send:policy.subject").expect("pass receipt");
        let deny = super::policy_gate_receipt(&envelope("send:other"), "send:policy.subject").expect("deny receipt");
        assert_eq!(pass.decision, "pass");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics[0].contains("missing capability"));
    }

    #[test]
    fn cairn_valence_and_receipt_index_refs_are_canonical() {
        let reference = crate::preserves_rail::content_ref_from_bytes(b"receipt");
        super::validate_cairn_receipt_ref(&reference).expect("cairn receipt ref");
        let evidence = super::valence_evidence_ref(reference.clone(), "function-object").expect("valence evidence");
        assert_eq!(evidence.evidence_ref, reference);

        let mut index = super::ReceiptIndex::default();
        index.insert("turn:1", evidence.evidence_ref.clone()).expect("insert receipt");
        assert_eq!(index.get("turn:1"), Some(evidence.evidence_ref.as_str()));
    }

    #[test]
    fn integration_evidence_binds_config_route_remote_and_policy_refs() {
        let evidence = super::integration_evidence(b"config", b"local", b"remote", b"policy");
        assert_eq!(evidence.config_ref, crate::preserves_rail::content_ref_from_bytes(b"config"));
        assert_eq!(evidence.local_route_ref, crate::preserves_rail::content_ref_from_bytes(b"local"));
        assert_eq!(evidence.remote_bridge_ref, crate::preserves_rail::content_ref_from_bytes(b"remote"));
        assert_eq!(evidence.policy_ref, crate::preserves_rail::content_ref_from_bytes(b"policy"));
    }

    #[hegel::test(test_cases = 8)]
    fn hegel_envelope_policy_identity_is_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let capability = format!("send:policy.subject.{salt}");
        let envelope = envelope(&capability);
        let left = super::policy_gate_receipt(&envelope, &capability).expect("left");
        let right = super::policy_gate_receipt(&envelope, &capability).expect("right");
        assert_eq!(left.envelope_ref, right.envelope_ref);
        assert_eq!(left.decision, "pass");
    }
