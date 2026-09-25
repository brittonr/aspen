    const ANNOTATED_ONE_PACKED: &[u8] = b"\x85\xb0\x01\x02\xb0\x01\x01";
    const TRAILING_SENTINEL_BYTE: u8 = 0;
    const TAMPER_MASK: u8 = 1;

    #[test]
    fn preserves_text_roundtrip_keeps_hash() {
        let value = super::parse_text("<example \"a\" [1 2 3]>").expect("parse initial text");
        let hash = super::canonical_hash(&value).expect("hash initial value");
        let rendered = super::to_text(&value).expect("render preserves text");
        let reparsed = super::parse_text(&rendered).expect("parse rendered text");
        assert_eq!(hash, super::canonical_hash(&reparsed).expect("hash reparsed value"));
    }

    #[test]
    fn strict_canonical_decode_accepts_molten_canonical_bytes() {
        // r[verify molten.preserves_canonical_bytes.strict_decode]
        let value = super::parse_text("<strict-decode-fixture [#t 42]>").expect("parse fixture");
        let bytes = super::canonical_bytes(&value).expect("canonical bytes");
        let decoded = super::strict_canonical_decode(&bytes).expect("strict decode");
        assert_eq!(decoded.value, value);
        assert_eq!(decoded.canonical_bytes, bytes);
        assert_eq!(decoded.value_ref.as_str(), super::canonical_hash(&decoded.value).expect("decoded hash"));
    }

    #[test]
    fn strict_canonical_decode_rejects_annotated_trailing_truncated_and_tampered_bytes() {
        // r[verify molten.preserves_canonical_bytes.noncanonical_denial]
        let annotated_error = super::strict_canonical_decode(ANNOTATED_ONE_PACKED)
            .expect_err("annotations are parseable but not canonical without annotations");
        assert!(annotated_error.to_string().contains("strict canonical Preserves decode failed"));

        let value = super::parse_text("<strict-decode-fixture \"payload\">").expect("parse fixture");
        let mut trailing = super::canonical_bytes(&value).expect("canonical bytes");
        trailing.push(TRAILING_SENTINEL_BYTE);
        assert!(super::strict_canonical_decode(&trailing).is_err());

        let mut truncated = super::canonical_bytes(&value).expect("canonical bytes");
        truncated.pop();
        assert!(super::strict_canonical_decode(&truncated).is_err());

        let original = super::canonical_bytes(&value).expect("canonical bytes");
        let original_ref = super::content_ref_from_bytes(&original);
        let mut tampered = original.clone();
        let first = tampered.first_mut().expect("non-empty canonical bytes");
        *first ^= TAMPER_MASK;
        assert!(super::strict_canonical_decode_with_ref(&tampered, &original_ref, "tampered-fixture").is_err());
    }

    fn boundary_schema_specs() -> Vec<&'static super::BoundarySchemaSpec> {
        vec![
            &super::NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA,
            &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA,
            &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA,
            &super::RETENTION_RECEIPT_BOUNDARY_SCHEMA,
            &super::EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_SCHEMA,
            &super::OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_SCHEMA,
        ]
    }

    fn boundary_test_ref(label: &str) -> String {
        super::content_ref_from_bytes(format!("boundary-schema-test-{label}").as_bytes())
    }

    fn boundary_schema_fixture(spec: &super::BoundarySchemaSpec) -> preserves::IOValue {
        super::record(
            spec.record_label,
            spec.fields
                .iter()
                .map(|field| boundary_field_fixture(spec, field))
                .collect(),
        )
    }

    fn boundary_field_fixture(
        spec: &super::BoundarySchemaSpec,
        field: &super::BoundaryFieldSpec,
    ) -> preserves::IOValue {
        match field.kind {
            super::BoundaryFieldKind::SchemaId => super::string(spec.schema_id),
            super::BoundaryFieldKind::AnyRecord => {
                super::record(field.label, vec![super::record("payload", vec![super::string("value")])])
            }
            super::BoundaryFieldKind::AnySequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("artifact", vec![super::string(boundary_test_ref(field.label))]),
            ])]),
            super::BoundaryFieldKind::ChainRecord => super::record("chain", vec![
                super::record("scope", vec![super::string("test-scope")]),
                super::record("id", vec![super::string("test-id")]),
                super::record("epoch", vec![super::string("test-epoch")]),
            ]),
            super::BoundaryFieldKind::ChecksRecord => super::checks_value(&[("schema-bound", "pass")]),
            super::BoundaryFieldKind::ConformanceRecord => super::record(field.label, vec![
                super::record("positive", vec![super::string(boundary_test_ref("positive"))]),
                super::record("negative", vec![super::string(boundary_test_ref("negative"))]),
                super::record("property", vec![super::string(boundary_test_ref("property"))]),
            ]),
            super::BoundaryFieldKind::DecisionRecord => super::record(field.label, vec![super::string("pass")]),
            super::BoundaryFieldKind::FileRefsRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("file", vec![
                    super::string("member.txt"),
                    super::string(boundary_test_ref("member")),
                ]),
            ])]),
            super::BoundaryFieldKind::HostcallDescriptorsRecord => hostcall_descriptors_fixture(field.label),
            super::BoundaryFieldKind::NonEmptyRefSequenceRecord | super::BoundaryFieldKind::RefSequenceRecord | super::BoundaryFieldKind::UniqueRefSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(boundary_test_ref(field.label)),
            ])]),
            super::BoundaryFieldKind::NonEmptyStringRecord | super::BoundaryFieldKind::StableIdRecord | super::BoundaryFieldKind::StringRecord => {
                super::record(field.label, vec![super::string(format!("{}-value", field.label))])
            }
            super::BoundaryFieldKind::ObjectRecord => super::record(field.label, vec![
                super::string(boundary_test_ref("object")),
                super::string("artifact"),
            ]),
            super::BoundaryFieldKind::OptionalRefRecord => super::record(field.label, vec![super::record(
                "some",
                vec![super::string(boundary_test_ref(field.label))],
            )]),
            super::BoundaryFieldKind::RefAndStringRecord => super::record(field.label, vec![
                super::string(boundary_test_ref(field.label)),
                super::string(format!("{}-path", field.label)),
            ]),
            super::BoundaryFieldKind::RefRecord => {
                super::record(field.label, vec![super::string(boundary_test_ref(field.label))])
            }


            super::BoundaryFieldKind::StringAndRefRecord => super::record(field.label, vec![
                super::string(format!("{}-value", field.label)),
                super::string(boundary_test_ref(field.label)),
            ]),

            super::BoundaryFieldKind::StringSequenceRecord | super::BoundaryFieldKind::UniqueStringSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(format!("{}-item", field.label)),
            ])]),


            super::BoundaryFieldKind::TwoRefsRecord => super::record(field.label, vec![
                super::string(boundary_test_ref(&format!("{}-a", field.label))),
                super::string(boundary_test_ref(&format!("{}-b", field.label))),
            ]),
            super::BoundaryFieldKind::U64Record => super::record(field.label, vec![super::u64_value(1)]),
        }
    }

    fn hostcall_descriptors_fixture(label: &'static str) -> preserves::IOValue {
        super::record(label, vec![super::sequence(vec![
                super::record("hostcall-descriptor", vec![
                    super::record("operation", vec![super::string("storage.read")]),
                    super::record("descriptor", vec![super::string(boundary_test_ref("descriptor"))]),
                    super::record("input-schema", vec![super::string(boundary_test_ref("input-schema"))]),
                    super::record("output-schema", vec![super::string(boundary_test_ref("output-schema"))]),
                    super::record("authority", vec![super::sequence(vec![super::string(boundary_test_ref("authority"))])]),
                    super::record("resource", vec![super::sequence(vec![super::string(boundary_test_ref("resource"))])]),
                    super::record("effects", vec![super::sequence(vec![super::string(boundary_test_ref("effects"))])]),
                    super::record("replay", vec![super::string("deterministic")]),
                    super::record("errors", vec![super::sequence(vec![super::string(boundary_test_ref("errors"))])]),
                ]),
            ])])
    }

    fn boundary_fixture_fields(
        value: &preserves::IOValue,
        spec: &super::BoundarySchemaSpec,
    ) -> Vec<preserves::IOValue> {
        let record = value
            .collect_simple_record(spec.record_label, Some(spec.fields.len()))
            .expect("boundary fixture record");
        let mut fields = Vec::with_capacity(spec.fields.len());
        for index in 0..spec.fields.len() {
            fields.push(super::value_to_iovalue(&record[index]));
        }
        fields
    }

    fn boundary_fixture_with_field(
        spec: &super::BoundarySchemaSpec,
        field_index: usize,
        replacement: preserves::IOValue,
    ) -> preserves::IOValue {
        let fixture = boundary_schema_fixture(spec);
        let mut fields = boundary_fixture_fields(&fixture, spec);
        fields[field_index] = replacement;
        super::record(spec.record_label, fields)
    }

    fn boundary_field_index(
        spec: &super::BoundarySchemaSpec,
        predicate: impl Fn(&super::BoundaryFieldSpec) -> bool,
    ) -> usize {
        spec.fields.iter().position(predicate).expect("matching boundary field")
    }

    fn malformed_ref_field(field: &super::BoundaryFieldSpec) -> preserves::IOValue {
        match field.kind {
            super::BoundaryFieldKind::RefRecord => super::record(field.label, vec![super::sequence(Vec::new())]),
            super::BoundaryFieldKind::RefSequenceRecord
            | super::BoundaryFieldKind::NonEmptyRefSequenceRecord
            | super::BoundaryFieldKind::UniqueRefSequenceRecord => {
                super::record(field.label, vec![super::sequence(vec![super::sequence(Vec::new())])])
            }
            super::BoundaryFieldKind::OptionalRefRecord => {
                super::record(field.label, vec![super::record("some", vec![super::sequence(Vec::new())])])
            }
            super::BoundaryFieldKind::StringAndRefRecord => super::record(field.label, vec![
                super::string("name"),
                super::sequence(Vec::new()),
            ]),
            super::BoundaryFieldKind::RefAndStringRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string("name"),
            ]),
            super::BoundaryFieldKind::TwoRefsRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string(boundary_test_ref(field.label)),
            ]),
            super::BoundaryFieldKind::FileRefsRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("file", vec![super::string("member.txt"), super::sequence(Vec::new())]),
            ])]),
            super::BoundaryFieldKind::ObjectRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string("artifact"),
            ]),
            _ => super::sequence(Vec::new()),
        }
    }

    fn is_ref_bearing_field(field: &super::BoundaryFieldSpec) -> bool {
        matches!(
            field.kind,
            super::BoundaryFieldKind::FileRefsRecord
                | super::BoundaryFieldKind::ObjectRecord
                | super::BoundaryFieldKind::OptionalRefRecord
                | super::BoundaryFieldKind::RefAndStringRecord
                | super::BoundaryFieldKind::NonEmptyRefSequenceRecord
                | super::BoundaryFieldKind::RefRecord
                | super::BoundaryFieldKind::RefSequenceRecord
                | super::BoundaryFieldKind::StringAndRefRecord
                | super::BoundaryFieldKind::UniqueRefSequenceRecord
                | super::BoundaryFieldKind::TwoRefsRecord
        )
    }

    #[test]
    fn boundary_schema_adapter_accepts_valid_versioned_records_for_all_adopted_families() {
        // r[verify molten.preserves_schema_boundaries.schema_adapter]
        // r[verify molten.preserves_boundary_field_contracts.field_contracts]
        for spec in boundary_schema_specs() {
            let value = boundary_schema_fixture(spec);
            let validation = super::validate_boundary_schema(&value, spec).expect("schema validation");
            assert_eq!(validation.decision, "pass");
            assert!(validation.schema_ref.as_str().starts_with("blake3:"));
            assert_eq!(validation.value_ref.as_str(), super::canonical_hash(&value).expect("value ref"));
            let diagnostic = super::boundary_schema_diagnostic_value(&validation);
            let diagnostic_text = super::to_text(&diagnostic).expect("diagnostic text");
            assert!(diagnostic_text.contains(spec.family));
            assert!(diagnostic_text.contains("schema-ref"));
        }
    }

    #[test]
    fn boundary_codec_report_binds_strict_decode_schema_and_typed_refs() {
        // r[verify molten.preserves_schema_boundaries.schema_artifacts]
        // r[verify molten.preserves_boundary_codegen.typed_codecs]
        // r[verify molten.preserves_boundary_codegen.strict_decode]
        // r[verify molten.preserves_boundary_codegen.schema_ref_evidence]
        // r[verify molten.preserves_boundary_codegen.fixture_corpus]
        let spec = &super::NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA;
        let value = boundary_schema_fixture(spec);
        let bytes = super::canonical_bytes(&value).expect("canonical boundary fixture bytes");
        let report = super::validate_boundary_bytes(&bytes, spec).expect("boundary codec report");
        assert_eq!(report.decision, "pass");
        assert_eq!(report.schema_ref, super::boundary_schema_ref(spec).expect("schema ref"));
        assert_eq!(report.input_bytes_ref.as_str(), super::content_ref_from_bytes(&bytes));
        assert_eq!(report.decoded_value_ref.as_str(), super::canonical_hash(&value).expect("value ref"));
        assert!(report.typed_value_ref.as_str().starts_with("blake3:"));
        let rendered = super::to_text(&super::boundary_codec_report_value(&report)).expect("report text");
        assert!(rendered.contains("typed-value-ref"));
        assert!(rendered.contains(spec.family));
    }
