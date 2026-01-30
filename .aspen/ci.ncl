# Aspen CI Pipeline Configuration
#
# Shell execution for format/build, Nix sandbox for tests.
# Using `nix build` for tests runs cargo in Nix's isolated sandbox,
# avoiding the tmpfs size issue where cargo's target/ fills /tmp.

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

    # Stage 2: Full build
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
