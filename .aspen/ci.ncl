# Aspen CI Pipeline Configuration
#
# Stages:
# 1. Format check: shell job (~5s)
# 2. Build: VM-isolated nix build (~2-5min with cache)
#
# Tests are run separately via `nix flake check` or `cargo nextest run`.
# This keeps the dogfood CI fast for iteration.
#
# VM jobs execute inside isolated Cloud Hypervisor microVMs for:
# - Full kernel-level isolation
# - No network access (offline builds using pre-fetched flake inputs)
# - Ephemeral filesystem (fresh state each build)
# - Resource limits enforced by hypervisor

{
  name = "aspen-build",
  description = "CI pipeline for Aspen (build only, tests run separately)",

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
          args = ["build", "-L", "--print-out-paths", ".#packages.x86_64-linux.default"],
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 3600,
          artifacts = ["result", "result/*"],
        },
      ],
    },

    # NOTE: Tests removed for faster dogfood iteration.
    # Run tests locally with: cargo nextest run -P quick
    # Or via nix: nix build .#checks.x86_64-linux.nextest-quick
  ],

  artifacts = {
    storage = 'blobs,
    retention_days = 30,
    compress = true,
  },

  env = {
    RUST_BACKTRACE = "1",
    # Enable debug logging for SNIX-related modules to diagnose upload issues
    RUST_LOG = "info,aspen_snix=debug,aspen_ci::workers::local_executor=debug,aspen_rpc_handlers::handlers::snix=debug",
  },

  timeout_secs = 3600,
  priority = 'normal,
}
