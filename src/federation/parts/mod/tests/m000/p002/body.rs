
    #[test]
    fn iroh_locator_records_are_hint_only_and_admission_requires_verification() {
        let subject_ref = ref_for("locator-subject");
        let evidence_ref = ref_for("locator-evidence");
        let announcement = locator_announcement(&LocatorAnnouncementInput {
            peer_ref: "peer:locator-source",
            signer: "peer:locator-source",
            subject_ref: &subject_ref,
            availability: LOCATOR_COMPLETE,
            freshness: LOCATOR_FRESH,
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("locator announcement");
        assert_eq!(announcement.decision, "pass");
        assert!(!announcement.can_import);
        let text = crate::preserves_rail::to_text(&announcement.value).expect("locator text");
        assert!(text.contains("hint-only"));

        let locator_only = admit_locator_import(&LocatorAdmissionInput {
            locator_refs: std::slice::from_ref(&announcement.evidence_ref),
            fetched_ref: None,
            verification_refs: &[],
            admission_refs: &[],
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
        })
        .expect("locator-only admission");
        assert_eq!(locator_only.decision, "deny");
        assert!(locator_only.diagnostics.iter().any(|diagnostic| diagnostic.contains("hint-only")));

        let verification_ref = ref_for("hash-verification");
        let admission_ref = ref_for("local-admission");
        let authority_ref = ref_for("authority");
        let policy_ref = ref_for("policy");
        let resource_ref = ref_for("resource");
        let admitted = admit_locator_import(&LocatorAdmissionInput {
            locator_refs: std::slice::from_ref(&announcement.evidence_ref),
            fetched_ref: Some(&subject_ref),
            verification_refs: std::slice::from_ref(&verification_ref),
            admission_refs: std::slice::from_ref(&admission_ref),
            authority_refs: std::slice::from_ref(&authority_ref),
            policy_refs: std::slice::from_ref(&policy_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
        })
        .expect("admitted locator boundary");
        assert_eq!(admitted.decision, "pass");
    }

    #[test]
    fn pkarr_locator_pointer_is_optional_and_stale_denies() {
        let key_ref = ref_for("pkarr-key");
        let subject_ref = ref_for("pkarr-subject");
        let signature_ref = ref_for("pkarr-signature");
        let fresh = pkarr_locator_result(&PkarrLocatorInput {
            key_ref: &key_ref,
            signer: "peer:pkarr",
            resolved_subject_ref: &subject_ref,
            freshness: LOCATOR_FRESH,
            signature_ref: &signature_ref,
        })
        .expect("fresh pkarr");
        assert_eq!(fresh.decision, "pass");
        assert!(!fresh.can_import);

        let stale = pkarr_locator_result(&PkarrLocatorInput {
            freshness: LOCATOR_STALE,
            ..PkarrLocatorInput {
                key_ref: &key_ref,
                signer: "peer:pkarr",
                resolved_subject_ref: &subject_ref,
                freshness: LOCATOR_FRESH,
                signature_ref: &signature_ref,
            }
        })
        .expect("stale pkarr");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));
    }
