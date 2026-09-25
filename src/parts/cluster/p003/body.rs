
#[cfg(test)]
mod tests {
    use super::*;

    fn node_names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(format!("cluster-lifecycle:{label}").as_bytes())
    }

    fn phase(name: &str) -> ClusterLifecyclePhaseObservation {
        ClusterLifecyclePhaseObservation {
            phase: name.to_string(),
            decision: CLUSTER_LIFECYCLE_PASS.to_string(),
            receipt_refs: vec![local_ref(&format!("phase:{name}"))],
        }
    }

    fn lifecycle_node(node_id: &str, manifest_ref: &str) -> ClusterLifecycleNodeSummary {
        ClusterLifecycleNodeSummary {
            node_id: node_id.to_string(),
            manifest_ref: manifest_ref.to_string(),
            config_ref: local_ref(&format!("{node_id}:config")),
            identity_ref: Some(local_ref(&format!("{node_id}:identity"))),
            startup_ref: Some(local_ref(&format!("{node_id}:startup"))),
            health_ref: Some(local_ref(&format!("{node_id}:health"))),
            queue_ref: Some(local_ref(&format!("{node_id}:queue"))),
            control_ref: Some(local_ref(&format!("{node_id}:control"))),
            heartbeat_ref: Some(local_ref(&format!("{node_id}:heartbeat"))),
            shutdown_ref: Some(local_ref(&format!("{node_id}:shutdown"))),
            stop_control_ref: Some(local_ref(&format!("{node_id}:stop-control"))),
            already_running_ref: Some(local_ref(&format!("{node_id}:already-running"))),
        }
    }

    fn lifecycle_input() -> ClusterLifecycleRunInput {
        let manifest_ref = local_ref("manifest");
        ClusterLifecycleRunInput {
            workflow_id: "cluster-two-node-lifecycle".to_string(),
            manifest_ref: manifest_ref.clone(),
            ordered_node_ids: vec!["node:node-a".to_string(), "node:node-b".to_string()],
            phases: vec![
                phase(CLUSTER_LIFECYCLE_PHASE_INIT),
                phase(CLUSTER_LIFECYCLE_PHASE_START),
                phase(CLUSTER_LIFECYCLE_PHASE_STATUS),
                phase(CLUSTER_LIFECYCLE_PHASE_STOP),
            ],
            node_summaries: vec![
                lifecycle_node("node:node-a", &manifest_ref),
                lifecycle_node("node:node-b", &manifest_ref),
            ],
            already_running_refs: vec![local_ref("already-running-observation")],
            stop_order: vec!["node:node-b".to_string(), "node:node-a".to_string()],
            diagnostics: Vec::new(),
            caveats: vec!["cluster lifecycle run evidence is local wrapper evidence only".to_string()],
        }
    }

    #[test]
    fn cluster_lifecycle_run_receipt_binds_complete_two_node_workflow() {
        // r[verify molten.testing.cluster_lifecycle_receipt.run_receipt]
        let receipt = build_cluster_lifecycle_run_receipt(&lifecycle_input()).expect("cluster lifecycle receipt");
        let rendered = crate::preserves_rail::to_text(&receipt.value).expect("render lifecycle receipt");

        assert_eq!(receipt.decision, CLUSTER_LIFECYCLE_PASS);
        assert!(receipt.diagnostics.is_empty());
        assert!(receipt.receipt_ref.starts_with("blake3:"));
        assert!(rendered.contains("cluster-lifecycle-run-v1"));
        assert!(rendered.contains("stdout-not-evidence"));
    }

    #[test]
    fn cluster_lifecycle_run_receipt_denies_missing_stale_and_stdout_only_evidence() {
        // r[verify molten.testing.cluster_lifecycle_receipt.fail_closed_validation]
        let mut input = lifecycle_input();
        for phase in &mut input.phases {
            phase.receipt_refs.clear();
        }
        input.node_summaries[0].identity_ref = None;
        input.node_summaries[0].startup_ref = None;
        input.node_summaries[0].health_ref = None;
        input.node_summaries[0].queue_ref = None;
        input.node_summaries[0].control_ref = None;
        input.node_summaries[0].heartbeat_ref = None;
        input.node_summaries[0].shutdown_ref = None;
        input.node_summaries[0].stop_control_ref = None;
        input.node_summaries[0].already_running_ref = None;
        input.node_summaries[1].node_id = input.node_summaries[0].node_id.clone();
        input.node_summaries[1].manifest_ref = local_ref("stale-manifest");
        input.node_summaries[1].identity_ref = None;
        input.node_summaries[1].startup_ref = None;
        input.node_summaries[1].health_ref = None;
        input.node_summaries[1].queue_ref = None;
        input.node_summaries[1].control_ref = None;
        input.node_summaries[1].heartbeat_ref = None;
        input.node_summaries[1].shutdown_ref = None;
        input.node_summaries[1].stop_control_ref = None;
        input.node_summaries[1].already_running_ref = None;
        input.already_running_refs.clear();
        input.stop_order.reverse();
        let receipt = build_cluster_lifecycle_run_receipt(&input).expect("denied lifecycle receipt");

        assert_eq!(receipt.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-missing-phase-receipts:")));
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-stale-manifest:")));
        assert!(receipt.diagnostics.iter().any(|item| item.starts_with("cluster-lifecycle-duplicate-node-summary:")));
        assert!(receipt.diagnostics.iter().any(|item| item == "cluster-lifecycle-stop-order-drift"));
        assert!(receipt.diagnostics.iter().any(|item| item == "cluster-lifecycle-stdout-only-evidence"));
    }

    #[test]
    fn cluster_lifecycle_drift_summary_compares_stable_fields() {
        // r[verify molten.testing.cluster_lifecycle_summary_drift.receipt_summary]
        let first = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("first lifecycle summary");
        let second = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("second lifecycle summary");
        let comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: first,
            right: second,
            allowed_variances: Vec::new(),
        })
        .expect("stable lifecycle drift comparison");

        assert_eq!(comparison.decision, CLUSTER_LIFECYCLE_PASS);
        assert!(comparison.diagnostics.is_empty());
    }

    #[test]
    fn cluster_lifecycle_drift_summary_denies_child_node_and_field_kind_drift() {
        // r[verify molten.testing.cluster_lifecycle_summary_drift.negatives]
        let left = cluster_lifecycle_drift_summary(&lifecycle_input()).expect("left summary");
        let mut changed_child_input = lifecycle_input();
        changed_child_input.node_summaries[0].startup_ref = Some(local_ref("node-a:startup:changed"));
        let changed_child = cluster_lifecycle_drift_summary(&changed_child_input).expect("changed child summary");
        let child_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: left.clone(),
            right: changed_child,
            allowed_variances: Vec::new(),
        })
        .expect("changed child comparison");

        let mut changed_order_input = lifecycle_input();
        changed_order_input.ordered_node_ids.reverse();
        let changed_order = cluster_lifecycle_drift_summary(&changed_order_input).expect("changed order summary");
        let order_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left: left.clone(),
            right: changed_order,
            allowed_variances: Vec::new(),
        })
        .expect("changed order comparison");

        let mut missing_field_input = lifecycle_input();
        missing_field_input.node_summaries[0].startup_ref = None;
        let missing_field = cluster_lifecycle_drift_summary(&missing_field_input).expect("missing field summary");
        let field_kind_comparison = crate::deterministic_drift::compare(&crate::deterministic_drift::ComparisonInput {
            left,
            right: missing_field,
            allowed_variances: Vec::new(),
        })
        .expect("field kind comparison");

        assert_eq!(child_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(child_comparison.diagnostics.iter().any(|diagnostic| diagnostic.path == "node:node:node-a:startup"));
        assert_eq!(order_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(order_comparison.diagnostics.iter().any(|diagnostic| diagnostic.path == "node-order"));
        assert_eq!(field_kind_comparison.decision, CLUSTER_LIFECYCLE_DENY);
        assert!(field_kind_comparison.diagnostics.iter().any(|diagnostic| diagnostic.kind == "field-kind-drift"));
    }

    #[test]
    fn plans_cluster_nodes_and_round_trips_manifest() {
        const EXPECTED_CLUSTER_NODE_COUNT: usize = 2;

        let root = std::path::PathBuf::from("target/cluster");
        let plan = plan_cluster(&root, &node_names(&["node-a", "node_b"])).expect("cluster plan");
        assert_eq!(plan.nodes.len(), EXPECTED_CLUSTER_NODE_COUNT);
        assert_eq!(plan.nodes[0].node_id, "node:node-a");
        assert_eq!(plan.nodes[0].path_component, "node-a");
        assert_eq!(plan.nodes[0].state_root, root.join("node-a"));
        assert_eq!(plan.nodes[1].node_id, "node:node_b");
        assert_eq!(cluster_manifest_path(&root), root.join(CLUSTER_MANIFEST_FILE));

        let manifest = render_cluster_manifest(&plan);
        let parsed = parse_cluster_manifest(&manifest).expect("parse manifest");
        let reparsed_plan = plan_cluster(&root, &parsed).expect("reparsed plan");
        let reparsed_node_ids: Vec<&str> = reparsed_plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        let planned_node_ids: Vec<&str> = plan.nodes.iter().map(|node| node.node_id.as_str()).collect();
        assert_eq!(reparsed_node_ids, planned_node_ids);
        assert_eq!(reparsed_plan.state_root, plan.state_root);
    }

    #[test]
    fn denies_empty_duplicate_and_unsafe_nodes() {
        let root = std::path::PathBuf::from("target/cluster");
        let empty = plan_cluster(&root, &[]).expect_err("empty denied");
        assert!(empty.to_string().contains("at least one"));

        let duplicate = plan_cluster(&root, &node_names(&["node-a", "node:node-a"])).expect_err("duplicate denied");
        assert!(duplicate.to_string().contains("duplicate cluster node"));

        let relative = plan_cluster(&root, &node_names(&["../node-a"])).expect_err("relative denied");
        assert!(relative.to_string().contains("ASCII letters"));

        let colon = plan_cluster(&root, &node_names(&["node:a:b"])).expect_err("colon denied");
        assert!(colon.to_string().contains("must not contain ':'"));

        let current_root =
            plan_cluster(std::path::Path::new("."), &node_names(&["node-a"])).expect_err("current root denied");
        assert!(current_root.to_string().contains("must not be ambient"));

        let parent_root =
            plan_cluster(std::path::Path::new(".."), &node_names(&["node-a"])).expect_err("parent root denied");
        assert!(parent_root.to_string().contains("must not be ambient"));
    }

    #[test]
    fn denies_malformed_manifests() {
        let empty = parse_cluster_manifest("").expect_err("empty manifest denied");
        assert!(empty.to_string().contains("manifest is empty"));

        let header = parse_cluster_manifest("not-a-cluster\nnode:node-a\n").expect_err("bad header denied");
        assert!(header.to_string().contains("unsupported header"));

        let no_nodes = parse_cluster_manifest("molten.cluster.nodes.v1\n").expect_err("empty nodes denied");
        assert!(no_nodes.to_string().contains("no nodes"));
    }

    #[test]
    fn bounds_manifest_node_count() {
        let manifest_with = |count: usize| {
            let mut manifest = format!("{CLUSTER_MANIFEST_HEADER}\n");
            for index in 0..count {
                manifest.push_str(&format!("node:node-{index}\n"));
            }
            manifest
        };

        let at_limit = parse_cluster_manifest(&manifest_with(MAX_CLUSTER_MANIFEST_NODES)).expect("at-limit manifest");
        assert_eq!(at_limit.len(), MAX_CLUSTER_MANIFEST_NODES);
        let last_node = format!("node:node-{}", MAX_CLUSTER_MANIFEST_NODES - 1);
        assert_eq!(at_limit.last(), Some(&last_node));

        let past_limit = parse_cluster_manifest(&manifest_with(MAX_CLUSTER_MANIFEST_NODES + 1))
            .expect_err("one-past-limit manifest denied");
        let expected = format!(
            "cluster manifest node count {} exceeds maximum {MAX_CLUSTER_MANIFEST_NODES}",
            MAX_CLUSTER_MANIFEST_NODES + 1
        );
        assert!(past_limit.to_string().contains(&expected));
    }
}
