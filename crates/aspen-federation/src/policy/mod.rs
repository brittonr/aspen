//! Federation policy primitives.
//!
//! Generic, application-agnostic policies for controlling how federated
//! resources are verified, selected, and synchronized across clusters.
//!
//! Any application (Forge, CI, blobs, docs) can configure these policies
//! for its resources. The federation layer enforces them uniformly.
//!
//! # Components
//!
//! - [`ResourcePolicy`]: Per-resource federation policy (access + verification + selection + fork
//!   handling)
//! - [`VerificationConfig`]: Quorum thresholds and signature requirements
//! - [`SelectionStrategy`]: How to pick which cluster to sync from
//! - [`ForkDetectionMode`]: What to do when seeders disagree
//! - [`ClusterSelector`]: Trait for custom cluster ranking
//! - [`SeederQuorumVerifier`]: Verifies quorum agreement before accepting data

pub mod fork_detection;
pub mod resource_policy;
pub mod selection;
pub mod verification;

pub use fork_detection::ForkDetectionMode;
pub use resource_policy::ResourcePolicy;
pub use selection::ClusterSelector;
pub use selection::DefaultClusterSelector;
pub use selection::RankedCluster;
pub use selection::RankedSeeder;
pub use selection::SelectionStrategy;
pub use verification::SeederQuorumVerifier;
pub use verification::VerificationConfig;
