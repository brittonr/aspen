#!/usr/bin/env bash
# Run the snix test suite across all snix-related crates.
#
# Usage:
#   scripts/test-snix.sh              # Run all non-ignored snix tests
#   scripts/test-snix.sh --run-ignored all  # Include sandbox-dependent tests (bwrap, nix daemon)
#   scripts/test-snix.sh -E 'test(/build_service/)'  # Filter to specific tests
#   scripts/test-snix.sh --list       # List all snix tests without running
#
# Crates:
#   aspen-snix             - Raft-backed BlobService, DirectoryService, PathInfoService
#   aspen-ci-executor-nix  - Nix eval, build_service, derivation, flake, fetch, cache
#   aspen-snix-bridge      - gRPC + nix-daemon bridge
#   aspen-nix-cache-gateway - HTTP binary cache gateway (nar-bridge)
#
# NixOS VM integration tests (not run by this script):
#   nix build .#checks.x86_64-linux.snix-store-test
#   nix build .#checks.x86_64-linux.snix-bridge-test
#   nix build .#checks.x86_64-linux.snix-native-build-test
#   nix build .#checks.x86_64-linux.snix-daemon-test
#   nix build .#checks.x86_64-linux.snix-boot-test
#   nix build .#checks.x86_64-linux.snix-bridge-virtiofs-test
#   nix build .#checks.x86_64-linux.snix-flake-native-build-test

set -euo pipefail

SNIX_PACKAGES=(
    -p aspen-snix
    -p aspen-ci-executor-nix
    -p aspen-snix-bridge
    -p aspen-nix-cache-gateway
)

# Enable snix-build feature for NativeBuildService / LocalStoreBuildService tests.
# Without this, build_service.rs (behind #[cfg(feature = "snix-build")]) is skipped.
exec cargo nextest run \
    -P snix \
    "${SNIX_PACKAGES[@]}" \
    --features snix-build \
    "$@"
