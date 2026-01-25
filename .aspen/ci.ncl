# Aspen CI Pipeline Configuration
#
# This file defines the CI/CD pipeline for the Aspen project itself,
# demonstrating self-hosted CI using aspen-ci.
#
# IMPORTANT: All cargo commands use CARGO_TARGET_DIR to avoid bloating
# the checkout directory. The target directory is set to a shared
# location that can be cleaned up separately.

{
  name = "aspen-build",
  description = "CI pipeline for Aspen distributed systems platform",

  triggers = {
    # Trigger on pushes to main and feature branches
    refs = [
      "refs/heads/main",
      "refs/heads/feature/*",
      "refs/heads/fix/*",
    ],
    # Don't trigger on documentation-only changes
    ignore_paths = [
      "*.md",
      "docs/*",
      "*.txt",
      ".gitignore",
    ],
  },

  stages = [
    # Stage 1: Fast checks (formatting only - skip clippy for speed)
    {
      name = "check",
      parallel = true,
      jobs = [
        {
          name = "fmt",
          type = 'shell,
          # Use nix run directly - doesn't need cargo target dir
          command = "nix",
          args = ["fmt", "--", "--check"],
          timeout_secs = 300,
          allow_failure = true,
        },
      ],
    },

    # Stage 2: Build using Nix (sandboxed, uses /nix/store)
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-release",
          type = 'nix,
          flake_url = ".",
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 3600,
          isolation = 'nix_sandbox,
        },
      ],
    },

    # Stage 3: Run tests via nix flake check (nextest profile)
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "nextest",
          type = 'nix,
          flake_url = ".",
          # Run nextest check - all builds happen in /nix/store, not checkout dir
          flake_attr = "checks.x86_64-linux.nextest",
          timeout_secs = 3600,
          isolation = 'nix_sandbox,
          allow_failure = true,
        },
      ],
    },
  ],

  artifacts = {
    storage = 'blobs,
    retention_days = 30,
    compress = true,
  },

  env = {
    RUST_BACKTRACE = "1",
    RUST_LOG = "info",
  },

  timeout_secs = 7200,
  priority = 'normal,
}
