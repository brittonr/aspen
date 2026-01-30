# Aspen CI Pipeline Configuration
#
# - Format check: shell (quick, no VM overhead)
# - Build: VM type with full microVM isolation (Cloud Hypervisor nested VMs)
# - Test: VM type with full microVM isolation
#
# VM jobs execute inside isolated Cloud Hypervisor microVMs for:
# - Full kernel-level isolation
# - No network access (offline builds using pre-fetched flake inputs)
# - Ephemeral filesystem (fresh state each build)
# - Resource limits enforced by hypervisor

{
  name = "aspen-build",
  description = "CI pipeline for Aspen with microVM-isolated builds",

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
    # Format check is fast and safe - no need for VM isolation
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

    # Stage 2: Full build in isolated microVM
    # Uses CloudHypervisorWorker for kernel-level isolation
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
          artifacts = ["result", "result/*"],
        },
      ],
    },

    # Stage 3: Quick tests in isolated microVM
    # Uses -P quick profile which skips slow proptest/chaos/madsim tests
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "nextest-quick",
          type = 'vm,
          command = "nix",
          args = ["build", "-L", ".#checks.x86_64-linux.nextest-quick"],
          flake_attr = "checks.x86_64-linux.nextest-quick",
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
