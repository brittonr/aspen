
    fn storage_extension_contract(profile: &str, version: &str) -> PluginExtensionContract {
        let authority_refs = vec![test_ref("storage-read-authority")];
        let resource_refs = vec![test_ref("storage-resource")];
        let effect_refs = vec![test_ref("effect-extension")];
        let error_refs = vec![test_ref("storage-error")];
        let descriptor_ref = test_ref("extension-storage-descriptor");
        let input_schema_ref = test_ref("storage-input-schema");
        let output_schema_ref = test_ref("storage-output-schema");
        let descriptor = PluginHostcallDescriptorInput {
            operation: "storage.read",
            descriptor_ref: &descriptor_ref,
            input_schema_ref: &input_schema_ref,
            output_schema_ref: &output_schema_ref,
            authority_refs: &authority_refs,
            resource_refs: &resource_refs,
            effect_manifest_refs: &effect_refs,
            replay_class: "idempotent",
            error_class_refs: &error_refs,
        };
        contract_from_descriptors(profile, version, &[descriptor])
    }

    fn storage_extension_contract_without_hostcall(profile: &str, version: &str) -> PluginExtensionContract {
        let authority_refs = vec![test_ref("other-authority")];
        let resource_refs = vec![test_ref("other-resource")];
        let effect_refs = vec![test_ref("effect-extension")];
        let error_refs = vec![test_ref("other-error")];
        let descriptor_ref = test_ref("other-descriptor");
        let input_schema_ref = test_ref("other-input-schema");
        let output_schema_ref = test_ref("other-output-schema");
        let descriptor = PluginHostcallDescriptorInput {
            operation: "storage.write",
            descriptor_ref: &descriptor_ref,
            input_schema_ref: &input_schema_ref,
            output_schema_ref: &output_schema_ref,
            authority_refs: &authority_refs,
            resource_refs: &resource_refs,
            effect_manifest_refs: &effect_refs,
            replay_class: "idempotent",
            error_class_refs: &error_refs,
        };
        contract_from_descriptors(profile, version, &[descriptor])
    }

    fn contract_from_descriptors(
        profile: &str,
        version: &str,
        descriptors: &[PluginHostcallDescriptorInput<'_>],
    ) -> PluginExtensionContract {
        let lifecycle = vec!["start".to_string(), "health".to_string()];
        let policy_refs = vec![test_ref("extension-policy")];
        let supply_refs = vec![test_ref("extension-supply")];
        let positive_suite_ref = test_ref("extension-positive-suite");
        let negative_suite_ref = test_ref("extension-negative-suite");
        let property_suite_ref = test_ref("extension-property-suite");
        let conformance = PluginExtensionConformanceInput {
            positive_suite_ref: &positive_suite_ref,
            negative_suite_ref: &negative_suite_ref,
            property_suite_ref: &property_suite_ref,
        };
        let value = plugin_extension_contract_value(&PluginExtensionContractInput {
            extension_id: "plugin-extension:storage",
            version,
            compatible_host_abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &lifecycle,
            hostcall_descriptors: descriptors,
            conformance,
            policy_refs: &policy_refs,
            supply_chain_refs: &supply_refs,
            production_profile: profile == PLUGIN_PROFILE_PRODUCTION,
        })
        .expect("extension contract value");
        parse_plugin_extension_contract(&value).expect("parse extension contract")
    }

    fn manifest_value_with_extension_refs(extension_contract_refs: &[String], effect_ref: &str) -> IoValue {
        plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:extension",
            artifact_ref: &test_ref("extension-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string(), "health".to_string(), "remove".to_string()],
            effect_manifest_refs: &[effect_ref.to_string()],
            hostcall_refs: &[storage_read_hostcall_ref().expect("primitive hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs,
        })
        .expect("manifest with extension refs")
    }

    fn manifest_with_extension_refs(extension_contract_refs: &[String], effect_ref: &str) -> PluginManifest {
        parse_plugin_manifest(&manifest_value_with_extension_refs(extension_contract_refs, effect_ref))
            .expect("parse extension manifest")
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
