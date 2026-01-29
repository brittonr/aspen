# Aspen CI Pipeline Configuration
#
# Local shell execution for testing (no VM isolation).
# For production, change type = 'shell to type = 'vm for:
# - Kernel-level isolation between builds
# - Clean Nix store access via VirtioFS
# - No network access (security)
# - Fast boot (~125ms) with warm VM pool

{
  name = "aspen-build",
  description = "CI pipeline for Aspen (local shell for testing)",

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
    # Stage 1: Quick format check (shell, no VM overhead)
    {
      name = "check",
      jobs = [
        {
          name = "format-check",
          type = 'shell,
          command = "nix fmt . -- --check",
          timeout_secs = 300,
        },
      ],
    },

    # Stage 2: Full build (local shell for testing)
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-release",
          type = 'shell,
          command = "nix build -L .#packages.x86_64-linux.default",
          timeout_secs = 3600,
        },
      ],
    },

    # Stage 3: Unit tests (local shell for testing)
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "quick-tests",
          type = 'shell,
          command = "nix develop -c cargo nextest run -P quick",
          timeout_secs = 1800,
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
