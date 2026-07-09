
    #[test]
    fn effect_manifest_profiles_admit_declared_operations() {
        let (manifest, profile, request) = manifest_profile_and_request("storage.read", "read");
        let parsed_manifest = parse_effect_manifest(&manifest).expect("parse manifest");
        let declared = parsed_manifest.declared_effects.first().expect("declared effect");
        assert_eq!(declared.resource_class, "hostcall");
        assert!(declared.capability_refs.iter().any(|reference| reference == &fake_ref("capability")));
        let parsed_profile = parse_handler_profile(&profile).expect("parse profile");
        let admission = admit_handler_profile_for_manifest(HandlerProfileAdmissionInput {
            manifest: manifest.clone(),
            handler_profile: profile.clone(),
            supported_effects: parsed_manifest.declared_effects.clone(),
            determinism_class: EFFECT_DETERMINISM_DETERMINISTIC.to_string(),
            replay_class: EFFECT_REPLAY_CLASS_RECORDED.to_string(),
            current_policy_ref: parsed_profile.policy_ref.clone(),
            current_capability_context_ref: parsed_profile.capability_context_ref.clone(),
            evidence_refs: vec![fake_ref("profile-admission-evidence")],
        })
        .expect("handler profile admission");
        assert_eq!(admission.decision, "pass");
        assert_eq!(admission.manifest_ref, parsed_manifest.manifest_ref);
        assert_eq!(admission.handler_profile_ref, parsed_profile.profile_ref);
        assert!(admission.checks.iter().any(|check| check == "operation-schema-bound"));
        let parsed_admission = parse_handler_profile_admission_receipt(&admission.value).expect("parse admission");
        assert_eq!(parsed_admission.receipt_ref, admission.receipt_ref);

        let admitted = admit_effect_request(&manifest, &profile, &request, &[fake_ref("request-admission-evidence")])
            .expect("admitted request");
        assert_eq!(admitted.decision, "pass");
    }

    #[test]
    fn effect_requests_deny_missing_declared_capabilities() {
        let (manifest, profile, _) = manifest_profile_and_request("storage.write", "write");
        let artifact_ref = parse_effect_manifest(&manifest).expect("manifest").artifact_ref;
        let request = effect_request_value(&EffectRequestInput {
            artifact_ref,
            effect_id: "storage.write".to_string(),
            operation: "write".to_string(),
            handler_profile: HANDLER_PROFILE_LOCAL.to_string(),
            input_ref: fake_ref("input"),
            capability_refs: Vec::new(),
            evidence_refs: vec![fake_ref("request-evidence")],
        })
        .expect("request without required capability");
        let denied = admit_effect_request(&manifest, &profile, &request, &[fake_ref("request-admission-evidence")])
            .expect("denied request");
        assert_eq!(denied.decision, "deny");
        assert!(denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("missing required capability")));
    }

    #[test]
    fn handler_profile_admission_denies_stale_context_and_schema_mismatch() {
        let (manifest, profile, _) = manifest_profile_and_request("dataspace.assert", "assert");
        let parsed_manifest = parse_effect_manifest(&manifest).expect("parse manifest");
        let parsed_profile = parse_handler_profile(&profile).expect("parse profile");
        let stale = admit_handler_profile_for_manifest(HandlerProfileAdmissionInput {
            manifest: manifest.clone(),
            handler_profile: profile.clone(),
            supported_effects: parsed_manifest.declared_effects.clone(),
            determinism_class: EFFECT_DETERMINISM_DETERMINISTIC.to_string(),
            replay_class: EFFECT_REPLAY_CLASS_RECORDED.to_string(),
            current_policy_ref: fake_ref("stale-policy"),
            current_capability_context_ref: parsed_profile.capability_context_ref.clone(),
            evidence_refs: vec![fake_ref("profile-admission-evidence")],
        })
        .expect("stale profile admission");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));

        let mut supported = parsed_manifest.declared_effects.clone();
        supported.first_mut().expect("supported effect").input_schema_ref = fake_ref("wrong-input-schema");
        let mismatched = admit_handler_profile_for_manifest(HandlerProfileAdmissionInput {
            manifest,
            handler_profile: profile,
            supported_effects: supported,
            determinism_class: EFFECT_DETERMINISM_DETERMINISTIC.to_string(),
            replay_class: EFFECT_REPLAY_CLASS_RECORDED.to_string(),
            current_policy_ref: parsed_profile.policy_ref,
            current_capability_context_ref: parsed_profile.capability_context_ref,
            evidence_refs: vec![fake_ref("profile-admission-evidence")],
        })
        .expect("schema mismatch admission");
        assert_eq!(mismatched.decision, "deny");
        assert!(mismatched
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("schema/resource/capability mismatch")));
    }

    #[test]
    fn effect_profile_replay_binding_denies_profile_drift_without_compatibility() {
        let (manifest, profile, _) = manifest_profile_and_request("job.run", "run");
        let parsed_manifest = parse_effect_manifest(&manifest).expect("parse manifest");
        let parsed_profile = parse_handler_profile(&profile).expect("parse profile");
        let admission = admit_handler_profile_for_manifest(HandlerProfileAdmissionInput {
            manifest,
            handler_profile: profile,
            supported_effects: parsed_manifest.declared_effects.clone(),
            determinism_class: EFFECT_DETERMINISM_DETERMINISTIC.to_string(),
            replay_class: EFFECT_REPLAY_CLASS_RECORDED.to_string(),
            current_policy_ref: parsed_profile.policy_ref.clone(),
            current_capability_context_ref: parsed_profile.capability_context_ref.clone(),
            evidence_refs: vec![fake_ref("profile-admission-evidence")],
        })
        .expect("profile admission");
        let binding = bind_effect_profile_replay_evidence(EffectProfileReplayBindingInput {
            integration_kind: EFFECT_PROFILE_INTEGRATION_EVAL_CACHE.to_string(),
            subject_ref: fake_ref("cache-entry"),
            effect_manifest_ref: parsed_manifest.manifest_ref.clone(),
            handler_profile_ref: parsed_profile.profile_ref.clone(),
            profile_admission_ref: admission.receipt_ref.clone(),
            expected_manifest_ref: Some(parsed_manifest.manifest_ref.clone()),
            expected_handler_profile_ref: Some(parsed_profile.profile_ref.clone()),
            compatibility_ref: None,
            evidence_refs: vec![fake_ref("cache-binding-evidence")],
        })
        .expect("profile binding");
        assert_eq!(binding.decision, "pass");
        let parsed = parse_effect_profile_replay_binding(&binding.value).expect("parse profile binding");
        assert_eq!(parsed.binding_ref, binding.binding_ref);

        let drift = bind_effect_profile_replay_evidence(EffectProfileReplayBindingInput {
            integration_kind: EFFECT_PROFILE_INTEGRATION_EVAL_CACHE.to_string(),
            subject_ref: fake_ref("cache-entry"),
            effect_manifest_ref: parsed_manifest.manifest_ref,
            handler_profile_ref: fake_ref("different-handler-profile"),
            profile_admission_ref: admission.receipt_ref,
            expected_manifest_ref: None,
            expected_handler_profile_ref: Some(parsed_profile.profile_ref),
            compatibility_ref: None,
            evidence_refs: vec![fake_ref("cache-binding-evidence")],
        })
        .expect("profile drift binding");
        assert_eq!(drift.decision, "deny");
        assert!(drift
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("handler profile ref changed")));
    }

    #[test]
    fn unison_effect_compatibility_claims_are_denied() {
        let clean = unison_effect_compatibility_claim_diagnostics("Unison abilities are prior art only");
        assert!(clean.is_empty());
        let denied = unison_effect_compatibility_claim_diagnostics("this artifact claims unison-compatible effects");
        assert!(denied.iter().any(|diagnostic| diagnostic.contains("prior art only")));
    }
