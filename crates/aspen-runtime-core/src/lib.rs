//! Portable runtime host-loading model types.
//!
//! This crate intentionally defines data contracts and pure validation helpers
//! only. Host implementations that touch processes, WASM engines, Hyperlight,
//! OCI runtimes, microVMs, filesystems, network, or cryptographic verification
//! belong in runtime shell crates.

use serde::Deserialize;
use serde::Serialize;

/// Runtime host boundary selected before a unit is resolved or started.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "detail")]
pub enum RuntimeHostKind {
    /// First-party Rust service linked into `aspen-node`.
    NativeBuiltIn,
    /// Trusted operator-installed native binary launched out-of-process.
    NativeProcess,
    /// WASM module executed with bounded host functions/resources.
    Wasm,
    /// Hyperlight-backed isolated native-ish execution boundary.
    Hyperlight,
    /// OCI/container runner. Useful for packaging; not a strong boundary alone.
    OciContainer,
    /// Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm, or equivalent guest boundary.
    MicroVm { engine: MicroVmEngine },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicroVmEngine {
    Firecracker,
    CloudHypervisor,
    /// Hermit-focused minimal hypervisor for sealed unikernel guests.
    Uhyve,
    /// QEMU's minimalist microvm machine type or QEMU+loader development path.
    QemuMicrovm,
    Other(String),
}

/// Executable artifact identity. This is separate from host boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum RuntimeArtifact {
    BuiltIn {
        name: String,
        version: String,
    },
    NativeBinary {
        hash: String,
        store_path: Option<String>,
        entrypoint: String,
    },
    WasmModule {
        module_hash: String,
        abi: String,
        entrypoint: String,
    },
    HyperlightImage {
        image_hash: String,
        entrypoint: String,
    },
    OciImage {
        image_digest: String,
        entrypoint: String,
        args: Vec<String>,
    },
    LinuxGuest {
        kernel_hash: String,
        initrd_hash: Option<String>,
        rootfs_hash: String,
    },
    Unikernel {
        unikernel_kind: UnikernelKind,
        image_hash: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnikernelKind {
    HermitOs,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeUnitKind {
    Service,
    ExecutionRun,
    WorkflowActivity,
    Hook,
    Adapter,
}

/// UCAN-shaped capability binding without making this portable crate depend on
/// UCAN shell behavior or raw credentials.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCapabilityBinding {
    pub handle_id: String,
    pub ability: String,
    pub resource: String,
    pub proof_refs: Vec<String>,
    pub caveats: Vec<RuntimeCaveat>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCaveat {
    pub name: String,
    pub value_shape: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct RuntimeResources {
    pub memory_bytes: Option<u64>,
    pub cpu_millis: Option<u64>,
    pub wall_time_ms: Option<u64>,
    pub wasm_fuel: Option<u64>,
    pub max_open_files: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRouteDeclaration {
    pub route_id: String,
    pub protocol: String,
    pub owner_unit: String,
    pub handler: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeUnitDeclaration {
    pub unit_id: String,
    pub unit_kind: RuntimeUnitKind,
    pub host_kind: RuntimeHostKind,
    pub artifact: RuntimeArtifact,
    pub capabilities: Vec<RuntimeCapabilityBinding>,
    pub resources: RuntimeResources,
    pub routes: Vec<RuntimeRouteDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeLifecycleStatus {
    Declared,
    Resolving,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReceipt {
    pub receipt_id: String,
    pub unit_id: String,
    pub host_kind: RuntimeHostKind,
    pub lifecycle_status: RuntimeLifecycleStatus,
    pub artifact_summary: String,
    pub granted_authority: Vec<String>,
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDiagnostic {
    pub key: String,
    pub value: RedactedValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum RedactedValue {
    Plain(String),
    Redacted,
    OpaqueHandle(String),
    Hash(String),
}

impl RuntimeReceipt {
    #[must_use]
    pub fn contains_raw_secret(&self) -> bool {
        self.diagnostics.iter().any(|d| d.value.looks_secret())
    }
}

impl RedactedValue {
    #[must_use]
    pub fn looks_secret(&self) -> bool {
        match self {
            Self::Plain(value) => looks_like_secret(value),
            Self::Redacted | Self::OpaqueHandle(_) | Self::Hash(_) => false,
        }
    }
}

#[must_use]
pub fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("token=")
        || lower.contains("private_key")
        || lower.contains("cluster_cookie")
        || lower.contains("connection_string")
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeServiceManifest {
    pub name: String,
    pub version: String,
    pub routes: Vec<RuntimeRouteDeclaration>,
    pub required_capabilities: Vec<RuntimeCapabilityBinding>,
}

/// Pure registry entry for linked first-party services.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeBuiltInServiceFactory {
    pub service_name: String,
    pub manifest: NativeServiceManifest,
}

impl NativeBuiltInServiceFactory {
    #[must_use]
    pub fn as_declaration(&self, unit_id: impl Into<String>) -> RuntimeUnitDeclaration {
        RuntimeUnitDeclaration {
            unit_id: unit_id.into(),
            unit_kind: RuntimeUnitKind::Service,
            host_kind: RuntimeHostKind::NativeBuiltIn,
            artifact: RuntimeArtifact::BuiltIn {
                name: self.service_name.clone(),
                version: self.manifest.version.clone(),
            },
            capabilities: self.manifest.required_capabilities.clone(),
            resources: RuntimeResources::default(),
            routes: self.manifest.routes.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    EmptyUnitId,
    NativeBuiltInRequiresBuiltInArtifact,
    OciRequiresDigest,
    MicroVmRequiresGuestArtifact,
    ReceiptContainsRawSecret,
}

pub fn admit_unit(decl: &RuntimeUnitDeclaration) -> Result<(), AdmissionError> {
    if decl.unit_id.trim().is_empty() {
        return Err(AdmissionError::EmptyUnitId);
    }
    match (&decl.host_kind, &decl.artifact) {
        (RuntimeHostKind::NativeBuiltIn, RuntimeArtifact::BuiltIn { .. }) => {}
        (RuntimeHostKind::NativeBuiltIn, _) => return Err(AdmissionError::NativeBuiltInRequiresBuiltInArtifact),
        (RuntimeHostKind::OciContainer, RuntimeArtifact::OciImage { image_digest, .. })
            if image_digest.starts_with("sha256:") => {}
        (RuntimeHostKind::OciContainer, RuntimeArtifact::OciImage { .. }) => {
            return Err(AdmissionError::OciRequiresDigest);
        }
        (RuntimeHostKind::MicroVm { .. }, RuntimeArtifact::LinuxGuest { .. } | RuntimeArtifact::Unikernel { .. }) => {}
        (RuntimeHostKind::MicroVm { .. }, _) => return Err(AdmissionError::MicroVmRequiresGuestArtifact),
        _ => {}
    }
    Ok(())
}

pub fn admit_receipt(receipt: &RuntimeReceipt) -> Result<(), AdmissionError> {
    if receipt.contains_raw_secret() {
        return Err(AdmissionError::ReceiptContainsRawSecret);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forge_manifest() -> NativeServiceManifest {
        NativeServiceManifest {
            name: "forge".to_string(),
            version: "0.1.0".to_string(),
            routes: vec![RuntimeRouteDeclaration {
                route_id: "forge.git".to_string(),
                protocol: "iroh-alpn".to_string(),
                owner_unit: "forge".to_string(),
                handler: "forge-rpc".to_string(),
            }],
            required_capabilities: vec![RuntimeCapabilityBinding {
                handle_id: "kv:forge".to_string(),
                ability: "store/write".to_string(),
                resource: "aspen://kv/forge".to_string(),
                proof_refs: vec!["ucan-proof:operator".to_string()],
                caveats: vec![RuntimeCaveat {
                    name: "scope".to_string(),
                    value_shape: "repo-prefix".to_string(),
                }],
            }],
        }
    }

    #[test]
    fn native_factory_wraps_forge_as_builtin() {
        let factory = NativeBuiltInServiceFactory {
            service_name: "forge".to_string(),
            manifest: forge_manifest(),
        };
        let decl = factory.as_declaration("service/forge");
        assert_eq!(decl.host_kind, RuntimeHostKind::NativeBuiltIn);
        assert!(matches!(decl.artifact, RuntimeArtifact::BuiltIn { ref name, .. } if name == "forge"));
        assert_eq!(decl.routes.len(), 1);
        admit_unit(&decl).unwrap();
    }

    #[test]
    fn serialization_roundtrip_covers_host_taxonomy() {
        let decl = RuntimeUnitDeclaration {
            unit_id: "run/build-1".to_string(),
            unit_kind: RuntimeUnitKind::ExecutionRun,
            host_kind: RuntimeHostKind::MicroVm {
                engine: MicroVmEngine::Uhyve,
            },
            artifact: RuntimeArtifact::Unikernel {
                unikernel_kind: UnikernelKind::HermitOs,
                image_hash: "sha256:abc".to_string(),
            },
            capabilities: vec![],
            resources: RuntimeResources {
                memory_bytes: Some(512 * 1024 * 1024),
                cpu_millis: Some(1000),
                wall_time_ms: Some(30_000),
                wasm_fuel: None,
                max_open_files: Some(32),
            },
            routes: vec![],
        };
        let encoded = serde_json::to_string(&decl).unwrap();
        let decoded: RuntimeUnitDeclaration = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, decl);
        admit_unit(&decoded).unwrap();
    }

    #[test]
    fn admission_rejects_unsafe_shapes() {
        let bad_native = RuntimeUnitDeclaration {
            unit_id: "svc/bad".to_string(),
            unit_kind: RuntimeUnitKind::Service,
            host_kind: RuntimeHostKind::NativeBuiltIn,
            artifact: RuntimeArtifact::NativeBinary {
                hash: "sha256:abc".to_string(),
                store_path: None,
                entrypoint: "run".to_string(),
            },
            capabilities: vec![],
            resources: RuntimeResources::default(),
            routes: vec![],
        };
        assert_eq!(admit_unit(&bad_native), Err(AdmissionError::NativeBuiltInRequiresBuiltInArtifact));

        let bad_oci = RuntimeUnitDeclaration {
            host_kind: RuntimeHostKind::OciContainer,
            artifact: RuntimeArtifact::OciImage {
                image_digest: "latest".to_string(),
                entrypoint: "/bin/app".to_string(),
                args: vec![],
            },
            ..bad_native.clone()
        };
        assert_eq!(admit_unit(&bad_oci), Err(AdmissionError::OciRequiresDigest));
    }

    #[test]
    fn receipts_redact_raw_secrets() {
        let safe = RuntimeReceipt {
            receipt_id: "r1".to_string(),
            unit_id: "svc/forge".to_string(),
            host_kind: RuntimeHostKind::NativeBuiltIn,
            lifecycle_status: RuntimeLifecycleStatus::Running,
            artifact_summary: "built-in:forge@0.1.0".to_string(),
            granted_authority: vec!["kv:forge".to_string()],
            diagnostics: vec![RuntimeDiagnostic {
                key: "ticket".to_string(),
                value: RedactedValue::Redacted,
            }],
        };
        admit_receipt(&safe).unwrap();

        let unsafe_receipt = RuntimeReceipt {
            diagnostics: vec![RuntimeDiagnostic {
                key: "env".to_string(),
                value: RedactedValue::Plain("token=abc123".to_string()),
            }],
            ..safe
        };
        assert_eq!(admit_receipt(&unsafe_receipt), Err(AdmissionError::ReceiptContainsRawSecret));
    }
}
