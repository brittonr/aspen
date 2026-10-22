    // r[verify molten.runtime_spine.record_pattern_subset]

    const SERVICE_LABEL: &str = "service-ready";
    const NESTED_LABEL: &str = "service-endpoint";

    fn record_value(label: &'static str, fields: Vec<IoValue>) -> RuntimeValue {
        RuntimeValue::new(crate::preserves_rail::record(label, fields)).expect("record value")
    }

    fn string_value(value: &str) -> IoValue {
        crate::preserves_rail::string(value)
    }

    fn service_ready(service: &str, extra: Option<&str>) -> RuntimeValue {
        let mut fields = vec![string_value(service)];
        if let Some(extra) = extra {
            fields.push(string_value(extra));
        }
        record_value(SERVICE_LABEL, fields)
    }

    fn service_pattern(service: &str) -> RuntimePattern {
        RuntimePattern::record(
            SERVICE_LABEL,
            vec![RecordPatternEntry {
                position: 0,
                pattern: Box::new(RuntimePattern::exact(
                    RuntimeValue::new(string_value(service)).expect("service value"),
                )),
            }],
        )
    }

    #[test]
    fn a_record_with_an_extra_field_matches_and_binds_in_declared_order() {
        let pattern = RuntimePattern::record(
            SERVICE_LABEL,
            vec![
                RecordPatternEntry {
                    position: 0,
                    pattern: Box::new(RuntimePattern::wildcard("service")),
                },
                RecordPatternEntry {
                    position: 1,
                    pattern: Box::new(RuntimePattern::wildcard("region")),
                },
            ],
        );
        let candidate = service_ready("db", Some("zone-a"));
        let (matched, bindings) = pattern.matches_value(&candidate).expect("record pattern match");
        assert!(matched, "an extra field keeps the match");
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].0, "service");
        assert_eq!(bindings[1].0, "region");
    }

    #[test]
    fn a_nested_record_binds_its_sub_portion() {
        let nested = RuntimePattern::record(
            NESTED_LABEL,
            vec![RecordPatternEntry {
                position: 0,
                pattern: Box::new(RuntimePattern::wildcard("host")),
            }],
        );
        let pattern = RuntimePattern::record(
            SERVICE_LABEL,
            vec![RecordPatternEntry {
                position: 0,
                pattern: Box::new(nested),
            }],
        );
        let endpoint = record_value(NESTED_LABEL, vec![string_value("node-a")]);
        let candidate = record_value(SERVICE_LABEL, vec![endpoint.as_iovalue().clone()]);
        let (matched, bindings) = pattern.matches_value(&candidate).expect("nested record pattern match");
        assert!(matched, "the nested record matches");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].0, "host");
    }

    #[test]
    fn a_missing_declared_position_fails_without_bindings() {
        let pattern = service_pattern("db");
        let candidate = service_ready("db", None);
        let (matched, bindings) = pattern.matches_value(&candidate).expect("record pattern match");
        assert!(matched, "the declared position is present");
        assert!(bindings.is_empty(), "an exact sub-pattern binds nothing");

        let missing = RuntimePattern::record(
            SERVICE_LABEL,
            vec![RecordPatternEntry {
                position: 3,
                pattern: Box::new(RuntimePattern::wildcard("absent")),
            }],
        );
        let (matched, bindings) = missing.matches_value(&candidate).expect("record pattern match");
        assert!(!matched, "a missing declared position fails the match");
        assert!(bindings.is_empty(), "a failed match leaves no binding");
    }

    #[test]
    fn a_wrong_type_at_a_declared_position_fails_without_bindings() {
        let pattern = service_pattern("db");
        let candidate = record_value(SERVICE_LABEL, vec![crate::preserves_rail::u64_value(7)]);
        let (matched, bindings) = pattern.matches_value(&candidate).expect("record pattern match");
        assert!(!matched, "a mistyped declared position fails the match");
        assert!(bindings.is_empty(), "a failed match leaves no binding");
    }

    #[test]
    fn a_mismatched_nested_sub_pattern_leaves_no_binding() {
        let nested = RuntimePattern::record(
            NESTED_LABEL,
            vec![RecordPatternEntry {
                position: 0,
                pattern: Box::new(RuntimePattern::wildcard("host")),
            }],
        );
        let pattern = RuntimePattern::record(
            SERVICE_LABEL,
            vec![
                RecordPatternEntry {
                    position: 0,
                    pattern: Box::new(RuntimePattern::wildcard("service")),
                },
                RecordPatternEntry {
                    position: 1,
                    pattern: Box::new(nested),
                },
            ],
        );
        let candidate = record_value(
            SERVICE_LABEL,
            vec![
                string_value("db"),
                record_value("other-label", vec![string_value("node-a")]).as_iovalue().clone(),
            ],
        );
        let (matched, bindings) = pattern.matches_value(&candidate).expect("record pattern match");
        assert!(!matched, "the mismatched nested sub-pattern fails the match");
        assert!(bindings.is_empty(), "the speculative binding is rolled back");
    }

    #[test]
    fn an_over_bound_pattern_denies_at_admission() {
        let entries = (0..17)
            .map(|position| RecordPatternEntry {
                position,
                pattern: Box::new(RuntimePattern::wildcard("binding")),
            })
            .collect();
        let pattern = RuntimePattern::record(SERVICE_LABEL, entries);
        let error = pattern.validate().expect_err("an over-bound pattern denies");
        assert!(error.to_string().contains("more than"));

        let mut nested = RuntimePattern::wildcard("leaf");
        for _ in 0..5 {
            nested = RuntimePattern::record(
                SERVICE_LABEL,
                vec![RecordPatternEntry {
                    position: 0,
                    pattern: Box::new(nested),
                }],
            );
        }
        let error = nested.validate().expect_err("an over-deep pattern denies");
        assert!(error.to_string().contains("nests deeper"));
    }

    #[test]
    fn an_undeclared_form_denies_before_routing() {
        let error = RuntimePattern::from_ast_value(&crate::preserves_rail::record(
            "runtime-pattern-unknown-v1",
            vec![string_value("service")],
        ))
        .expect_err("an undeclared pattern form denies");
        assert!(error.to_string().contains("unsupported Preserves routing pattern AST form"));
    }

    #[test]
    fn the_record_pattern_round_trips_through_its_ast_form() {
        let pattern = service_pattern("db");
        let value = pattern.to_value();
        let decoded = RuntimePattern::from_ast_value(&value).expect("record pattern ast");
        assert_eq!(decoded, pattern);
        assert!(pattern.validate().is_ok());
    }

    #[test]
    fn the_identity_of_a_matched_record_is_unchanged() {
        let candidate = service_ready("db", Some("zone-a"));
        let before = candidate.value_ref().to_string();
        let pattern = service_pattern("db");
        let (matched, _) = pattern.matches_value(&candidate).expect("record pattern match");
        assert!(matched, "the record matches the declared pattern");
        assert_eq!(candidate.value_ref(), before, "the canonical value ref is unchanged");
    }
