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
#[cfg(feature = "snix-build")]
pub mod build_service;
mod cache;
#[cfg(feature = "snix-eval")]
pub mod call_flake;
mod config;
pub mod derivation;
#[cfg(feature = "snix-eval")]
pub mod eval;
mod executor;
#[cfg(feature = "snix-build")]
pub mod fetch;
#[cfg(feature = "snix-eval")]
pub mod flake_compat;
#[cfg(feature = "snix-eval")]
pub mod flake_lock;
pub mod flakeref;
mod payload;
#[cfg(feature = "snix")]
mod snix;
mod timing;
mod worker;

#[cfg(feature = "snix-build")]
pub use build_service::NativeBuildService;
pub use cache::UploadedStorePath;
pub use config::NixBuildWorkerConfig;
#[cfg(feature = "snix-eval")]
pub use eval::NixEvaluator;
pub use executor::NixBuildWorker;
pub use executor::ProjectType;
pub use executor::detect_project_type;
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

    /// Verify that NixBuildWorker compiles and can be constructed without
    /// snix-build feature — subprocess path only, no native service field.
    #[test]
    fn test_worker_construction_default_features() {
        let config = NixBuildWorkerConfig::default();
        let _worker = NixBuildWorker::new(config);
        // Worker should be constructible; no native_build_service field
        // when snix-build is off (won't even compile if struct is wrong).
    }

    /// Verify that try_native_build returns error when native service is None.
    /// This confirms the fallback path is taken.
    #[cfg(feature = "snix-build")]
    #[tokio::test]
    async fn test_try_native_build_no_service_returns_error() {
        let config = NixBuildWorkerConfig::default();
        let worker = NixBuildWorker::new(config);

        // native_build_service is None by default
        assert!(!worker.has_native_builds());

        let payload = NixBuildPayload {
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
            should_upload_result: false,
            publish_to_cache: false,
            cache_outputs: vec![],
            source_hash: None,
        };

        let result = worker.try_native_build(&payload, &payload.flake_ref(), None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{err}");
        assert!(err_msg.contains("not initialized"), "expected 'not initialized' error, got: {err_msg}");
    }

    /// Verify resolve_drv_path validates the output format.
    /// Uses a fake nix binary that prints a non-drv path.
    #[cfg(feature = "snix-build")]
    #[tokio::test]
    async fn test_resolve_drv_path_rejects_invalid_output() {
        use std::io::Write;

        // Create a fake nix binary that prints garbage
        let tmpdir = tempfile::tempdir().unwrap();
        let fake_nix = tmpdir.path().join("nix");
        {
            let mut f = std::fs::File::create(&fake_nix).unwrap();
            writeln!(f, "#!/bin/sh").unwrap();
            writeln!(f, "echo '/not/a/drv/path'").unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake_nix, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = NixBuildWorkerConfig {
            nix_binary: fake_nix.to_string_lossy().to_string(),
            ..Default::default()
        };
        let worker = NixBuildWorker::new(config);

        let payload = NixBuildPayload {
            run_id: None,
            job_name: None,
            flake_url: ".".to_string(),
            attribute: "default".to_string(),
            extra_args: vec![],
            working_dir: None,
            timeout_secs: 1800,
            sandbox: true,
            cache_key: None,
            artifacts: vec![],
            should_upload_result: false,
            publish_to_cache: false,
            cache_outputs: vec![],
            source_hash: None,
        };

        let result: std::result::Result<std::path::PathBuf, _> = worker.resolve_drv_path(&payload, ".#default").await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("unexpected drv path"), "expected 'unexpected drv path' error, got: {err_msg}");
    }
}
