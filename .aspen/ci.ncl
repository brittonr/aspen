# Aspen CI Pipeline Configuration
#
# Uses Cloud Hypervisor VMs for isolated builds.
# VM isolation provides:
# - Kernel-level isolation between builds
# - Clean Nix store access via VirtioFS
# - No network access (security)
# - Fast boot (~125ms) with warm VM pool

{
  name = "aspen-build",
  description = "CI pipeline for Aspen with Cloud Hypervisor VM isolation",

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

    # Stage 2: Full build in VM isolation
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-release",
          type = 'vm,
          command = "nix",
          args = ["build", "-L", ".#packages.x86_64-linux.default"],
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 3600,
          artifacts = ["result/**"],
          env = {
            RUST_BACKTRACE = "1",
            RUST_LOG = "info",
          },
        },
      ],
    },

    # Stage 3: Unit tests in VM
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "quick-tests",
          type = 'vm,
          command = "nix",
          args = ["develop", "-c", "cargo", "nextest", "run", "-P", "quick"],
          flake_attr = "devShells.x86_64-linux.default",
          timeout_secs = 1800,
          env = {
            RUST_BACKTRACE = "1",
          },
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
