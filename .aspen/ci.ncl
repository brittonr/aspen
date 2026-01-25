# Aspen CI Pipeline Configuration
#
# This file defines the CI/CD pipeline for the Aspen project itself,
# demonstrating self-hosted CI using aspen-ci.
#
# Uses Nix sandbox for builds - all artifacts stay in /nix/store,
# not the checkout directory, preventing disk exhaustion.

{
  name = "aspen-build",
  description = "CI pipeline for Aspen distributed systems platform",

  triggers = {
    refs = [
      "refs/heads/main",
      "refs/heads/feature/*",
      "refs/heads/fix/*",
    ],
    ignore_paths = [
      "*.md",
      "docs/*",
      "*.txt",
      ".gitignore",
    ],
  },

  stages = [
    # Build using Nix (sandboxed, all builds in /nix/store)
    {
      name = "build",
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
