
    #[test]
    fn boundary_codec_report_denies_malformed_canonical_records_before_side_effects() {
        // r[verify molten.preserves_boundary_codegen.fixture_corpus]
        let spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let malformed = boundary_fixture_with_field(spec, SCHEMA_FIELD_INDEX, super::string("unsupported.schema.v0"));
        let bytes = super::canonical_bytes(&malformed).expect("canonical malformed fixture bytes");
        let report = super::validate_boundary_bytes(&bytes, spec).expect("canonical malformed report");
        assert_eq!(report.decision, "deny");
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unsupported schema")));

        let mut non_canonical = bytes.clone();
        non_canonical.push(TRAILING_SENTINEL_BYTE);
        assert!(super::validate_boundary_bytes(&non_canonical, spec).is_err());
    }

    #[test]
    fn boundary_schema_adapter_denies_malformed_records_for_all_adopted_families() {
        // r[verify molten.preserves_schema_boundaries.schema_denials]
        for spec in boundary_schema_specs() {
            let fixture = boundary_schema_fixture(spec);
            let fields = boundary_fixture_fields(&fixture, spec);
            let wrong_label = super::record("wrong-label", fields.clone());
            assert!(super::validate_boundary_schema(&wrong_label, spec).is_err());

            let mut missing_fields = fields.clone();
            missing_fields.pop();
            let missing = super::record(spec.record_label, missing_fields);
            assert!(super::validate_boundary_schema(&missing, spec).is_err());

            let wrong_schema_type = boundary_fixture_with_field(spec, SCHEMA_FIELD_INDEX, super::u64_value(1));
            assert!(super::validate_boundary_schema(&wrong_schema_type, spec).is_err());

            let wrong_version = boundary_fixture_with_field(
                spec,
                SCHEMA_FIELD_INDEX,
                super::string("unsupported.schema.v0"),
            );
            assert!(super::validate_boundary_schema(&wrong_version, spec).is_err());

            let mut extra_fields = fields.clone();
            extra_fields.push(super::record("extra-critical", vec![super::string("deny")]));
            let extra = super::record(spec.record_label, extra_fields);
            assert!(super::validate_boundary_schema(&extra, spec).is_err());

            let checks_index = boundary_field_index(spec, |field| {
                matches!(field.kind, super::BoundaryFieldKind::ChecksRecord)
            });
            let malformed_checks = boundary_fixture_with_field(spec, checks_index, super::record(
                "checks",
                vec![super::sequence(vec![super::record("check", vec![super::string("missing-status")])])],
            ));
            assert!(super::validate_boundary_schema(&malformed_checks, spec).is_err());

            let ref_index = boundary_field_index(spec, is_ref_bearing_field);
            let malformed_ref = boundary_fixture_with_field(spec, ref_index, malformed_ref_field(&spec.fields[ref_index]));
            assert!(super::validate_boundary_schema(&malformed_ref, spec).is_err());
        }
    }

    #[test]
    fn boundary_schema_ref_binds_field_labels_kinds_and_constraints() {
        const TEST_SCHEMA_FIELD_COUNT: usize = 2;
        const BASE_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::StringRecord },
        ];
        const LABEL_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload-renamed", kind: super::BoundaryFieldKind::StringRecord },
        ];
        const KIND_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::RefRecord },
        ];
        const CONSTRAINT_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::NonEmptyStringRecord },
        ];
        let base = test_schema_spec("contract-ref-base", &BASE_FIELDS);
        let label_drift = test_schema_spec("contract-ref-base", &LABEL_DRIFT_FIELDS);
        let kind_drift = test_schema_spec("contract-ref-base", &KIND_DRIFT_FIELDS);
        let constraint_drift = test_schema_spec("contract-ref-base", &CONSTRAINT_DRIFT_FIELDS);
        let base_ref = super::boundary_schema_ref(&base).expect("base schema ref");
        assert_ne!(base_ref, super::boundary_schema_ref(&label_drift).expect("label drift ref"));
        assert_ne!(base_ref, super::boundary_schema_ref(&kind_drift).expect("kind drift ref"));
        assert_ne!(base_ref, super::boundary_schema_ref(&constraint_drift).expect("constraint drift ref"));
        let artifact_text = super::to_text(&super::boundary_schema_artifact_value(&base).expect("artifact"))
            .expect("artifact text");
        assert!(artifact_text.contains("fields"));
        assert!(artifact_text.contains("constraints"));
    }

    #[test]
    fn boundary_validation_reports_stale_claimed_schema_ref() {
        let spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let stale = boundary_test_ref("old-schema-ref");
        let error = super::validate_boundary_claimed_schema_ref(spec, &stale)
            .expect_err("stale schema ref denies");
        assert!(error.to_string().contains(spec.family));
        assert!(error.to_string().contains("stale schema ref"));
    }

    #[test]
    fn boundary_field_contracts_reject_invalid_domains_and_duplicates() {
        // r[verify molten.preserves_boundary_field_contracts.field_contract_denials]
        let plugin_spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let decision_index = boundary_field_index(plugin_spec, |field| field.label == "decision");
        let bad_decision = boundary_fixture_with_field(
            plugin_spec,
            decision_index,
            super::record("decision", vec![super::string("maybe")]),
        );
        assert!(super::validate_boundary_schema(&bad_decision, plugin_spec).is_err());

        let extension_spec = &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA;
        let policy_index = boundary_field_index(extension_spec, |field| field.label == "policy");
        let empty_policy = boundary_fixture_with_field(
            extension_spec,
            policy_index,
            super::record("policy", vec![super::sequence(Vec::new())]),
        );
        assert!(super::validate_boundary_schema(&empty_policy, extension_spec).is_err());

        let retention_spec = &super::RETENTION_RECEIPT_BOUNDARY_SCHEMA;
        let pins_index = boundary_field_index(retention_spec, |field| field.label == "pins");
        let duplicated_ref = boundary_test_ref("duplicate-pin");
        let duplicate_pins = boundary_fixture_with_field(
            retention_spec,
            pins_index,
            super::record("pins", vec![super::sequence(vec![
                super::string(&duplicated_ref),
                super::string(&duplicated_ref),
            ])]),
        );
        assert!(super::validate_boundary_schema(&duplicate_pins, retention_spec).is_err());

        let extension_spec = &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA;
        let hostcalls_index = boundary_field_index(extension_spec, |field| field.label == "hostcalls");
        let hostcalls = boundary_field_fixture(extension_spec, &extension_spec.fields[hostcalls_index]);
        let hostcall_record = hostcalls
            .collect_simple_record("hostcalls", Some(1))
            .expect("hostcalls record");
        let descriptors = hostcall_record[0].collect_sequence().expect("descriptor sequence");
        let first = super::value_to_iovalue(&descriptors[0]);
        let duplicate_hostcalls = boundary_fixture_with_field(
            extension_spec,
            hostcalls_index,
            super::record("hostcalls", vec![super::sequence(vec![first.clone(), first])]),
        );
        assert!(super::validate_boundary_schema(&duplicate_hostcalls, extension_spec).is_err());
    }

    fn test_schema_spec(
        family: &'static str,
        fields: &'static [super::BoundaryFieldSpec],
    ) -> super::BoundarySchemaSpec {
        super::BoundarySchemaSpec {
            family,
            version: "v1",
            record_label: "test-boundary-v1",
            schema_id: "molten.test-boundary.v1",
            fields,
        }
    }

    const SCHEMA_FIELD_INDEX: usize = 0;

    #[test]
    fn structural_scan_detects_nested_markers_without_scanning_rendered_strings() {
        // r[verify molten.preserves_value_inspection.structural_scan]
        // r[verify molten.preserves_value_inspection.marker_detection]
        let nested = super::record("outer", vec![
            super::sequence(vec![super::record("secret", vec![super::string("payload")])]),
            super::string("<credential \"looks rendered but is inert\">"),
        ]);
        let marker = super::find_sensitive_structural_marker(&nested)
            .expect("scan nested")
            .expect("sensitive marker");
        assert_eq!(marker.kind, super::StructuralTokenKind::RecordLabel);
        assert_eq!(marker.token, "secret");

        let inert = super::record("outer", vec![super::string("<secret \"looks rendered but is inert\">")]);
        assert!(
            super::find_sensitive_structural_marker(&inert)
                .expect("scan inert")
                .is_none(),
            "rendered-looking strings are diagnostics, not structural markers"
        );
    }

    #[test]
    fn structural_scan_finds_nested_content_refs() {
        // r[verify molten.preserves_value_inspection.ref_retention]
        let target = super::content_ref_from_bytes(b"structural-content-ref-target");
        let value = super::record("outer", vec![
            super::sequence(vec![super::record("metadata", vec![super::string(&target)])]),
            super::string("blake3:not-a-valid-ref"),
        ]);
        let found = super::find_structural_content_ref(&value, &target)
            .expect("scan content ref")
            .expect("content ref match");
        assert_eq!(found.kind, super::StructuralTokenKind::ContentRef);
        assert_eq!(found.token, target);
    }

    #[test]
    fn structural_scan_reports_bounds() {
        let value = super::record("outer", vec![super::record("inner", Vec::new())]);
        let error = super::find_structural_match_with_limits(
            &value,
            super::StructuralInspectionScope::all(),
            super::StructuralInspectionLimits {
                max_nodes: 1,
                max_depth: super::DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH,
            },
            |_, _| false,
        )
        .expect_err("bounded scan should fail");
        assert!(error.to_string().contains("structural Preserves scan exceeded"));
    }

    fn scan_nested_symbol(
        text: &str,
        limits: super::StructuralInspectionLimits,
    ) -> super::Result<Option<super::StructuralMatch>> {
        let value = super::parse_text(text)?;
        super::find_structural_match_with_limits(&value, super::StructuralInspectionScope::all(), limits, |kind, token| {
            kind == super::StructuralTokenKind::Symbol && token == "needle"
        })
    }

    #[test]
    fn structural_scan_reports_the_first_preorder_match_path() {
        // r[verify molten.preserves_value_inspection.structural_scan]
        let limits = super::StructuralInspectionLimits::default();
        let nested = scan_nested_symbol("<outer [\"a\" <m needle>] {k: needle}>", limits).expect("scan nested value");
        assert_eq!(nested.expect("record field match").path, vec!["$", "field[0]", "item[1]", "field[0]"]);
        let value = scan_nested_symbol("[{k: needle}]", limits).expect("scan dictionary value");
        assert_eq!(value.expect("dictionary value match").path, vec!["$", "item[0]", "entry[0].value"]);
        let key = scan_nested_symbol("[{needle: 1}]", limits).expect("scan dictionary key");
        assert_eq!(key.expect("dictionary key match").path, vec!["$", "item[0]", "entry[0].key"]);
    }

    #[test]
    fn structural_scan_admits_the_exact_depth_and_node_bounds_and_denies_one_past() {
        const NESTED: &str = "[[[other]]]";
        const NESTED_DEPTH: usize = 4;
        const NESTED_NODES: usize = 4;
        let exact = super::StructuralInspectionLimits {
            max_nodes: NESTED_NODES,
            max_depth: NESTED_DEPTH,
        };
        assert!(scan_nested_symbol(NESTED, exact).expect("exact bounds admit the scan").is_none());
        let shallow = super::StructuralInspectionLimits {
            max_depth: NESTED_DEPTH - 1,
            ..exact
        };
        let depth_error = scan_nested_symbol(NESTED, shallow).expect_err("one level past the depth bound denies");
        assert!(depth_error.to_string().contains("exceeded depth 3"));
        let small = super::StructuralInspectionLimits {
            max_nodes: NESTED_NODES - 1,
            ..exact
        };
        let node_error = scan_nested_symbol(NESTED, small).expect_err("one node past the node bound denies");
        assert!(node_error.to_string().contains("exceeded 3 nodes"));
    }

    #[test]
    fn structural_scan_bounds_wide_containers_by_the_node_budget() {
        const WIDE_BUDGET: usize = 3;
        let limits = super::StructuralInspectionLimits {
            max_nodes: WIDE_BUDGET,
            max_depth: super::DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH,
        };
        let early = scan_nested_symbol("[other needle other other other]", limits).expect("match inside the budget");
        assert_eq!(early.expect("early match").path, vec!["$", "item[1]"]);
        let late = scan_nested_symbol("[other other other needle]", limits).expect_err("match past the budget denies");
        assert!(late.to_string().contains("exceeded 3 nodes"));
    }
