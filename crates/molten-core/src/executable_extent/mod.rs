//! Pure executable-extent consumer admission.
//!
//! This module receives decoded bundle, remeasurement, runtime, and authority
//! facts. It performs no file, mapping, clock, network, or policy I/O.

mod admission;
mod model;

pub use admission::admit_code_profile;
pub use model::ActivationDecision;
pub use model::ActivationDenial;
pub use model::ActivationFacts;
pub use model::AdmissionDecision;
pub use model::AdmissionError;
pub use model::BuiltArtifactIdentity;
pub use model::CodeProfile;
pub use model::ConsumerProfile;
pub use model::ExecutableExtentIdentity;
pub use model::ExtentCodeRootProfile;
pub use model::ExtentDescriptor;
pub use model::ExtentManifestIdentity;
pub use model::ExtentMappingIntent;
pub use model::ExtentPlan;
pub use model::LiveMappingIdentity;
pub use model::MappingTransition;
pub use model::PolicyIdentity;
pub use model::ProducerBundleFacts;
pub use model::ProducerReceiptIdentity;
pub use model::RemeasuredExtent;
pub use model::RuntimeCohortIdentity;
pub use model::SemanticCodeIdentity;

#[cfg(test)]
mod tests;
