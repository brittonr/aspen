# Aspen CI Pipeline Configuration
#
# - Format check: shell (quick, no sandbox)
# - Build: nix type with upload_result=true (uploads binaries to blob store)
# - Test: nix type with upload_result=false (26GB test artifacts not cached)

{
  name = "aspen-build",
  description = "CI pipeline for Aspen with Nix-sandboxed tests",

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

    # Stage 2: Full build (uses NixBuildWorker for store path upload)
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-release",
          type = 'nix,
          flake_attr = "packages.x86_64-linux.default",
          timeout_secs = 3600,
          # Upload the release binaries to blob store (typically ~50-100MB)
          upload_result = true,
        },
      ],
    },

    # Stage 3: Quick tests (Nix sandbox isolates from host tmpfs)
    # Uses -P quick profile which skips slow proptest/chaos/madsim tests
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "nextest-quick",
          type = 'nix,
          flake_attr = "checks.x86_64-linux.nextest-quick",
          timeout_secs = 1800,
          # Test outputs are transient (26GB target dir); don't cache
          upload_result = false,
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
