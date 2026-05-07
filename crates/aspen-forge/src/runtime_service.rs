//! Runtime-service wrapper metadata for Forge.
//!
//! This module exposes Forge as a linked native Aspen runtime service without
//! changing Forge internals or introducing a dynamic native plugin boundary.

use aspen_runtime_core::NativeBuiltInServiceFactory;
use aspen_runtime_core::NativeLoadingPolicy;
use aspen_runtime_core::NativeServiceManifest;
use aspen_runtime_core::RedactedValue;
use aspen_runtime_core::RuntimeCapabilityBinding;
use aspen_runtime_core::RuntimeCaveat;
use aspen_runtime_core::RuntimeDiagnostic;
use aspen_runtime_core::RuntimeHealthState;
use aspen_runtime_core::RuntimeLifecycleStatus;
use aspen_runtime_core::RuntimeReceipt;
use aspen_runtime_core::RuntimeRouteDeclaration;

pub const FORGE_RUNTIME_SERVICE_NAME: &str = "forge";
pub const FORGE_RUNTIME_SERVICE_SYMBOL: &str = "aspen_forge::runtime_service::forge_runtime_service_factory";
pub const FORGE_GIT_ROUTE_ID: &str = "forge.git";
pub const FORGE_REPO_ROUTE_ID: &str = "forge.repo";
pub const FORGE_HEALTH_ROUTE_ID: &str = "forge.health";

#[must_use]
pub fn forge_runtime_manifest() -> NativeServiceManifest {
    NativeServiceManifest {
        name: FORGE_RUNTIME_SERVICE_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        routes: forge_runtime_routes(),
        required_capabilities: forge_runtime_capabilities(),
    }
}

#[must_use]
pub fn forge_runtime_service_factory() -> NativeBuiltInServiceFactory {
    NativeBuiltInServiceFactory {
        service_name: FORGE_RUNTIME_SERVICE_NAME.to_string(),
        linked_symbol: FORGE_RUNTIME_SERVICE_SYMBOL.to_string(),
        loading_policy: NativeLoadingPolicy::LinkedBuiltInOnly,
        manifest: forge_runtime_manifest(),
    }
}

#[must_use]
pub fn forge_runtime_routes() -> Vec<RuntimeRouteDeclaration> {
    vec![
        RuntimeRouteDeclaration {
            route_id: FORGE_GIT_ROUTE_ID.to_string(),
            protocol: "iroh-alpn".to_string(),
            owner_unit: FORGE_RUNTIME_SERVICE_NAME.to_string(),
            handler: "forge.git".to_string(),
        },
        RuntimeRouteDeclaration {
            route_id: FORGE_REPO_ROUTE_ID.to_string(),
            protocol: "aspen-client-rpc".to_string(),
            owner_unit: FORGE_RUNTIME_SERVICE_NAME.to_string(),
            handler: "forge.repo".to_string(),
        },
        RuntimeRouteDeclaration {
            route_id: FORGE_HEALTH_ROUTE_ID.to_string(),
            protocol: "runtime-health".to_string(),
            owner_unit: FORGE_RUNTIME_SERVICE_NAME.to_string(),
            handler: "forge.health".to_string(),
        },
    ]
}

#[must_use]
pub fn forge_runtime_capabilities() -> Vec<RuntimeCapabilityBinding> {
    vec![
        RuntimeCapabilityBinding {
            handle_id: "forge.kv".to_string(),
            ability: "store/read-write".to_string(),
            resource: "aspen://kv/forge".to_string(),
            proof_refs: vec!["operator-grant:forge.kv".to_string()],
            caveats: vec![RuntimeCaveat {
                name: "prefix".to_string(),
                value_shape: "forge/".to_string(),
            }],
        },
        RuntimeCapabilityBinding {
            handle_id: "forge.blob".to_string(),
            ability: "blob/read-write".to_string(),
            resource: "aspen://blob/forge".to_string(),
            proof_refs: vec!["operator-grant:forge.blob".to_string()],
            caveats: vec![RuntimeCaveat {
                name: "content-addressed-only".to_string(),
                value_shape: "blake3".to_string(),
            }],
        },
    ]
}

#[must_use]
pub fn forge_health_receipt(status: RuntimeHealthState) -> RuntimeReceipt {
    let lifecycle_status = match status {
        RuntimeHealthState::Unknown | RuntimeHealthState::Starting => RuntimeLifecycleStatus::Starting,
        RuntimeHealthState::Healthy | RuntimeHealthState::Degraded | RuntimeHealthState::Unhealthy => {
            RuntimeLifecycleStatus::Running
        }
        RuntimeHealthState::Stopped => RuntimeLifecycleStatus::Stopped,
    };
    RuntimeReceipt {
        receipt_id: "forge.health".to_string(),
        unit_id: FORGE_RUNTIME_SERVICE_NAME.to_string(),
        host_kind: aspen_runtime_core::RuntimeHostKind::NativeBuiltIn,
        lifecycle_status,
        artifact_summary: format!("built-in:{FORGE_RUNTIME_SERVICE_NAME}@{}", env!("CARGO_PKG_VERSION")),
        granted_authority: forge_runtime_manifest()
            .capability_handle_refs()
            .into_iter()
            .filter_map(|value| match value {
                RedactedValue::OpaqueHandle(handle) => Some(handle),
                _ => None,
            })
            .collect(),
        diagnostics: vec![RuntimeDiagnostic {
            key: "health".to_string(),
            value: RedactedValue::OpaqueHandle(format!("forge.health.{status:?}")),
        }],
    }
}

#[must_use]
pub fn forge_lifecycle_receipt(status: RuntimeLifecycleStatus) -> RuntimeReceipt {
    RuntimeReceipt {
        receipt_id: format!("forge.lifecycle.{status:?}"),
        unit_id: FORGE_RUNTIME_SERVICE_NAME.to_string(),
        host_kind: aspen_runtime_core::RuntimeHostKind::NativeBuiltIn,
        lifecycle_status: status,
        artifact_summary: format!("built-in:{FORGE_RUNTIME_SERVICE_NAME}@{}", env!("CARGO_PKG_VERSION")),
        granted_authority: forge_runtime_manifest()
            .capability_handle_refs()
            .into_iter()
            .filter_map(|value| match value {
                RedactedValue::OpaqueHandle(handle) => Some(handle),
                _ => None,
            })
            .collect(),
        diagnostics: vec![RuntimeDiagnostic {
            key: "lifecycle".to_string(),
            value: RedactedValue::OpaqueHandle("forge.lifecycle".to_string()),
        }],
    }
}

#[cfg(test)]
mod tests {
    use aspen_runtime_core::admit_native_factory;
    use aspen_runtime_core::admit_receipt;

    use super::*;

    #[test]
    fn forge_runtime_factory_is_linked_native_startup_anchor() {
        let factory = forge_runtime_service_factory();
        admit_native_factory(&factory).unwrap();
        assert_eq!(factory.service_name, FORGE_RUNTIME_SERVICE_NAME);
        assert_eq!(factory.loading_policy, NativeLoadingPolicy::LinkedBuiltInOnly);
        assert!(factory.linked_symbol.ends_with("forge_runtime_service_factory"));
        let declaration = factory.as_declaration("forge");
        assert_eq!(declaration.host_kind, aspen_runtime_core::RuntimeHostKind::NativeBuiltIn);
        assert!(matches!(declaration.artifact, aspen_runtime_core::RuntimeArtifact::BuiltIn { .. }));
    }

    #[test]
    fn forge_routes_are_owned_and_registered_in_manifest() {
        let manifest = forge_runtime_manifest();
        let route_ids: Vec<_> = manifest.routes.iter().map(|route| route.route_id.as_str()).collect();
        assert_eq!(route_ids, vec![FORGE_GIT_ROUTE_ID, FORGE_REPO_ROUTE_ID, FORGE_HEALTH_ROUTE_ID]);
        assert!(manifest.routes.iter().all(|route| route.owner_unit == FORGE_RUNTIME_SERVICE_NAME));
        assert!(manifest.routes.iter().any(|route| route.protocol == "iroh-alpn"));
        assert!(manifest.routes.iter().any(|route| route.protocol == "runtime-health"));
    }

    #[test]
    fn forge_runtime_receipts_are_secret_safe() {
        let health = forge_health_receipt(RuntimeHealthState::Healthy);
        let lifecycle = forge_lifecycle_receipt(RuntimeLifecycleStatus::Running);
        admit_receipt(&health).unwrap();
        admit_receipt(&lifecycle).unwrap();
        assert!(!health.contains_raw_secret());
        assert!(!lifecycle.contains_raw_secret());
        assert!(health.diagnostics.iter().all(|diagnostic| !matches!(diagnostic.value, RedactedValue::Plain(_))));
        assert_eq!(health.granted_authority, vec!["forge.kv".to_string(), "forge.blob".to_string()]);
    }
}
