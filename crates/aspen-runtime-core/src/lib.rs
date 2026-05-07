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

/// Minimal application/service ownership reference used before full app
/// install/upgrade semantics exist.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeApplicationRef {
    pub application_id: String,
    pub service_id: String,
    pub generation: u64,
    pub route_namespace: String,
    pub receipt_owner: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct RuntimePlacementHints {
    pub preferred_node_ids: Vec<String>,
    pub required_labels: Vec<String>,
    pub avoid_node_ids: Vec<String>,
    pub requires_persistent_storage: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeHealthPolicy {
    pub check_interval_ms: u64,
    pub timeout_ms: u64,
    pub unhealthy_threshold: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeRestartPolicy {
    pub max_restarts: u16,
    pub window_ms: u64,
    pub backoff_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeUpgradePolicy {
    pub max_unavailable: u16,
    pub allow_downgrade: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReceiptPolicy {
    pub emit_lifecycle: bool,
    pub emit_routes: bool,
    pub emit_health: bool,
    pub redact_diagnostics: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeServiceSpec {
    pub ownership: RuntimeApplicationRef,
    pub host_kind: RuntimeHostKind,
    pub artifact: RuntimeArtifact,
    pub desired_replicas: u16,
    pub singleton: bool,
    pub placement: RuntimePlacementHints,
    pub resources: RuntimeResources,
    pub capabilities: Vec<RuntimeCapabilityBinding>,
    pub routes: Vec<RuntimeRouteDeclaration>,
    pub health_policy: RuntimeHealthPolicy,
    pub restart_policy: RuntimeRestartPolicy,
    pub upgrade_policy: RuntimeUpgradePolicy,
    pub receipt_policy: RuntimeReceiptPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeHealthState {
    Unknown,
    Starting,
    Healthy,
    Degraded,
    Unhealthy,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeServiceInstance {
    pub ownership: RuntimeApplicationRef,
    pub instance_id: String,
    pub assigned_node_id: Option<String>,
    pub lifecycle_status: RuntimeLifecycleStatus,
    pub health_state: RuntimeHealthState,
    pub lease_epoch: u64,
    pub heartbeat_ms: Option<u64>,
    pub active_routes: Vec<String>,
    pub last_receipt_id: Option<String>,
}

impl RuntimeServiceSpec {
    #[must_use]
    pub fn as_unit_declaration(&self) -> RuntimeUnitDeclaration {
        RuntimeUnitDeclaration {
            unit_id: self.ownership.service_id.clone(),
            unit_kind: RuntimeUnitKind::Service,
            host_kind: self.host_kind.clone(),
            artifact: self.artifact.clone(),
            capabilities: self.capabilities.clone(),
            resources: self.resources.clone(),
            routes: self.routes.clone(),
        }
    }
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
#[serde(rename_all = "kebab-case")]
pub enum SponsoredPrincipalRole {
    Sponsor,
    Beneficiary,
    Provider,
    Workload,
    Service,
    Organization,
    User,
    Node,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredPrincipalRef {
    pub principal_id: String,
    pub role: SponsoredPrincipalRole,
    pub proof_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredNodePrincipalRef {
    pub node_id: String,
    pub principal: SponsoredPrincipalRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredPluginPrincipalRef {
    pub plugin_id: String,
    pub principal: SponsoredPrincipalRef,
    pub plugin_family: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredSettlementReference {
    pub method_tag: String,
    pub opaque_ref: RedactedValue,
}

impl SponsoredSettlementReference {
    #[must_use]
    pub fn contains_raw_secret(&self) -> bool {
        self.opaque_ref.looks_secret()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredResourceLimits {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub storage_bytes_ms: u64,
    pub network_bytes: u64,
    pub wall_time_ms: u64,
    pub max_concurrent: u16,
}

impl SponsoredResourceLimits {
    #[must_use]
    pub fn fits_within(&self, available: &Self) -> bool {
        self.cpu_millis <= available.cpu_millis
            && self.memory_bytes <= available.memory_bytes
            && self.storage_bytes_ms <= available.storage_bytes_ms
            && self.network_bytes <= available.network_bytes
            && self.wall_time_ms <= available.wall_time_ms
            && self.max_concurrent <= available.max_concurrent
    }

    #[must_use]
    pub fn checked_sub(&self, used: &Self) -> Option<Self> {
        Some(Self {
            cpu_millis: self.cpu_millis.checked_sub(used.cpu_millis)?,
            memory_bytes: self.memory_bytes.checked_sub(used.memory_bytes)?,
            storage_bytes_ms: self.storage_bytes_ms.checked_sub(used.storage_bytes_ms)?,
            network_bytes: self.network_bytes.checked_sub(used.network_bytes)?,
            wall_time_ms: self.wall_time_ms.checked_sub(used.wall_time_ms)?,
            max_concurrent: self.max_concurrent.checked_sub(used.max_concurrent)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredGrantScope {
    pub workload_principal_ids: Vec<String>,
    pub service_principal_ids: Vec<String>,
    pub provider_principal_ids: Vec<String>,
    pub node_principal_ids: Vec<String>,
    pub plugin_principal_ids: Vec<String>,
    pub resource_classes: Vec<String>,
    pub isolation_modes: Vec<RuntimeHostKind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredRevocationRef {
    pub revocation_id: String,
    pub revoked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredRuntimeGrant {
    pub grant_id: String,
    pub sponsor: SponsoredPrincipalRef,
    pub beneficiary: SponsoredPrincipalRef,
    pub provider_scope: Vec<SponsoredPrincipalRef>,
    pub workload_scope: SponsoredGrantScope,
    pub limits: SponsoredResourceLimits,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
    pub revocation: SponsoredRevocationRef,
    pub settlement: SponsoredSettlementReference,
    pub policy_tags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredQuotaReservation {
    pub reservation_id: String,
    pub grant_id: String,
    pub workload_id: String,
    pub reserved: SponsoredResourceLimits,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredQuotaConsumption {
    pub consumption_id: String,
    pub reservation_id: String,
    pub consumed: SponsoredResourceLimits,
    pub released: SponsoredResourceLimits,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredQuotaLedger {
    pub grant_id: String,
    pub total: SponsoredResourceLimits,
    pub reserved: SponsoredResourceLimits,
    pub consumed: SponsoredResourceLimits,
    pub active_concurrency: u16,
}

impl SponsoredQuotaLedger {
    #[must_use]
    pub fn remaining(&self) -> Option<SponsoredResourceLimits> {
        let accounted = SponsoredResourceLimits {
            cpu_millis: self.reserved.cpu_millis.checked_add(self.consumed.cpu_millis)?,
            memory_bytes: self.reserved.memory_bytes.checked_add(self.consumed.memory_bytes)?,
            storage_bytes_ms: self.reserved.storage_bytes_ms.checked_add(self.consumed.storage_bytes_ms)?,
            network_bytes: self.reserved.network_bytes.checked_add(self.consumed.network_bytes)?,
            wall_time_ms: self.reserved.wall_time_ms.checked_add(self.consumed.wall_time_ms)?,
            max_concurrent: self.active_concurrency,
        };
        self.total.checked_sub(&accounted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SponsoredReceiptOutcome {
    Started,
    Reserved,
    Consumed,
    Completed,
    Failed,
    RevocationDenied,
    QuotaDenied,
    PolicyDenied,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredUsageReceipt {
    pub schema: String,
    pub receipt_id: String,
    pub execution_id: String,
    pub workload_principal_id: String,
    pub service_principal_id: Option<String>,
    pub provider_principal_id: String,
    pub sponsor_principal_id: String,
    pub grant_id: String,
    pub measured: SponsoredResourceLimits,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub outcome: SponsoredReceiptOutcome,
    pub artifact_refs: Vec<String>,
    pub isolation_summary: String,
    pub settlement: SponsoredSettlementReference,
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

impl SponsoredUsageReceipt {
    #[must_use]
    pub fn contains_raw_secret(&self) -> bool {
        self.settlement.contains_raw_secret() || self.diagnostics.iter().any(|d| d.value.looks_secret())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SignedSponsoredUsageReceipt {
    pub receipt: SponsoredUsageReceipt,
    pub signer_principal_id: String,
    pub signature_ref: RedactedValue,
}

impl SignedSponsoredUsageReceipt {
    #[must_use]
    pub fn contains_raw_secret(&self) -> bool {
        self.receipt.contains_raw_secret() || self.signature_ref.looks_secret()
    }
}

pub struct SponsoredUsageReceiptInput {
    pub receipt_id: String,
    pub execution_id: String,
    pub workload_principal_id: String,
    pub service_principal_id: Option<String>,
    pub provider_principal_id: String,
    pub sponsor_principal_id: String,
    pub grant_id: String,
    pub measured: SponsoredResourceLimits,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub outcome: SponsoredReceiptOutcome,
    pub artifact_refs: Vec<String>,
    pub isolation_summary: String,
    pub settlement: SponsoredSettlementReference,
    pub diagnostics: Vec<RuntimeDiagnostic>,
    pub signer_principal_id: String,
    pub signature_ref: RedactedValue,
}

#[must_use]
pub fn signed_sponsored_usage_receipt(input: SponsoredUsageReceiptInput) -> SignedSponsoredUsageReceipt {
    SignedSponsoredUsageReceipt {
        receipt: SponsoredUsageReceipt {
            schema: "aspen.sponsored-usage-receipt.v1".to_string(),
            receipt_id: input.receipt_id,
            execution_id: input.execution_id,
            workload_principal_id: input.workload_principal_id,
            service_principal_id: input.service_principal_id,
            provider_principal_id: input.provider_principal_id,
            sponsor_principal_id: input.sponsor_principal_id,
            grant_id: input.grant_id,
            measured: input.measured,
            started_at_ms: input.started_at_ms,
            completed_at_ms: input.completed_at_ms,
            outcome: input.outcome,
            artifact_refs: input.artifact_refs,
            isolation_summary: input.isolation_summary,
            settlement: input.settlement,
            diagnostics: input.diagnostics,
        },
        signer_principal_id: input.signer_principal_id,
        signature_ref: input.signature_ref,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredProviderPolicy {
    pub provider_principal_id: String,
    pub accepted_sponsor_principal_ids: Vec<String>,
    pub accepted_workload_principal_ids: Vec<String>,
    pub accepted_service_principal_ids: Vec<String>,
    pub accepted_settlement_tags: Vec<String>,
    pub accepted_isolation_modes: Vec<RuntimeHostKind>,
    pub max_request: SponsoredResourceLimits,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredAdmissionRequest {
    pub grant: SponsoredRuntimeGrant,
    pub provider_policy: SponsoredProviderPolicy,
    pub ledger: SponsoredQuotaLedger,
    pub workload_principal_id: String,
    pub service_principal_id: Option<String>,
    pub provider_principal_id: String,
    pub isolation_mode: RuntimeHostKind,
    pub requested: SponsoredResourceLimits,
    pub now_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SponsoredAdmissionError {
    MissingPrincipalProof,
    ExpiredGrant,
    RevokedGrant,
    ProviderRejected,
    UnsupportedSettlementTag,
    QuotaExhausted,
    IsolationMismatch,
    ScopeMismatch,
    SecretBearingSettlementRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SponsoredPlacementSurface {
    RuntimeService,
    Job,
    CiRun,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SponsoredPlacementConstraint {
    pub surface: SponsoredPlacementSurface,
    pub unit_id: String,
    pub sponsorship_required: bool,
    pub request: Option<SponsoredAdmissionRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SponsoredPlacementError {
    MissingRequiredGrant,
    Rejected(SponsoredAdmissionError),
}

pub fn admit_sponsored_placement(constraint: &SponsoredPlacementConstraint) -> Result<(), SponsoredPlacementError> {
    match (constraint.sponsorship_required, constraint.request.as_ref()) {
        (false, None) => Ok(()),
        (_, Some(request)) => admit_sponsored_request(request).map_err(SponsoredPlacementError::Rejected),
        (true, None) => Err(SponsoredPlacementError::MissingRequiredGrant),
    }
}

pub fn admit_sponsored_request(request: &SponsoredAdmissionRequest) -> Result<(), SponsoredAdmissionError> {
    let grant = &request.grant;
    if grant.sponsor.proof_ref.as_deref().unwrap_or_default().trim().is_empty()
        || grant.beneficiary.proof_ref.as_deref().unwrap_or_default().trim().is_empty()
        || grant
            .provider_scope
            .iter()
            .any(|principal| principal.proof_ref.as_deref().unwrap_or_default().trim().is_empty())
    {
        return Err(SponsoredAdmissionError::MissingPrincipalProof);
    }
    if request.now_ms < grant.valid_from_ms || request.now_ms >= grant.valid_until_ms {
        return Err(SponsoredAdmissionError::ExpiredGrant);
    }
    if grant.revocation.revoked {
        return Err(SponsoredAdmissionError::RevokedGrant);
    }
    if grant.settlement.contains_raw_secret() {
        return Err(SponsoredAdmissionError::SecretBearingSettlementRef);
    }
    if !grant.workload_scope.provider_principal_ids.contains(&request.provider_principal_id)
        || !grant.provider_scope.iter().any(|principal| principal.principal_id == request.provider_principal_id)
        || request.provider_policy.provider_principal_id != request.provider_principal_id
        || !request.provider_policy.accepted_sponsor_principal_ids.contains(&grant.sponsor.principal_id)
    {
        return Err(SponsoredAdmissionError::ProviderRejected);
    }
    if !request.provider_policy.accepted_settlement_tags.contains(&grant.settlement.method_tag) {
        return Err(SponsoredAdmissionError::UnsupportedSettlementTag);
    }
    if !grant.workload_scope.isolation_modes.contains(&request.isolation_mode)
        || !request.provider_policy.accepted_isolation_modes.contains(&request.isolation_mode)
    {
        return Err(SponsoredAdmissionError::IsolationMismatch);
    }
    if !grant.workload_scope.workload_principal_ids.contains(&request.workload_principal_id)
        || !request.provider_policy.accepted_workload_principal_ids.contains(&request.workload_principal_id)
        || request.service_principal_id.as_ref().is_some_and(|service| {
            !grant.workload_scope.service_principal_ids.contains(service)
                || !request.provider_policy.accepted_service_principal_ids.contains(service)
        })
    {
        return Err(SponsoredAdmissionError::ScopeMismatch);
    }
    if !request.requested.fits_within(&grant.limits)
        || !request.requested.fits_within(&request.provider_policy.max_request)
    {
        return Err(SponsoredAdmissionError::QuotaExhausted);
    }
    let Some(remaining) = request.ledger.remaining() else {
        return Err(SponsoredAdmissionError::QuotaExhausted);
    };
    if !request.requested.fits_within(&remaining) {
        return Err(SponsoredAdmissionError::QuotaExhausted);
    }
    Ok(())
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

impl NativeServiceManifest {
    #[must_use]
    pub fn capability_handle_refs(&self) -> Vec<RedactedValue> {
        self.required_capabilities
            .iter()
            .map(|binding| RedactedValue::OpaqueHandle(binding.handle_id.clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeLoadingPolicy {
    /// The service is linked into the Aspen binary and selected from a static registry.
    LinkedBuiltInOnly,
}

/// Pure registry entry for linked first-party services.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeBuiltInServiceFactory {
    pub service_name: String,
    pub linked_symbol: String,
    pub loading_policy: NativeLoadingPolicy,
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
    EmptyServiceId,
    DesiredReplicasRequired,
    SingletonRequiresOneReplica,
    RouteOwnerMismatch,
    InvalidHealthPolicy,
    InvalidRestartPolicy,
    InvalidUpgradePolicy,
    InvalidLifecycleTransition,
    NativeFactoryNameMismatch,
    NativeFactoryMissingLinkedSymbol,
}

pub fn admit_native_factory(factory: &NativeBuiltInServiceFactory) -> Result<(), AdmissionError> {
    if factory.service_name != factory.manifest.name {
        return Err(AdmissionError::NativeFactoryNameMismatch);
    }
    if factory.linked_symbol.trim().is_empty() {
        return Err(AdmissionError::NativeFactoryMissingLinkedSymbol);
    }
    match factory.loading_policy {
        NativeLoadingPolicy::LinkedBuiltInOnly => Ok(()),
    }
}

pub fn admit_service_spec(spec: &RuntimeServiceSpec) -> Result<(), AdmissionError> {
    if spec.ownership.service_id.trim().is_empty() {
        return Err(AdmissionError::EmptyServiceId);
    }
    if spec.desired_replicas == 0 {
        return Err(AdmissionError::DesiredReplicasRequired);
    }
    if spec.singleton && spec.desired_replicas != 1 {
        return Err(AdmissionError::SingletonRequiresOneReplica);
    }
    if spec.routes.iter().any(|route| route.owner_unit != spec.ownership.service_id) {
        return Err(AdmissionError::RouteOwnerMismatch);
    }
    if spec.health_policy.check_interval_ms == 0
        || spec.health_policy.timeout_ms == 0
        || spec.health_policy.timeout_ms > spec.health_policy.check_interval_ms
        || spec.health_policy.unhealthy_threshold == 0
    {
        return Err(AdmissionError::InvalidHealthPolicy);
    }
    if spec.restart_policy.window_ms == 0 || spec.restart_policy.backoff_ms == 0 {
        return Err(AdmissionError::InvalidRestartPolicy);
    }
    if spec.upgrade_policy.max_unavailable > spec.desired_replicas {
        return Err(AdmissionError::InvalidUpgradePolicy);
    }
    admit_unit(&spec.as_unit_declaration())
}

pub fn admit_lifecycle_transition(
    from: RuntimeLifecycleStatus,
    to: RuntimeLifecycleStatus,
) -> Result<(), AdmissionError> {
    let allowed = matches!(
        (from, to),
        (RuntimeLifecycleStatus::Declared, RuntimeLifecycleStatus::Resolving)
            | (RuntimeLifecycleStatus::Resolving, RuntimeLifecycleStatus::Starting)
            | (RuntimeLifecycleStatus::Starting, RuntimeLifecycleStatus::Running)
            | (RuntimeLifecycleStatus::Running, RuntimeLifecycleStatus::Stopping)
            | (RuntimeLifecycleStatus::Stopping, RuntimeLifecycleStatus::Stopped)
            | (_, RuntimeLifecycleStatus::Failed)
    );
    if allowed {
        Ok(())
    } else {
        Err(AdmissionError::InvalidLifecycleTransition)
    }
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

    fn forge_service_spec() -> RuntimeServiceSpec {
        let manifest = forge_manifest();
        RuntimeServiceSpec {
            ownership: RuntimeApplicationRef {
                application_id: "aspen-system".to_string(),
                service_id: "forge".to_string(),
                generation: 7,
                route_namespace: "system".to_string(),
                receipt_owner: "operator".to_string(),
            },
            host_kind: RuntimeHostKind::NativeBuiltIn,
            artifact: RuntimeArtifact::BuiltIn {
                name: manifest.name,
                version: manifest.version,
            },
            desired_replicas: 1,
            singleton: true,
            placement: RuntimePlacementHints {
                preferred_node_ids: vec!["node-a".to_string()],
                required_labels: vec!["ssd".to_string()],
                avoid_node_ids: vec![],
                requires_persistent_storage: true,
            },
            resources: RuntimeResources {
                memory_bytes: Some(256 * 1024 * 1024),
                cpu_millis: Some(500),
                wall_time_ms: None,
                wasm_fuel: None,
                max_open_files: Some(64),
            },
            capabilities: manifest.required_capabilities,
            routes: manifest.routes,
            health_policy: RuntimeHealthPolicy {
                check_interval_ms: 10_000,
                timeout_ms: 1_000,
                unhealthy_threshold: 3,
            },
            restart_policy: RuntimeRestartPolicy {
                max_restarts: 5,
                window_ms: 60_000,
                backoff_ms: 1_000,
            },
            upgrade_policy: RuntimeUpgradePolicy {
                max_unavailable: 0,
                allow_downgrade: false,
            },
            receipt_policy: RuntimeReceiptPolicy {
                emit_lifecycle: true,
                emit_routes: true,
                emit_health: true,
                redact_diagnostics: true,
            },
        }
    }

    #[test]
    fn native_factory_wraps_forge_as_builtin() {
        let factory = NativeBuiltInServiceFactory {
            service_name: "forge".to_string(),
            linked_symbol: "aspen_forge::runtime_service_factory".to_string(),
            loading_policy: NativeLoadingPolicy::LinkedBuiltInOnly,
            manifest: forge_manifest(),
        };
        let decl = factory.as_declaration("service/forge");
        admit_native_factory(&factory).unwrap();
        assert_eq!(factory.loading_policy, NativeLoadingPolicy::LinkedBuiltInOnly);
        assert!(factory.linked_symbol.contains("runtime_service_factory"));
        assert_eq!(decl.host_kind, RuntimeHostKind::NativeBuiltIn);
        assert!(matches!(decl.artifact, RuntimeArtifact::BuiltIn { ref name, .. } if name == "forge"));
        assert_eq!(decl.routes.len(), 1);
        admit_unit(&decl).unwrap();

        let mut bad_factory = factory;
        bad_factory.linked_symbol.clear();
        assert_eq!(admit_native_factory(&bad_factory), Err(AdmissionError::NativeFactoryMissingLinkedSymbol));
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
    fn built_in_declarations_use_native_host_and_redacted_capability_handles() {
        let manifest = forge_manifest();
        let handles = manifest.capability_handle_refs();
        assert_eq!(handles, vec![RedactedValue::OpaqueHandle("kv:forge".to_string())]);
        assert!(handles.iter().all(|handle| !handle.looks_secret()));
        assert!(handles.iter().all(|handle| !matches!(handle, RedactedValue::Plain(_))));

        let factory = NativeBuiltInServiceFactory {
            service_name: "forge".to_string(),
            linked_symbol: "aspen_forge::runtime_service_factory".to_string(),
            loading_policy: NativeLoadingPolicy::LinkedBuiltInOnly,
            manifest,
        };
        let decl = factory.as_declaration("forge");
        assert_eq!(decl.host_kind, RuntimeHostKind::NativeBuiltIn);
        assert!(matches!(decl.artifact, RuntimeArtifact::BuiltIn { .. }));
        assert_eq!(decl.capabilities[0].handle_id, "kv:forge");
        assert_eq!(decl.capabilities[0].proof_refs, vec!["ucan-proof:operator".to_string()]);
    }

    #[test]
    fn service_model_invariants_cover_identity_routes_policies_and_host_boundary() {
        let spec = forge_service_spec();
        admit_service_spec(&spec).unwrap();
        let unit = spec.as_unit_declaration();
        assert_eq!(unit.unit_id, "forge");
        assert_eq!(unit.routes[0].owner_unit, spec.ownership.service_id);

        let mut empty_id = spec.clone();
        empty_id.ownership.service_id = " ".to_string();
        assert_eq!(admit_service_spec(&empty_id), Err(AdmissionError::EmptyServiceId));

        let mut route_mismatch = spec.clone();
        route_mismatch.routes[0].owner_unit = "ci".to_string();
        assert_eq!(admit_service_spec(&route_mismatch), Err(AdmissionError::RouteOwnerMismatch));

        let mut bad_singleton = spec.clone();
        bad_singleton.desired_replicas = 2;
        assert_eq!(admit_service_spec(&bad_singleton), Err(AdmissionError::SingletonRequiresOneReplica));

        let mut bad_health = spec.clone();
        bad_health.health_policy.timeout_ms = 20_000;
        assert_eq!(admit_service_spec(&bad_health), Err(AdmissionError::InvalidHealthPolicy));

        let mut bad_restart = spec.clone();
        bad_restart.restart_policy.backoff_ms = 0;
        assert_eq!(admit_service_spec(&bad_restart), Err(AdmissionError::InvalidRestartPolicy));

        let mut bad_upgrade = spec.clone();
        bad_upgrade.upgrade_policy.max_unavailable = 2;
        assert_eq!(admit_service_spec(&bad_upgrade), Err(AdmissionError::InvalidUpgradePolicy));

        let mut bad_host_artifact = spec;
        bad_host_artifact.artifact = RuntimeArtifact::NativeBinary {
            hash: "sha256:abc".to_string(),
            store_path: None,
            entrypoint: "run".to_string(),
        };
        assert_eq!(admit_service_spec(&bad_host_artifact), Err(AdmissionError::NativeBuiltInRequiresBuiltInArtifact));
    }

    #[test]
    fn lifecycle_transitions_and_health_receipts_are_bounded() {
        admit_lifecycle_transition(RuntimeLifecycleStatus::Declared, RuntimeLifecycleStatus::Resolving).unwrap();
        admit_lifecycle_transition(RuntimeLifecycleStatus::Resolving, RuntimeLifecycleStatus::Starting).unwrap();
        admit_lifecycle_transition(RuntimeLifecycleStatus::Starting, RuntimeLifecycleStatus::Running).unwrap();
        admit_lifecycle_transition(RuntimeLifecycleStatus::Running, RuntimeLifecycleStatus::Failed).unwrap();
        assert_eq!(
            admit_lifecycle_transition(RuntimeLifecycleStatus::Declared, RuntimeLifecycleStatus::Running),
            Err(AdmissionError::InvalidLifecycleTransition)
        );

        let spec = forge_service_spec();
        let instance = RuntimeServiceInstance {
            ownership: spec.ownership.clone(),
            instance_id: "forge/0".to_string(),
            assigned_node_id: Some("node-a".to_string()),
            lifecycle_status: RuntimeLifecycleStatus::Running,
            health_state: RuntimeHealthState::Healthy,
            lease_epoch: 1,
            heartbeat_ms: Some(42_000),
            active_routes: spec.routes.iter().map(|route| route.route_id.clone()).collect(),
            last_receipt_id: Some("receipt-1".to_string()),
        };
        assert_eq!(instance.health_state, RuntimeHealthState::Healthy);
        assert_eq!(instance.active_routes, vec!["forge.git".to_string()]);

        let health_receipt = RuntimeReceipt {
            receipt_id: "receipt-1".to_string(),
            unit_id: instance.ownership.service_id,
            host_kind: RuntimeHostKind::NativeBuiltIn,
            lifecycle_status: instance.lifecycle_status,
            artifact_summary: "built-in:forge@0.1.0".to_string(),
            granted_authority: vec!["kv:forge".to_string()],
            diagnostics: vec![RuntimeDiagnostic {
                key: "health".to_string(),
                value: RedactedValue::Plain("healthy".to_string()),
            }],
        };
        admit_receipt(&health_receipt).unwrap();
    }

    fn sponsored_limits() -> SponsoredResourceLimits {
        SponsoredResourceLimits {
            cpu_millis: 10_000,
            memory_bytes: 1024 * 1024 * 1024,
            storage_bytes_ms: 50_000,
            network_bytes: 1_000_000,
            wall_time_ms: 60_000,
            max_concurrent: 2,
        }
    }

    fn sponsored_grant() -> SponsoredRuntimeGrant {
        SponsoredRuntimeGrant {
            grant_id: "grant-open-source-ci".to_string(),
            sponsor: SponsoredPrincipalRef {
                principal_id: "org/aspen-foundation".to_string(),
                role: SponsoredPrincipalRole::Sponsor,
                proof_ref: Some("ucan:proof:sponsor".to_string()),
            },
            beneficiary: SponsoredPrincipalRef {
                principal_id: "workload/aspen-ci".to_string(),
                role: SponsoredPrincipalRole::Beneficiary,
                proof_ref: Some("ucan:proof:beneficiary".to_string()),
            },
            provider_scope: vec![SponsoredPrincipalRef {
                principal_id: "provider/nodepool-a".to_string(),
                role: SponsoredPrincipalRole::Provider,
                proof_ref: Some("ucan:proof:provider".to_string()),
            }],
            workload_scope: SponsoredGrantScope {
                workload_principal_ids: vec!["workload/aspen-ci".to_string()],
                service_principal_ids: vec!["service/forge".to_string()],
                provider_principal_ids: vec!["provider/nodepool-a".to_string()],
                node_principal_ids: vec!["node/n1".to_string()],
                plugin_principal_ids: vec!["plugin/internal-budget".to_string()],
                resource_classes: vec!["ci-small".to_string()],
                isolation_modes: vec![RuntimeHostKind::NativeBuiltIn],
            },
            limits: sponsored_limits(),
            valid_from_ms: 1_000,
            valid_until_ms: 10_000,
            revocation: SponsoredRevocationRef {
                revocation_id: "rev/grant-open-source-ci".to_string(),
                revoked: false,
            },
            settlement: SponsoredSettlementReference {
                method_tag: "none:internal-grant".to_string(),
                opaque_ref: RedactedValue::OpaqueHandle("settlement:internal:42".to_string()),
            },
            policy_tags: vec!["open-source-ci".to_string()],
        }
    }

    #[test]
    fn sponsored_grant_model_is_bounded_scoped_and_settlement_opaque() {
        let grant = sponsored_grant();
        assert!(grant.workload_scope.workload_principal_ids.contains(&"workload/aspen-ci".to_string()));
        assert!(grant.workload_scope.provider_principal_ids.contains(&"provider/nodepool-a".to_string()));
        assert!(grant.workload_scope.isolation_modes.contains(&RuntimeHostKind::NativeBuiltIn));
        assert!(!grant.settlement.contains_raw_secret());
        assert!(matches!(grant.settlement.opaque_ref, RedactedValue::OpaqueHandle(_)));
        assert_eq!(grant.valid_from_ms, 1_000);
        assert_eq!(grant.valid_until_ms, 10_000);

        let request = SponsoredResourceLimits {
            cpu_millis: 2_000,
            memory_bytes: 128 * 1024 * 1024,
            storage_bytes_ms: 10_000,
            network_bytes: 100_000,
            wall_time_ms: 30_000,
            max_concurrent: 1,
        };
        assert!(request.fits_within(&grant.limits));

        let too_large = SponsoredResourceLimits {
            memory_bytes: grant.limits.memory_bytes + 1,
            ..request.clone()
        };
        assert!(!too_large.fits_within(&grant.limits));
    }

    #[test]
    fn sponsored_quota_arithmetic_tracks_remaining_reservation_and_consumption() {
        let ledger = SponsoredQuotaLedger {
            grant_id: "grant-open-source-ci".to_string(),
            total: sponsored_limits(),
            reserved: SponsoredResourceLimits {
                cpu_millis: 1_000,
                memory_bytes: 128 * 1024 * 1024,
                storage_bytes_ms: 1_000,
                network_bytes: 10_000,
                wall_time_ms: 5_000,
                max_concurrent: 0,
            },
            consumed: SponsoredResourceLimits {
                cpu_millis: 2_000,
                memory_bytes: 256 * 1024 * 1024,
                storage_bytes_ms: 2_000,
                network_bytes: 20_000,
                wall_time_ms: 10_000,
                max_concurrent: 0,
            },
            active_concurrency: 1,
        };
        let remaining = ledger.remaining().unwrap();
        assert_eq!(remaining.cpu_millis, 7_000);
        assert_eq!(remaining.memory_bytes, 640 * 1024 * 1024);
        assert_eq!(remaining.max_concurrent, 1);

        let overdrawn = SponsoredQuotaLedger {
            reserved: SponsoredResourceLimits {
                cpu_millis: 20_000,
                ..ledger.reserved.clone()
            },
            ..ledger
        };
        assert!(overdrawn.remaining().is_none());
    }

    #[test]
    fn sponsored_rust_derived_dtos_serialize_roundtrip() {
        let grant = sponsored_grant();
        let grant_json = serde_json::to_string(&grant).unwrap();
        assert!(grant_json.contains("sponsor"));
        assert!(grant_json.contains("native-built-in"));
        let decoded_grant: SponsoredRuntimeGrant = serde_json::from_str(&grant_json).unwrap();
        assert_eq!(decoded_grant, grant);

        let ledger = SponsoredQuotaLedger {
            grant_id: "grant-open-source-ci".to_string(),
            total: sponsored_limits(),
            reserved: SponsoredResourceLimits {
                cpu_millis: 1_000,
                memory_bytes: 128 * 1024 * 1024,
                storage_bytes_ms: 1_000,
                network_bytes: 10_000,
                wall_time_ms: 5_000,
                max_concurrent: 0,
            },
            consumed: SponsoredResourceLimits {
                cpu_millis: 2_000,
                memory_bytes: 256 * 1024 * 1024,
                storage_bytes_ms: 2_000,
                network_bytes: 20_000,
                wall_time_ms: 10_000,
                max_concurrent: 0,
            },
            active_concurrency: 1,
        };
        let ledger_json = serde_json::to_string(&ledger).unwrap();
        let decoded_ledger: SponsoredQuotaLedger = serde_json::from_str(&ledger_json).unwrap();
        assert_eq!(decoded_ledger, ledger);

        let receipt = SponsoredUsageReceipt {
            schema: "aspen.sponsored-usage-receipt.v1".to_string(),
            receipt_id: "receipt/sponsored/roundtrip".to_string(),
            execution_id: "run/ci/roundtrip".to_string(),
            workload_principal_id: "workload/aspen-ci".to_string(),
            service_principal_id: Some("service/forge".to_string()),
            provider_principal_id: "provider/nodepool-a".to_string(),
            sponsor_principal_id: "org/aspen-foundation".to_string(),
            grant_id: "grant-open-source-ci".to_string(),
            measured: sponsored_limits(),
            started_at_ms: 2_000,
            completed_at_ms: Some(3_000),
            outcome: SponsoredReceiptOutcome::Completed,
            artifact_refs: vec!["blake3:artifact".to_string()],
            isolation_summary: "native-built-in".to_string(),
            settlement: SponsoredSettlementReference {
                method_tag: "voucher".to_string(),
                opaque_ref: RedactedValue::Redacted,
            },
            diagnostics: vec![RuntimeDiagnostic {
                key: "operator-note".to_string(),
                value: RedactedValue::Redacted,
            }],
        };
        let receipt_json = serde_json::to_string(&receipt).unwrap();
        assert!(receipt_json.contains("completed"));
        assert!(!receipt_json.contains("token="));
        let decoded_receipt: SponsoredUsageReceipt = serde_json::from_str(&receipt_json).unwrap();
        assert_eq!(decoded_receipt, receipt);
    }

    fn signed_receipt_for_outcome(outcome: SponsoredReceiptOutcome) -> SignedSponsoredUsageReceipt {
        signed_sponsored_usage_receipt(SponsoredUsageReceiptInput {
            receipt_id: format!("receipt/sponsored/{outcome:?}"),
            execution_id: "run/ci/1".to_string(),
            workload_principal_id: "workload/aspen-ci".to_string(),
            service_principal_id: Some("service/forge".to_string()),
            provider_principal_id: "provider/nodepool-a".to_string(),
            sponsor_principal_id: "org/aspen-foundation".to_string(),
            grant_id: "grant-open-source-ci".to_string(),
            measured: sponsored_limits(),
            started_at_ms: 2_000,
            completed_at_ms: Some(3_000),
            outcome,
            artifact_refs: vec!["blake3:artifact".to_string()],
            isolation_summary: "native-built-in".to_string(),
            settlement: SponsoredSettlementReference {
                method_tag: "voucher".to_string(),
                opaque_ref: RedactedValue::Redacted,
            },
            diagnostics: vec![RuntimeDiagnostic {
                key: "operator-note".to_string(),
                value: RedactedValue::Redacted,
            }],
            signer_principal_id: "provider/nodepool-a".to_string(),
            signature_ref: RedactedValue::Redacted,
        })
    }

    #[test]
    fn signed_sponsored_usage_receipts_cover_all_required_paths_and_redact_handles() {
        let required = [
            SponsoredReceiptOutcome::Started,
            SponsoredReceiptOutcome::Reserved,
            SponsoredReceiptOutcome::Consumed,
            SponsoredReceiptOutcome::Completed,
            SponsoredReceiptOutcome::Failed,
            SponsoredReceiptOutcome::RevocationDenied,
        ];
        for outcome in required {
            let signed = signed_receipt_for_outcome(outcome.clone());
            assert_eq!(signed.receipt.schema, "aspen.sponsored-usage-receipt.v1");
            assert_eq!(signed.receipt.outcome, outcome);
            assert_eq!(signed.signer_principal_id, "provider/nodepool-a");
            assert!(matches!(signed.signature_ref, RedactedValue::Redacted));
            assert!(!signed.contains_raw_secret());
        }

        let unsafe_signed = SignedSponsoredUsageReceipt {
            signature_ref: RedactedValue::Plain("token=raw-secret".to_string()),
            ..signed_receipt_for_outcome(SponsoredReceiptOutcome::Failed)
        };
        assert!(unsafe_signed.contains_raw_secret());
    }

    #[test]
    fn sponsored_usage_receipts_redact_settlement_and_diagnostics() {
        let grant = sponsored_grant();
        let receipt = SponsoredUsageReceipt {
            schema: "aspen.sponsored-usage-receipt.v1".to_string(),
            receipt_id: "receipt/sponsored/1".to_string(),
            execution_id: "run/ci/1".to_string(),
            workload_principal_id: grant.beneficiary.principal_id.clone(),
            service_principal_id: Some("service/forge".to_string()),
            provider_principal_id: "provider/nodepool-a".to_string(),
            sponsor_principal_id: grant.sponsor.principal_id.clone(),
            grant_id: grant.grant_id,
            measured: sponsored_limits(),
            started_at_ms: 2_000,
            completed_at_ms: Some(3_000),
            outcome: SponsoredReceiptOutcome::Completed,
            artifact_refs: vec!["blake3:artifact".to_string()],
            isolation_summary: "native-built-in".to_string(),
            settlement: grant.settlement,
            diagnostics: vec![RuntimeDiagnostic {
                key: "operator-note".to_string(),
                value: RedactedValue::Plain("completed".to_string()),
            }],
        };
        assert!(!receipt.contains_raw_secret());

        let unsafe_receipt = SponsoredUsageReceipt {
            settlement: SponsoredSettlementReference {
                method_tag: "voucher".to_string(),
                opaque_ref: RedactedValue::Plain("token=raw-payment-credential".to_string()),
            },
            ..receipt
        };
        assert!(unsafe_receipt.contains_raw_secret());
    }

    fn sponsored_provider_policy() -> SponsoredProviderPolicy {
        SponsoredProviderPolicy {
            provider_principal_id: "provider/nodepool-a".to_string(),
            accepted_sponsor_principal_ids: vec!["org/aspen-foundation".to_string()],
            accepted_workload_principal_ids: vec!["workload/aspen-ci".to_string()],
            accepted_service_principal_ids: vec!["service/forge".to_string()],
            accepted_settlement_tags: vec!["none:internal-grant".to_string()],
            accepted_isolation_modes: vec![RuntimeHostKind::NativeBuiltIn],
            max_request: sponsored_limits(),
        }
    }

    fn sponsored_admission_request() -> SponsoredAdmissionRequest {
        SponsoredAdmissionRequest {
            grant: sponsored_grant(),
            provider_policy: sponsored_provider_policy(),
            ledger: SponsoredQuotaLedger {
                grant_id: "grant-open-source-ci".to_string(),
                total: sponsored_limits(),
                reserved: SponsoredResourceLimits {
                    cpu_millis: 1_000,
                    memory_bytes: 128 * 1024 * 1024,
                    storage_bytes_ms: 1_000,
                    network_bytes: 10_000,
                    wall_time_ms: 5_000,
                    max_concurrent: 0,
                },
                consumed: SponsoredResourceLimits {
                    cpu_millis: 1_000,
                    memory_bytes: 128 * 1024 * 1024,
                    storage_bytes_ms: 1_000,
                    network_bytes: 10_000,
                    wall_time_ms: 5_000,
                    max_concurrent: 0,
                },
                active_concurrency: 0,
            },
            workload_principal_id: "workload/aspen-ci".to_string(),
            service_principal_id: Some("service/forge".to_string()),
            provider_principal_id: "provider/nodepool-a".to_string(),
            isolation_mode: RuntimeHostKind::NativeBuiltIn,
            requested: SponsoredResourceLimits {
                cpu_millis: 1_000,
                memory_bytes: 128 * 1024 * 1024,
                storage_bytes_ms: 1_000,
                network_bytes: 10_000,
                wall_time_ms: 5_000,
                max_concurrent: 1,
            },
            now_ms: 2_000,
        }
    }

    #[test]
    fn sponsored_placement_constraint_is_optional_but_fails_closed_when_required() {
        let request = sponsored_admission_request();
        let optional = SponsoredPlacementConstraint {
            surface: SponsoredPlacementSurface::RuntimeService,
            unit_id: "service/forge".to_string(),
            sponsorship_required: false,
            request: None,
        };
        assert_eq!(admit_sponsored_placement(&optional), Ok(()));

        let required_missing = SponsoredPlacementConstraint {
            surface: SponsoredPlacementSurface::Job,
            unit_id: "job/ci-build".to_string(),
            sponsorship_required: true,
            request: None,
        };
        assert_eq!(admit_sponsored_placement(&required_missing), Err(SponsoredPlacementError::MissingRequiredGrant));

        let required_accepted = SponsoredPlacementConstraint {
            surface: SponsoredPlacementSurface::CiRun,
            unit_id: "ci/run/1".to_string(),
            sponsorship_required: true,
            request: Some(request.clone()),
        };
        assert_eq!(admit_sponsored_placement(&required_accepted), Ok(()));

        let rejected = SponsoredPlacementConstraint {
            request: Some(SponsoredAdmissionRequest {
                now_ms: 99_999,
                ..request
            }),
            ..required_accepted
        };
        assert_eq!(
            admit_sponsored_placement(&rejected),
            Err(SponsoredPlacementError::Rejected(SponsoredAdmissionError::ExpiredGrant))
        );
    }

    #[test]
    fn sponsored_admission_accepts_only_complete_in_scope_grants() {
        let request = sponsored_admission_request();
        admit_sponsored_request(&request).unwrap();
    }

    #[test]
    fn sponsored_admission_fails_closed_for_principal_time_revocation_and_provider_policy() {
        let mut missing_proof = sponsored_admission_request();
        missing_proof.grant.sponsor.proof_ref = None;
        assert_eq!(admit_sponsored_request(&missing_proof), Err(SponsoredAdmissionError::MissingPrincipalProof));

        let mut expired = sponsored_admission_request();
        expired.now_ms = expired.grant.valid_until_ms;
        assert_eq!(admit_sponsored_request(&expired), Err(SponsoredAdmissionError::ExpiredGrant));

        let mut revoked = sponsored_admission_request();
        revoked.grant.revocation.revoked = true;
        assert_eq!(admit_sponsored_request(&revoked), Err(SponsoredAdmissionError::RevokedGrant));

        let mut rejected_provider = sponsored_admission_request();
        rejected_provider.provider_principal_id = "provider/other".to_string();
        assert_eq!(admit_sponsored_request(&rejected_provider), Err(SponsoredAdmissionError::ProviderRejected));
    }

    #[test]
    fn sponsored_admission_fails_closed_for_settlement_quota_isolation_and_scope() {
        let mut unsupported_settlement = sponsored_admission_request();
        unsupported_settlement.grant.settlement.method_tag = "crypto:chain-x".to_string();
        assert_eq!(
            admit_sponsored_request(&unsupported_settlement),
            Err(SponsoredAdmissionError::UnsupportedSettlementTag)
        );

        let mut secret_settlement = sponsored_admission_request();
        secret_settlement.grant.settlement.opaque_ref = RedactedValue::Plain("token=secret".to_string());
        assert_eq!(
            admit_sponsored_request(&secret_settlement),
            Err(SponsoredAdmissionError::SecretBearingSettlementRef)
        );

        let mut quota_exhausted = sponsored_admission_request();
        quota_exhausted.requested.cpu_millis = sponsored_limits().cpu_millis;
        assert_eq!(admit_sponsored_request(&quota_exhausted), Err(SponsoredAdmissionError::QuotaExhausted));

        let mut isolation_mismatch = sponsored_admission_request();
        isolation_mismatch.isolation_mode = RuntimeHostKind::Wasm;
        assert_eq!(admit_sponsored_request(&isolation_mismatch), Err(SponsoredAdmissionError::IsolationMismatch));

        let mut workload_mismatch = sponsored_admission_request();
        workload_mismatch.workload_principal_id = "workload/other".to_string();
        assert_eq!(admit_sponsored_request(&workload_mismatch), Err(SponsoredAdmissionError::ScopeMismatch));

        let mut service_mismatch = sponsored_admission_request();
        service_mismatch.service_principal_id = Some("service/ci".to_string());
        assert_eq!(admit_sponsored_request(&service_mismatch), Err(SponsoredAdmissionError::ScopeMismatch));
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
