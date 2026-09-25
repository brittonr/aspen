
    #[test]
    fn parser_toolkit_builds_and_parses_common_ref_shapes() {
        const MAX_REFS: u64 = 4;
        let first = super::content_ref_from_bytes(b"toolkit-first-ref");
        let second = super::content_ref_from_bytes(b"toolkit-second-ref");
        let record = super::record("refs", vec![super::refs_sequence(&[first.clone(), second.clone()])]);
        let refs = super::record_content_ref_strings(&preserves::Value::from(record), "refs", "toolkit refs", MAX_REFS)
            .expect("parse refs");
        assert_eq!(refs, vec![first, second]);

        let some = super::optional_ref_value(refs.first().map(String::as_str));
        assert_eq!(
            super::optional_content_ref_string(&preserves::Value::from(some), "optional ref")
                .expect("optional ref"),
            refs.first().cloned()
        );
    }

    #[test]
    fn parser_toolkit_rejects_malformed_shapes_and_checks() {
        // r[verify molten.preserves_rail_toolkit.parser_builders]
        // r[verify molten.preserves_rail_toolkit.check_sets]
        // r[verify molten.preserves_rail_toolkit.negative_shapes]
        const MAX_REFS: u64 = 4;
        let wrong_label = super::record("wrong", Vec::new());
        assert!(super::simple_record_fields(&wrong_label, "expected", 0).is_err());

        let wrong_arity = super::record("expected", vec![super::string("extra")]);
        assert!(super::simple_record_fields(&wrong_arity, "expected", 0).is_err());

        let wrong_type = preserves::Value::from(super::u64_value(1));
        assert!(super::required_string_field(&wrong_type, "string field").is_err());

        let invalid_ref_record = super::record("refs", vec![super::sequence(vec![super::string("blake3:not-valid")])]);
        assert!(
            super::record_content_ref_strings(
                &preserves::Value::from(invalid_ref_record),
                "refs",
                "toolkit refs",
                MAX_REFS,
            )
            .is_err()
        );

        let checks = super::checks_value(&[("shape", "pass")]);
        let parsed = super::parse_checks_record(&preserves::Value::from(checks), MAX_REFS, "toolkit")
            .expect("parse checks");
        assert!(super::require_checks_present(&parsed, &["missing"], "toolkit").is_err());

        let duplicate = super::checks_value(&[("shape", "pass"), ("shape", "pass")]);
        assert!(super::parse_checks_record(&preserves::Value::from(duplicate), MAX_REFS, "toolkit").is_err());

        let unsupported = super::checks_value(&[("shape", "unknown")]);
        assert!(super::parse_checks_record(&preserves::Value::from(unsupported), MAX_REFS, "toolkit").is_err());
    }

    #[test]
    fn content_ref_parser_rejects_non_canonical_shapes() {
        // r[verify molten.runtime_spine.canonical_content_refs.shape]
        // r[verify molten.runtime_spine.canonical_content_refs.filename_readback]
        // r[verify molten.runtime_spine.canonical_content_refs.scoped_aliases]
        // r[verify molten.runtime_spine.canonical_content_refs.negative_tests]
        let valid = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        super::validate_content_ref(valid).expect("valid ref");
        let parsed = super::ContentRef::parse(valid).expect("parsed ref");
        assert_eq!(parsed.as_str(), valid);
        assert_eq!(parsed.into_string(), valid);
        assert_eq!(
            super::content_ref_from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .expect("ref from hex"),
            valid
        );

        for invalid in [
            "",
            "blake3:",
            "blake3:fixture",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde/",
        ] {
            assert!(super::validate_content_ref(invalid).is_err(), "invalid ref accepted: {invalid}");
        }

        for invalid_hex in [
            "",
            "fixture",
            "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde/",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
        ] {
            assert!(
                super::content_ref_from_hex(invalid_hex).is_err(),
                "invalid filename digest accepted: {invalid_hex}"
            );
        }
    }

    #[test]
    fn typed_domain_newtypes_parse_format_and_reject_invalid_values() {
        let stable = super::StableId::parse("node:alpha-1").expect("stable id");
        assert_eq!(stable.as_str(), "node:alpha-1");
        assert_eq!(stable.clone().into_string(), "node:alpha-1");
        assert_eq!(super::SchemaId::parse("molten.schema.v1").expect("schema id").as_str(), "molten.schema.v1");
        assert_eq!(super::OperationId::parse("storage.read").expect("operation id").as_str(), "storage.read");
        assert_eq!(super::ProfileId::parse("production").expect("profile id").as_str(), "production");
        assert_eq!(super::Decision::parse("pass").expect("decision").as_str(), "pass");
        assert_eq!(super::CheckStatus::parse("diagnostic").expect("check status").as_str(), "diagnostic");
        assert_eq!(super::ReplayClass::parse("deterministic").expect("replay class").as_str(), "deterministic");

        assert!(super::StableId::parse("").is_err());
        assert!(super::StableId::parse("bad/id").is_err());
        assert!(super::OperationId::parse("Storage.read").is_err());
        assert!(super::Decision::parse("maybe").is_err());
        assert!(super::CheckStatus::parse("unknown").is_err());
        assert!(super::ReplayClass::parse("nondeterministic").is_err());
    }

    #[test]
    fn content_ref_serde_preserves_wire_string_and_rejects_invalid_input() {
        let valid = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let parsed = super::ContentRef::parse(valid).expect("valid content ref");
        let rendered = serde_json::to_string(&parsed).expect("serialized ref");
        assert_eq!(rendered, format!("\"{valid}\""));
        let decoded: super::ContentRef = serde_json::from_str(&rendered).expect("decoded ref");
        assert_eq!(decoded, parsed);
        assert_eq!(decoded.to_string(), valid);
        assert!(serde_json::from_str::<super::ContentRef>("\"blake3:not-hex\"").is_err());
        assert!(serde_json::from_str::<super::ContentRef>("\"/tmp/not-a-ref\"").is_err());
    }

    #[test]
    fn canonical_content_ref_matches_canonical_hash() {
        let value = super::parse_text("<content-ref-fixture [#t 42]>").expect("parse fixture");
        let reference = super::canonical_content_ref(&value).expect("canonical content ref");
        assert_eq!(reference.as_str(), super::canonical_hash(&value).expect("canonical hash"));
    }
