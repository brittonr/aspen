//! Nix build executor for Aspen CI jobs.
//!
//! This crate provides the `NixBuildWorker` for executing Nix flake builds
//! and storing artifacts in the distributed blob store and Nix binary cache.
//!
//! # Features
//!
//! - Nix flake build execution with configurable timeout and sandbox settings
//! - Artifact collection with glob pattern matching
//! - NAR archive generation and upload to blob store
//! - Cache registration for distributed Nix binary cache
//! - SNIX integration for decomposed content-addressed storage
//! - Cache proxy support for using the cluster's binary cache as a substituter
//!
//! # Example
//!
//! ```ignore
//! use aspen_ci_executor_nix::{NixBuildWorker, NixBuildWorkerConfig};
//! use std::path::PathBuf;
//!
//! let config = NixBuildWorkerConfig {
//!     node_id: 1,
//!     cluster_id: "my-cluster".to_string(),
//!     output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
//!     ..Default::default()
//! };
//!
//! let worker = NixBuildWorker::new(config);
//! ```

mod artifacts;
mod cache;
mod config;
pub mod derivation;
mod executor;
pub mod flakeref;
mod payload;
#[cfg(feature = "snix")]
mod snix;
mod timing;
mod worker;

pub use cache::UploadedStorePath;
pub use config::NixBuildWorkerConfig;
pub use executor::NixBuildWorker;
pub use payload::NixBuildPayload;
#[cfg(feature = "snix")]
pub use snix::UploadedStorePathSnix;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_validation() {
        let valid = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            cache_outputs: vec![],
            source_hash: None,
        };

        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_payload_validation_empty_url() {
        let invalid = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: "".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            cache_outputs: vec![],
            source_hash: None,
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_payload_validation_timeout_too_long() {
        let invalid = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 100000, // Way too long
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            cache_outputs: vec![],
            source_hash: None,
        };

        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_flake_ref() {
        let payload = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: "github:owner/repo".to_string(),
            attribute: "packages.x86_64-linux.default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            cache_outputs: vec![],
            source_hash: None,
        };

        assert_eq!(payload.flake_ref(), "github:owner/repo#packages.x86_64-linux.default");
    }

    #[test]
    fn test_flake_ref_no_attribute() {
        let payload = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: true,
            publish_to_cache: true,
            cache_outputs: vec![],
            source_hash: None,
        };

        assert_eq!(payload.flake_ref(), ".");
    }
}
