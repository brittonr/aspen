//! The admitted materialization facts an execution plan binds.
//!
//! The shell (`src/wasm/component/evidence/materialization`) verifies Mantle bundles and artifact
//! bytes and produces this value; the policy core only consumes it.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub evidence_scope: super::model::EvidenceScope,
    pub component_ref: alloc::string::String,
    pub wit_ref: alloc::string::String,
    pub bundle_ref: Option<alloc::string::String>,
    pub consumer: super::model::Consumer,
    pub profile_ref: alloc::string::String,
    pub mantle_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub valence_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub cairn_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub policy_refs: alloc::vec::Vec<alloc::string::String>,
    pub authority_refs: alloc::vec::Vec<alloc::string::String>,
    pub resource_refs: alloc::vec::Vec<alloc::string::String>,
}
