//! Test infrastructure for snix-only build tests.
//!
//! Provides in-memory service stacks and a fake `BuildService` so that
//! the full eval→build→upload pipeline can be exercised without spawning
//! any `nix` or `nix-store` subprocess.

use std::collections::HashMap;
use std::io;
use std::num::NonZeroUsize;
use std::sync::Arc;

#[cfg(feature = "snix-build")]
use snix_build::buildservice::BuildOutput;
#[cfg(feature = "snix-build")]
use snix_build::buildservice::BuildRequest;
#[cfg(feature = "snix-build")]
use snix_build::buildservice::BuildResult;
#[cfg(feature = "snix-build")]
use snix_build::buildservice::BuildService;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::MemoryBlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_castore::directoryservice::RedbDirectoryService;
use snix_store::pathinfoservice::LruPathInfoService;
use snix_store::pathinfoservice::PathInfoService;

// ============================================================================
// Subprocess-free environment
// ============================================================================

/// Return a sanitised `PATH` environment variable that contains only safe
/// POSIX utilities and explicitly excludes `nix`, `nix-store`, etc.
///
/// Callers pass this to `Command::envs()` or use it to verify that test
/// code paths cannot reach the nix CLI.
pub fn no_nix_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    // Only allow basic POSIX utilities — no nix binaries.
    env.insert("PATH".to_string(), "/usr/bin:/usr/sbin:/bin:/sbin".to_string());
    // Prevent nix from reading user config.
    env.insert("NIX_CONF_DIR".to_string(), "/nonexistent".to_string());
    env
}

/// Assert that no `nix` binary is reachable from the sanitised PATH.
/// Call at the start of a test to document intent.
pub fn assert_no_nix_in_path(env: &HashMap<String, String>) {
    if let Some(path) = env.get("PATH") {
        for dir in path.split(':') {
            let nix_bin = std::path::Path::new(dir).join("nix");
            assert!(
                !nix_bin.exists() || !is_nix_binary(&nix_bin),
                "nix binary found at {}: test environment is not subprocess-free",
                nix_bin.display()
            );
        }
    }
}

/// Best-effort check whether a path is actually a nix binary.
fn is_nix_binary(path: &std::path::Path) -> bool {
    std::process::Command::new(path)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.contains("nix") && (stdout.contains("Nix") || stdout.contains("nix-"))
        })
        .unwrap_or(false)
}

// ============================================================================
// FakeBuildService
// ============================================================================

/// A `BuildService` that returns a pre-configured result without executing
/// anything. No subprocess, no filesystem, no sandbox.
#[cfg(feature = "snix-build")]
pub struct FakeBuildService {
    result: Result<BuildResult, String>,
}

#[cfg(feature = "snix-build")]
impl FakeBuildService {
    /// Create a fake that returns success with the given outputs.
    pub fn success(outputs: Vec<BuildOutput>) -> Self {
        Self {
            result: Ok(BuildResult { outputs }),
        }
    }

    /// Create a fake that returns an error.
    pub fn failure(msg: &str) -> Self {
        Self {
            result: Err(msg.to_string()),
        }
    }
}

#[cfg(feature = "snix-build")]
#[::tonic::async_trait]
impl BuildService for FakeBuildService {
    async fn do_build(&self, _request: BuildRequest) -> io::Result<BuildResult> {
        match &self.result {
            Ok(r) => Ok(r.clone()),
            Err(msg) => Err(io::Error::other(msg.clone())),
        }
    }
}

// ============================================================================
// TestSnixStack
// ============================================================================

/// In-memory snix service stack for testing. No disk, no network, no nix CLI.
pub struct TestSnixStack {
    pub blob_service: Arc<dyn BlobService>,
    pub directory_service: Arc<dyn DirectoryService>,
    pub pathinfo_service: Arc<dyn PathInfoService>,
}

impl TestSnixStack {
    /// Create a new stack with in-memory implementations.
    pub fn new() -> Self {
        Self {
            blob_service: Arc::new(MemoryBlobService::default()),
            directory_service: Arc::new(
                RedbDirectoryService::new_temporary("test".to_string(), Default::default())
                    .expect("failed to create temp dir service"),
            ),
            pathinfo_service: Arc::new(LruPathInfoService::with_capacity(
                "test".to_string(),
                NonZeroUsize::new(1000).unwrap(),
            )),
        }
    }

    /// Create a `NativeBuildService` wired to this stack with the given
    /// `FakeBuildService`.
    #[cfg(feature = "snix-build")]
    pub fn native_build_service(&self, fake: FakeBuildService) -> crate::build_service::NativeBuildService {
        crate::build_service::NativeBuildService::with_build_service(
            Box::new(fake),
            self.blob_service.clone(),
            self.directory_service.clone(),
            self.pathinfo_service.clone(),
        )
    }

    /// Build a `NixEvaluator` backed by this in-memory stack.
    #[cfg(feature = "snix-eval")]
    pub fn evaluator(&self) -> crate::eval::NixEvaluator {
        crate::eval::NixEvaluator::new(
            self.blob_service.clone(),
            self.directory_service.clone(),
            self.pathinfo_service.clone(),
        )
    }
}
