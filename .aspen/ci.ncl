# Aspen CI Pipeline Configuration — Self-Hosted
#
# Stages:
# 1. Check: format validation (shell) + clippy lint (nix check)
# 2. Build: aspen-node + aspen-cli via unit2nix (parallel nix jobs)
# 3. Test: Quick test suite via nextest (nix check, ci-nix profile)
#
# All stages use `type = 'nix` which runs `nix build` directly on the
# CI worker via NixBuildWorker. Cargo deps are vendored inside the Nix
# sandbox — no network access needed. The unit2nix build system compiles
# each crate as a separate Nix derivation for fine-grained caching.
#
# Build outputs are automatically published to the Aspen nix binary cache
# when `publish_to_cache = true`.

{
  name = "aspen-build",
  description = "CI pipeline for Aspen — self-hosted build and test",

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
    # Stage 1: Lint checks (format + clippy)
    {
      name = "check",
      jobs = [
        {
          name = "format-check",
          type = 'shell,
          command = "nix fmt . -- --check",
          timeout_secs = 300,
        },
        {
          name = "clippy",
          type = 'nix,
          flake_url = ".",
          flake_attr = "checks.x86_64-linux.clippy",
          timeout_secs = 1800,
        },
      ],
    },

    # Stage 2: Build all packages via Nix
    # Uses NixBuildWorker which runs `nix build` directly.
    # Outputs are published to the distributed nix cache.
    {
      name = "build",
      depends_on = ["check"],
      jobs = [
        {
          name = "build-node",
          type = 'nix,
          flake_url = ".",
          flake_attr = "packages.x86_64-linux.aspen-node",
          publish_to_cache = true,
          artifacts = ["bin/aspen-node"],
          timeout_secs = 3600,
        },
        {
          name = "build-cli",
          type = 'nix,
          flake_url = ".",
          flake_attr = "packages.x86_64-linux.aspen-cli",
          publish_to_cache = true,
          timeout_secs = 1800,
        },
      ],
    },

    # Stage 3: Run quick test suite via Nix
    # Uses the nextest-quick check from flake.nix which includes
    # vendored deps, the ci-nix profile, and all workspace crates.
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "nextest-quick",
          type = 'nix,
          flake_url = ".",
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
    RUST_LOG = "info,aspen_ci=debug",
  },

  timeout_secs = 7200,
  priority = 'normal,
}
