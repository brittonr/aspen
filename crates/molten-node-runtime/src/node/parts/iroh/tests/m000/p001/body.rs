
    #[test]
    fn metrics_snapshot_renders_openmetrics_and_rejects_secret_labels() {
        let pass = metrics_snapshot(&MetricsSnapshotInput {
            node: "node-a".to_string(),
            scrape_ref: fixture_ref("scrape"),
            policy_refs: refs(),
            redaction_refs: refs(),
            samples: vec![MetricSample {
                name: "molten_node_queue_depth".to_string(),
                kind: "gauge".to_string(),
                value: METRIC_VALUE,
                labels: vec![("route".to_string(), "redacted".to_string())],
            }],
        })
        .expect("metrics pass");
        assert_eq!(pass.decision, "pass");
        assert!(pass.openmetrics.contains("molten_node_queue_depth"));

        let deny = metrics_snapshot(&MetricsSnapshotInput {
            samples: vec![MetricSample {
                name: "molten_secret".to_string(),
                kind: "counter".to_string(),
                value: METRIC_VALUE,
                labels: vec![("ticket".to_string(), "ticket:abc".to_string())],
            }],
            ..MetricsSnapshotInput {
                node: "node-a".to_string(),
                scrape_ref: fixture_ref("scrape"),
                policy_refs: refs(),
                redaction_refs: refs(),
                samples: Vec::new(),
            }
        })
        .expect("metrics deny");
        assert_eq!(deny.decision, "deny");
    }

    #[test]
    fn external_bridge_disabled_by_default_and_scoped_when_enabled() {
        let disabled = external_diagnostics_bridge_receipt(&ExternalDiagnosticsBridgeInput {
            enabled: false,
            mode: "push".to_string(),
            target_service_ref: None,
            capability_refs: Vec::new(),
            policy_refs: Vec::new(),
            redaction_policy_refs: Vec::new(),
            api_secret_provenance_ref: None,
            operator_evidence_refs: Vec::new(),
            expiry_ref: None,
        })
        .expect("disabled");
        assert_eq!(disabled.decision, "deny");

        let enabled = external_diagnostics_bridge_receipt(&ExternalDiagnosticsBridgeInput {
            enabled: true,
            mode: "remote-request".to_string(),
            target_service_ref: Some(fixture_ref("target")),
            capability_refs: refs(),
            policy_refs: refs(),
            redaction_policy_refs: refs(),
            api_secret_provenance_ref: Some(fixture_ref("secret-provenance")),
            operator_evidence_refs: refs(),
            expiry_ref: Some(fixture_ref("expiry")),
        })
        .expect("enabled");
        assert_eq!(enabled.decision, "pass");
    }
