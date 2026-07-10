
    #[test]
    fn iroh_experiment_adoption_fixture_covers_pass_and_locator_only_denial() {
        let fixture = iroh_experiment_adoption_fixture().expect("iroh adoption fixture");
        assert_eq!(fixture.decision, PASS_DECISION);
        assert!(fixture.diagnostics.is_empty());
        let text = crate::preserves_rail::to_text(&fixture.receipt_value).expect("fixture text");
        assert!(text.contains("locator-hint-only-denial-covered"));
        assert!(text.contains("deterministic-traversal-covered"));
        assert!(text.contains("remote-bytes-verified-before-admission"));
    }
