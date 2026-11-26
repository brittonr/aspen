
    fn embedded_layers(inner: preserves::IOValue, layers: usize) -> preserves::IOValue {
        (0..layers).fold(inner, |value, _| preserves::IOValue::embedded(value))
    }

    #[test]
    fn structural_scan_descends_into_embedded_values() {
        // r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
        let target = super::content_ref_from_bytes(b"embedded-content-ref-target");
        let value = super::record("outer", vec![
            embedded_layers(super::record("secret", vec![super::string("payload")]), 1),
            embedded_layers(super::record("host-path", vec![super::string("/etc")]), 1),
            embedded_layers(super::record("metadata", vec![super::string(&target)]), 1),
        ]);
        let decoded = super::strict_canonical_decode(&super::canonical_bytes(&value).expect("canonical bytes"))
            .expect("strict decode admits canonical embedded values");
        assert_eq!(decoded.value, value);

        let secret = super::find_sensitive_structural_marker(&decoded.value)
            .expect("scan sensitive")
            .expect("embedded secret is detected");
        assert_eq!(secret.token, "secret");
        assert_eq!(secret.path, vec!["$", "field[0]", "embedded"]);
        let ambient = super::find_ambient_job_token(&decoded.value)
            .expect("scan ambient")
            .expect("embedded host path is detected");
        assert_eq!(ambient.token, "host-path");
        assert_eq!(ambient.path, vec!["$", "field[1]", "embedded"]);
        assert!(super::contains_structural_content_ref(&decoded.value, &target).expect("scan content ref"));

        let inert = super::record("outer", vec![embedded_layers(super::record("public", Vec::new()), 1)]);
        assert!(super::find_sensitive_structural_marker(&inert).expect("scan inert").is_none());
        assert!(super::find_ambient_job_token(&inert).expect("scan inert ambient").is_none());
    }

    #[test]
    fn structural_scan_counts_embedded_layers_against_the_depth_bound() {
        // r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
        let limits = super::StructuralInspectionLimits {
            max_nodes: super::DEFAULT_STRUCTURAL_SCAN_MAX_NODES,
            max_depth: 3,
        };
        let needle = || super::record("m", vec![preserves::IOValue::symbol("needle")]);
        let scan = |value: &preserves::IOValue| {
            super::find_structural_match_with_limits(value, super::StructuralInspectionScope::all(), limits, |kind, token| {
                kind == super::StructuralTokenKind::Symbol && token == "needle"
            })
        };
        assert!(scan(&embedded_layers(needle(), 1)).expect("within depth").is_some());
        let error = scan(&embedded_layers(needle(), 3)).expect_err("embedded layers count toward depth");
        assert!(error.to_string().contains("exceeded depth 3"));
    }

    #[hegel::test(test_cases = 32)]
    fn hegel_embedded_markers_and_refs_never_evade_structural_scans(tc: hegel::TestCase) {
        // r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
        let layers = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(6));
        let sensitive_index =
            tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(super::SENSITIVE_STRUCTURAL_MARKERS.len() - 1));
        let ambient_index =
            tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(super::AMBIENT_JOB_TOKENS.len() - 1));
        let salt = tc.draw(hegel::generators::binary().max_size(32));
        let sensitive = super::SENSITIVE_STRUCTURAL_MARKERS[sensitive_index];
        let ambient = super::AMBIENT_JOB_TOKENS[ambient_index];
        let target = super::content_ref_from_bytes(&salt);

        let wrapped_sensitive = embedded_layers(super::record(sensitive, Vec::new()), layers);
        let wrapped_ambient = embedded_layers(super::record(ambient, Vec::new()), layers);
        let wrapped_ref = embedded_layers(super::string(&target), layers);
        let value = super::sequence(vec![wrapped_sensitive, wrapped_ambient, wrapped_ref]);

        assert_eq!(
            super::find_sensitive_structural_marker(&value).expect("scan sensitive").map(|found| found.token),
            Some(sensitive.to_string())
        );
        assert!(super::find_ambient_job_token(&value).expect("scan ambient").is_some());
        assert!(super::contains_structural_content_ref(&value, &target).expect("scan ref"));
    }
