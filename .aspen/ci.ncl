# Aspen CI Pipeline Configuration — Self-Hosted
#
# Stages:
# 1. Check: format validation (shell) + clippy lint (nix check)
# 2. Build: near-full aspen-node + full aspen-cli (parallel nix jobs)
# 3. Test: Quick test suite via nextest (nix check, ci-nix profile)
# 4. Deploy (tagged releases only): rolling deployment of CI-built aspen-node
#
# build-node enables all features except plugins (needs aspen-wasm-plugin
# repo, not available in ciSrc). This covers: CI, forge, git-bridge, blob,
# SQL, snix (eval+build+daemon+http), deploy, federation, net, proxy, etc.
#
# All stages use `type = 'nix` which runs `nix build` directly on the
# CI worker via NixBuildWorker. Cargo deps are vendored inside the Nix
# sandbox — no network access needed.
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
          timeout_secs = 3600,
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
          flake_attr = "checks.x86_64-linux.build-node",
          publish_to_cache = true,
          artifacts = ["bin/aspen-node"],
          timeout_secs = 3600,
        },
        {
          name = "build-cli",
          type = 'nix,
          flake_url = ".",
          flake_attr = "checks.x86_64-linux.build-cli",
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

    # Stage 4: Deploy — rolling upgrade on tagged releases.
    #
    # Gated on release tags so it doesn't run during normal CI builds.
    # For 1-node dogfood-local, deployment is handled at the script
    # level (do_deploy stops/restarts the node with the CI-built binary).
    # In multi-node production clusters, this stage runs the full
    # DeploymentCoordinator rolling upgrade via DeployExecutor.
    {
      name = "deploy",
      depends_on = ["test"],
      when = "refs/tags/release/*",
      jobs = [
        {
          name = "deploy-node",
          type = 'deploy,
          artifact_from = "build-node",
          strategy = "rolling",
          max_concurrent = 1,
          health_check_timeout_secs = 120,
          timeout_secs = 600,
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
