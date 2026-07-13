//! Pure Molten boundary cores.
//!
//! This crate intentionally has no filesystem, process, network, clock, Redb,
//! Wasmtime, Steel, Nickel runtime, Iroh, or CLI dependencies. It owns small
//! in-memory decision cores that adapter shells can call before side effects.

pub mod cluster_harness;
pub mod codec;
pub mod dependency;
pub mod fabric;
pub mod fabric_durability;
pub mod fabric_membership;
pub mod fabric_time;
pub mod fabric_transport;
pub mod planning;
pub mod policy;
pub mod preserves_profile;
pub mod stack;
pub mod system_extension;

pub mod prelude {
    pub use crate::codec::CodecIssue;
    pub use crate::codec::DomainArtifactInput;
    pub use crate::codec::DomainArtifactSummary;
    pub use crate::codec::validate_domain_artifact;
    pub use crate::dependency::BoundaryDiagnostic;
    pub use crate::dependency::BoundaryRule;
    pub use crate::dependency::ImportFact;
    pub use crate::dependency::Layer;
    pub use crate::dependency::validate_dependency_boundaries;
    pub use crate::planning::AdmissionInputs;
    pub use crate::planning::BoundaryDecision;
    pub use crate::planning::EffectKind;
    pub use crate::planning::EffectPlan;
    pub use crate::planning::EvidencePolicyRuntimeInput;
    pub use crate::planning::HarnessGateInput;
    pub use crate::planning::JobExecutionInput;
    pub use crate::planning::NodeEnqueueInput;
    pub use crate::planning::RegistryDiscoveryInput;
    pub use crate::planning::RetentionGcInput;
    pub use crate::planning::StoreWriteInput;
    pub use crate::planning::plan_adapter_effects;
    pub use crate::planning::plan_evidence_policy_runtime_flow;
    pub use crate::planning::plan_harness_gate;
    pub use crate::planning::plan_job_execution;
    pub use crate::planning::plan_node_enqueue;
    pub use crate::planning::plan_registry_discovery;
    pub use crate::planning::plan_retention_gc;
    pub use crate::planning::plan_store_write;
    pub use crate::policy::PolicyFieldSet;
    pub use crate::policy::PolicyFreshnessIssue;
    pub use crate::policy::validate_policy_freshness;
    pub use crate::preserves_profile::PreservesArtifactMeasurement;
    pub use crate::preserves_profile::PreservesBoundaryIssue;
    pub use crate::preserves_profile::PreservesBoundaryRow;
    pub use crate::preserves_profile::validate_preserves_boundary_profile;
    pub use crate::stack::StackEvidenceEnvelope;
    pub use crate::stack::StackEvidenceIssue;
    pub use crate::stack::StackEvidenceMember;
    pub use crate::stack::StackEvidenceRole;
    pub use crate::stack::StackEvidenceSummary;
    pub use crate::stack::ValenceStackAdapterIssue;
    pub use crate::stack::ValenceStackAdapterReport;
    pub use crate::stack::ValenceStackAdapterReportRow;
    pub use crate::stack::ValenceStackAdapterRow;
    pub use crate::stack::default_valence_stack_adapter_rows;
    pub use crate::stack::validate_stack_evidence_envelope;
    pub use crate::stack::validate_valence_stack_adapter;
}
