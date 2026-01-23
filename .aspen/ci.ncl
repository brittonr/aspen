# Aspen CI Pipeline Configuration
#
# This file defines the CI/CD pipeline for the Aspen project itself,
# demonstrating self-hosted CI using aspen-ci.

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
    # Stage 1: Fast checks (formatting, linting)
    {
      name = "check",
      parallel = true,
      jobs = [
        {
          name = "fmt",
          type = 'shell,
          command = "nix",
          args = ["run", ".#rustfmt", "--", "check"],
          timeout_secs = 300,
          allow_failure = true,
        },
        {
          name = "clippy",
          type = 'shell,
          command = "cargo",
          args = ["clippy", "--all-targets", "--", "--deny", "warnings"],
          timeout_secs = 600,
        },
      ],
    },

    # Stage 2: Build
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-debug",
          type = 'nix,
          flake_url = ".",
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 1800,
          isolation = 'nix_sandbox,
        },
      ],
    },

    # Stage 3: Tests (parallel unit and integration tests)
    {
      name = "test",
      depends_on = ["build"],
      parallel = true,
      jobs = [
        {
          name = "unit-tests",
          type = 'shell,
          command = "cargo",
          args = ["nextest", "run", "-P", "quick"],
          timeout_secs = 1800,
          retry_count = 1,
        },
        {
          name = "doc-tests",
          type = 'shell,
          command = "cargo",
          args = ["test", "--doc"],
          timeout_secs = 600,
        },
      ],
    },

    # Stage 4: Extended tests (madsim, proptests) - only on main
    {
      name = "extended-tests",
      depends_on = ["test"],
      when = "refs/heads/main",
      parallel = true,
      jobs = [
        {
          name = "madsim-tests",
          type = 'shell,
          command = "cargo",
          args = ["nextest", "run", "-P", "ci", "--features", "madsim"],
          timeout_secs = 3600,
          allow_failure = true,
        },
        {
          name = "proptest",
          type = 'shell,
          command = "cargo",
          args = ["nextest", "run", "-E", "test(/proptest/)"],
          timeout_secs = 1800,
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
