
#[cfg(test)]
mod tests {
    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn manifest_value_for_artifact(artifact_ref: &str) -> IoValue {
        let lifecycle_callbacks = string_vec(&["init", "start", "health", "stop", "remove"]);
        let effect_refs = vec![test_ref("effect")];
        let hostcall_refs = vec![storage_read_hostcall_ref().expect("hostcall ref")];
        let schema_refs = vec![test_ref("schema")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let supply_refs = vec![test_ref("supply")];
        plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:test",
            artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &lifecycle_callbacks,
            effect_manifest_refs: &effect_refs,
            hostcall_refs: &hostcall_refs,
            schema_refs: &schema_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            supply_chain_refs: &supply_refs,
        })
        .expect("manifest")
    }

    #[test]
    fn plugin_fixture_runs_lifecycle_and_upgrade() {
        let dir = temp_dir("plugin-fixture");
        let run = minimal_plugin_fixture(&dir).expect("minimal plugin fixture");
        assert_eq!(run.decision, "pass");
        crate::preserves_rail::validate_content_ref(&run.manifest_ref).expect("manifest ref is canonical");
        crate::preserves_rail::validate_content_ref(&run.install_receipt_ref)
            .expect("install receipt ref is canonical");
        assert!(plugin_summary(&run.report_value).expect("summary").contains("plugin fixture report"));
        assert!(run.evidence_values.len() >= 10);
    }

    #[test]
    fn raw_host_path_missing_artifact_and_stale_provenance_deny() {
        let malformed = parse_text(
            "<plugin-manifest-v1 \"molten.plugin.manifest.v1\" \
             <plugin-id \"plugin:path\"> <artifact \"/usr/bin/plugin\"> <abi \"molten.plugin.host-abi.v1\"> \
             <lifecycle [\"start\"]> <effects []> <hostcalls []> <schemas []> <policy []> <resource []> \
             <supply-chain []> <checks [<check \"artifact-backed\" \"fail\"> <check \"no-ambient-authority\" \"pass\">]>>",
        )
        .expect("parse malformed manifest");
        assert!(parse_plugin_manifest(&malformed).is_err());

        let dir = temp_dir("plugin-deny");
        let registry = dir.join("registry");
        let artifact = crate::artifacts::install_artifact(&registry, &crate::artifacts::ArtifactInstallInput {
            kind: "plugin-executor".to_string(),
            payload: record("plugin", vec![string("x")]),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: Some(test_ref("effect")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("supply")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install artifact");
        let manifest = manifest_value_for_artifact(&artifact.artifact_ref);
        let install = install_plugin(&registry, &manifest).expect("install plugin");
        assert_eq!(install.decision, "pass");
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &manifest,
            authority_refs: &[test_ref("authority")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("stale-supply")],
        })
        .expect("permission receipt");
        let parsed = parse_plugin_permission_receipt(&permission).expect("parse permission");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("supply-chain")));
    }

    #[test]
    fn ambient_hostcall_failed_health_and_cleanup_are_receipted() {
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:ambient",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &[
                "start".to_string(),
                "health".to_string(),
                "stop".to_string(),
                "remove".to_string(),
            ],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("storage hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let denied_hostcall = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest,
            operation: "network.open",
            hostcall_ref: &network_open_hostcall_ref().expect("network hostcall"),
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("authority")],
            resource_refs: &[test_ref("resource")],
        })
        .expect("hostcall receipt");
        let denied = parse_plugin_hostcall_receipt(&denied_hostcall).expect("parse hostcall");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("ambient")));
        let health = plugin_health_receipt_value(&HealthReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("start"),
            service_refs: &[test_ref("service")],
            health_status: "failed",
            diagnostics: &["probe failed".to_string()],
        })
        .expect("health receipt");
        assert_eq!(parse_plugin_health_receipt(&health).expect("parse health").decision, "deny");
        let incomplete_removal = plugin_removal_receipt_value(&RemovalReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("remove"),
            owned_service_refs: &[test_ref("service")],
            assertion_refs: &[],
            handle_refs: &[],
            catalog_entry_refs: &[],
            diagnostics: &[],
        })
        .expect("removal receipt");
        assert_eq!(parse_plugin_removal_receipt(&incomplete_removal).expect("parse removal").decision, "deny");
    }

    #[test]
    fn host_abi_result_and_upgrade_compatibility_are_canonical() {
        let payload_ref = test_ref("payload");
        let result = plugin_host_abi_result_value(&HostAbiResultInput {
            status: "ok",
            payload_ref: Some(&payload_ref),
            error: None,
        })
        .expect("ABI result");
        assert!(to_text(&result).expect("render result").contains("plugin-host-abi-result-v1"));
        let old_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("old-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("old manifest");
        let new_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("new-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema"), test_ref("schema-extra")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("new manifest");
        let upgrade = plugin_upgrade_receipt_value(&UpgradeReceiptInput {
            old_manifest_value: &old_manifest,
            new_manifest_value: &new_manifest,
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            diagnostics: &[],
        })
        .expect("upgrade receipt");
        assert_eq!(parse_plugin_upgrade_receipt(&upgrade).expect("parse upgrade").decision, "pass");
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_plugin_artifacts() {
        let dir = temp_dir("plugin-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:catalog",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let imported = crate::ledger::import_artifact(&ledger_root, &manifest).expect("ledger import");
        assert_eq!(imported.artifact_kind, "plugin-manifest");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("plugin-manifest".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list plugin manifest");
        assert_eq!(listed.items.len(), 1);
        let rendered = to_text(&listed.value).expect("render catalog result");
        assert!(rendered.contains("ledger-kind:plugin-manifest"));
        let request = crate::catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "plugin-manifest",
        )])])
        .expect("MCP request");
        let mcp = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list plugin manifest");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("plugin-manifest"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_plugin_lifecycle_refs_are_deterministic_and_authority_gated(tc: hegel::TestCase) {
        let callback_count = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(4));
        let callback_count = usize::try_from(callback_count).expect("bounded callback count");
        let callbacks = ["init", "start", "health", "stop"]
            .iter()
            .take(callback_count)
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        let artifact_ref = test_ref("artifact-property");
        let value = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:property",
            artifact_ref: &artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &callbacks,
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let first_ref = canonical_hash(&value).expect("first ref");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(first_ref, canonical_hash(&reparsed).expect("second ref"));
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &value,
            authority_refs: &[],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("permission receipt");
        assert_eq!(parse_plugin_permission_receipt(&permission).expect("parse permission").decision, "deny");
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
