// r[impl molten.project.molten_core_no_std]
// Library builds without the `std` feature are `#![no_std]` + `alloc`; the unit-test harness keeps
// the standard library so test-only helpers stay unchanged across both feature profiles.
#![cfg_attr(not(any(feature = "std", test)), no_std)]
// The no_std build swaps the standard prelude for `core`'s plus the `alloc` items that the standard
// prelude would otherwise provide, so every module keeps one source form under both profiles and the
// standalone Octet consumers that mount these modules keep compiling against `std`.
#![cfg_attr(not(any(feature = "std", test)), feature(prelude_import))]
#![cfg_attr(not(any(feature = "std", test)), allow(internal_features))]
#![feature(register_tool)]
#![register_tool(tigerstyle)]

//! Pure Molten boundary cores.
//!
//! This crate intentionally has no filesystem, process, network, clock, Redb,
//! Wasmtime, Steel, Nickel runtime, Iroh, or CLI dependencies. It owns small
//! in-memory decision cores that adapter shells can call before side effects.
//!
//! Without the default `std` feature the crate is `#![no_std]` and uses only `core` and `alloc`.
//! The `std` feature adds `live_binding` and `world_merge` (which depend on `kamacite-core`) and
//! `world_branch_authority` (which depends on `basalt`); those upstream crates still require `std`.

extern crate alloc;

#[cfg(not(any(feature = "std", test)))]
#[allow(
    unused_imports,
    reason = "rustc's unused-import analysis does not count type names resolved through the prelude"
)]
mod no_std_prelude {
    pub(crate) use alloc::borrow::ToOwned;
    pub(crate) use alloc::boxed::Box;
    pub(crate) use alloc::format;
    pub(crate) use alloc::string::String;
    pub(crate) use alloc::string::ToString;
    pub(crate) use alloc::vec;
    pub(crate) use alloc::vec::Vec;
    pub(crate) use core::prelude::rust_2024::*;
}

#[cfg(not(any(feature = "std", test)))]
#[prelude_import]
#[allow(unused_imports)]
use crate::no_std_prelude::*;

pub mod addressable_actor;
#[path = "cluster/harness.rs"]
pub mod cluster_harness;
pub mod codec;
#[path = "collaborationscope/mod.rs"]
pub mod collaboration_scope;
pub mod committed_stream;
pub mod compute_capacity;
pub mod consensus_recovery;
pub mod content_replication;
pub mod content_store_adapter;
pub mod coordination_delivery;
pub mod dag_sync;
#[cfg(feature = "executable-extents")]
pub mod executable_extent;
pub mod fabric;
pub mod fabric_crypto_identity;
pub mod fabric_durability;
pub mod fabric_execution;
pub mod fabric_membership;
pub mod fabric_observability;
pub mod fabric_simulation;
pub mod fabric_time;
pub mod fabric_transport;
pub mod federation;
#[cfg(feature = "std")]
pub mod live_binding;
pub mod marble_store;
pub mod node_ingress;
pub mod nominal;
pub mod outside_in;
pub mod planning;
pub mod prolly_map;
pub mod protocol_oracle;
pub mod release_dependency;
pub mod stack;
pub mod startup_binding;
pub mod system_extension;
pub mod world_benchmark;
#[cfg(feature = "std")]
pub mod world_branch_authority;
#[path = "worldcommit/mod.rs"]
pub mod world_commit;
pub mod world_distribution;
pub mod world_faults;
pub mod world_head;
#[cfg(feature = "std")]
pub mod world_merge;
pub mod world_operator;
pub mod world_promotion;
pub mod world_replay;
pub mod world_snapshot;
pub mod world_state_oracle;
pub mod world_witness;

pub mod prelude {
    pub use crate::codec::CodecIssue;
    pub use crate::codec::DomainArtifactInput;
    pub use crate::codec::DomainArtifactSummary;
    pub use crate::codec::validate_domain_artifact;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::ActivationDecision;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::ActivationFacts;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::AdmissionDecision;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::CodeProfile;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::ConsumerProfile;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::ExtentCodeRootProfile;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::ProducerBundleFacts;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::RemeasuredExtent;
    #[cfg(feature = "executable-extents")]
    pub use crate::executable_extent::admit_code_profile;
    pub use crate::fabric_durability::ownership::CasLease;
    pub use crate::fabric_durability::ownership::CasLeaseDecision;
    pub use crate::fabric_durability::ownership::CasLeaseDecisionInput;
    pub use crate::fabric_durability::ownership::CasLeaseDisposition;
    pub use crate::fabric_durability::ownership::CasLeaseRejection;
    pub use crate::fabric_durability::ownership::MembershipPosture;
    pub use crate::fabric_durability::ownership::decide_cas_lease;
    pub use crate::planning::EffectKind;
    pub use crate::release_dependency::ArchiveReceiptExpectation;
    pub use crate::release_dependency::ArchiveReceiptObservation;
    pub use crate::release_dependency::CanonicalValenceAuthority;
    pub use crate::release_dependency::DependencyExpectation;
    pub use crate::release_dependency::DependencyObservation;
    pub use crate::release_dependency::DiagnosticCode;
    pub use crate::release_dependency::DistributionObservation;
    pub use crate::release_dependency::DistributionProfile;
    pub use crate::release_dependency::EvidenceFileExpectation;
    pub use crate::release_dependency::EvidenceFileObservation;
    pub use crate::release_dependency::ReleaseDependencyInput;
    pub use crate::release_dependency::ReleaseDependencyReport;
    pub use crate::release_dependency::ReleaseDisposition;
    pub use crate::release_dependency::ResolvedPackageIdentity;
    pub use crate::release_dependency::SourceKind;
    pub use crate::release_dependency::TransportPolicy;
    pub use crate::release_dependency::validate_release_dependencies;
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
    pub use crate::world_commit::CapturePlan;
    pub use crate::world_commit::CaptureRequest;
    pub use crate::world_commit::ClosureReport;
    pub use crate::world_commit::ClosureRequest;
    pub use crate::world_commit::RestorePlan;
    pub use crate::world_commit::RootKind;
    pub use crate::world_commit::SnapshotProfile;
    pub use crate::world_commit::SnapshotProfileKind;
    pub use crate::world_commit::WorldCommitCore;
    pub use crate::world_commit::WorldCommitRef;
    pub use crate::world_commit::WorldRootRef;
    pub use crate::world_commit::compare_revision_rechecks;
    pub use crate::world_commit::identify_world_commit;
    pub use crate::world_commit::plan_capture;
    pub use crate::world_commit::plan_restore;
    pub use crate::world_commit::validate_closure;
}
