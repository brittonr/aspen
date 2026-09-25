
    #[test]
    fn raw_closure_config_denies_before_execution() {
        // r[verify molten.preserves_value_inspection.ambient_token_denial]
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("ok")]),
            ])]),
        )
        .expect("source");
        let bad = test_node_value(
            "bad",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("host-path", vec![crate::preserves_rail::string("/bin/echo")]),
        );
        assert!(bad.expect_err("bad config").to_string().contains("mobile/ambient"));
        let edge = stream_edge_value("source", "bad").expect("edge");
        let bad_node = crate::preserves_rail::record("job-node-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_NODE_SCHEMA),
            crate::preserves_rail::record("id", vec![crate::preserves_rail::string("bad")]),
            crate::preserves_rail::record("kind", vec![crate::preserves_rail::string("map")]),
            crate::preserves_rail::record("stage-artifact", vec![crate::preserves_rail::record("none", Vec::new())]),
            crate::preserves_rail::record("inputs", vec![ports_sequence(&["in".to_string()])]),
            crate::preserves_rail::record("outputs", vec![ports_sequence(&["out".to_string()])]),
            crate::preserves_rail::record("config", vec![crate::preserves_rail::record("host-path", vec![
                crate::preserves_rail::string("/bin/echo"),
            ])]),
            crate::preserves_rail::record("effects", vec![crate::preserves_rail::sequence(Vec::new())]),
            crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(Vec::new())]),
            crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(Vec::new())]),
            checks_value(&["stage-artifact-not-closure"]),
        ]);
        let dag = test_dag_value(vec![source, bad_node], vec![edge], &["bad".to_string()]).expect("dag");
        assert!(parse_job_dag_value(&dag).expect_err("parse rejects").to_string().contains("mobile/ambient"));
    }

    #[test]
    fn rendered_ambient_looking_string_is_not_a_structural_token() {
        let node = test_node_value(
            "string-only",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::string("<host-path \"diagnostic-looking\">")
        )
        .expect("rendered-looking string is inert");
        let text = crate::preserves_rail::to_text(&node).expect("node text");
        assert!(text.contains("host-path"));
    }

    #[hegel::test(test_cases = 10)]
    fn hegel_dag_hash_and_memo_key_are_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dag = fixture_value(if salt.is_multiple_of(2) { "identity" } else { "count" });
        let first = parse_job_dag_value(&dag).expect("first");
        let second = parse_job_dag_value(
            &crate::preserves_rail::parse_text(&crate::preserves_rail::to_text(&dag).expect("text"))
                .expect("parse text"),
        )
        .expect("second");
        assert_eq!(first.job_ref, second.job_ref);
        assert_eq!(
            execution_order(&first.nodes, &first.edges).expect("order"),
            execution_order(&second.nodes, &second.edges).expect("order")
        );
    }
