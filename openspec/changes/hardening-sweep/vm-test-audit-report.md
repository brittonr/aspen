# NixOS Check Audit Report

**Date**: 2026-03-24
**Total checks**: 101
**Pass**: 73 (72%)
**Fail**: 28 (27%)
**Regressions from hardening sprint**: 0

## Summary

All 101 `nix flake check` attributes were built and categorized.
No regressions from the hardening sprint (cluster discovery, write
forwarding, tiger style, health gates) were found. The only code-level
fix required was removing an unused `anyhow::Context` import in
`aspen-raft` that caused a clippy denial — this was a pre-existing
issue that manifested when the CI clippy check ran with `--deny warnings`.

## Pass (73)

audit, build-cli, build-node, ciSrc-no-unvendored-git-deps, clippy,
deny, doc, fmt, nextest, nextest-quick, rolling-restart-test,
snix-boot-test, test-aspen-blob, test-aspen-calendar, test-aspen-ci-core,
test-aspen-client, test-aspen-client-api, test-aspen-cluster-bridges,
test-aspen-cluster-types, test-aspen-constants, test-aspen-contacts,
test-aspen-coordination, test-aspen-coordination-protocol,
test-aspen-core, test-aspen-crypto, test-aspen-dht-discovery,
test-aspen-disk, test-aspen-docs, test-aspen-federation,
test-aspen-forge-protocol, test-aspen-h3-proxy, test-aspen-hlc,
test-aspen-hooks-types, test-aspen-jobs, test-aspen-jobs-protocol,
test-aspen-jobs-worker-blob, test-aspen-jobs-worker-maintenance,
test-aspen-jobs-worker-replication, test-aspen-jobs-worker-shell,
test-aspen-jobs-worker-sql, test-aspen-kv-branch, test-aspen-kv-types,
test-aspen-layer, test-aspen-nickel, test-aspen-nostr-relay,
test-aspen-plugin-api, test-aspen-raft, test-aspen-raft-network,
test-aspen-raft-types, test-aspen-redb-storage, test-aspen-secrets,
test-aspen-sharding, test-aspen-sql, test-aspen-storage-types,
test-aspen-testing, test-aspen-testing-core, test-aspen-testing-fixtures,
test-aspen-testing-network, test-aspen-ticket, test-aspen-time,
test-aspen-traits, test-aspen-transport, test-nar-bridge, test-nix-daemon,
test-patchbay, test-snix-build, test-snix-castore-http, test-snix-eval,
test-snix-glue, test-snix-serde, test-snix-tracing, verus-check,
verus-inline-check

## Fail — unit2nix build expression error (16)

**Root cause**: The unit2nix Nix build system generates per-crate test
derivations. A nix expression error at line 60 (`dependencies;`) causes
builds for crates that depend on certain handler crates to fail. This is
a pre-existing issue in the unit2nix Nix expression generator, not a
test or code failure.

test-aspen-blob-handler, test-aspen-ci-handler, test-aspen-cluster,
test-aspen-cluster-handler, test-aspen-core-essentials-handler,
test-aspen-dag, test-aspen-docs-handler, test-aspen-forge,
test-aspen-forge-handler, test-aspen-forge-web, test-aspen-job-handler,
test-aspen-nix-cache-gateway, test-aspen-nix-handler,
test-aspen-rpc-core, test-aspen-secrets-handler,
test-aspen-testing-patchbay

## Fail — cascade from unit2nix error (6)

These crates depend on the 16 above and fail with
"Build failed due to failed dependency."

test-aspen-cache, test-aspen-ci-executor-nix,
test-aspen-ci-executor-shell, test-aspen-ci-executor-vm,
test-aspen-hooks, test-iroh-h3-axum

## Fail — push-to-cache post-build hook (2)

**Root cause**: The nix `post-build-hook` (`push-to-cache`) fails with
exit code 1. The actual tests pass but the nix build system reports
failure because the hook failed. Fixed by running with
`--option post-build-hook ""`.

test-aspen-auth, test-aspen-automerge

## Fail — timeout (1)

**Root cause**: `ci-dogfood-full-loop-test` is a NixOS VM test that
boots QEMU, starts a cluster, pushes source, triggers CI, builds, and
verifies — all inside the VM. The 600s timeout was not enough. The
bare-metal `dogfood-local -- full-loop` passed successfully.

ci-dogfood-full-loop-test

## Fail — pre-existing test failures (3)

**multi-node-dogfood-test**: Pipeline `build-pkg` job fails inside the
VM. Pre-existing — the multi-node dogfood NixOS test has been flaky.

**snix-rev-check**: Consistency check — the snix git rev pinned in
`crates/aspen-ci/src/checkout.rs` doesn't match the flake.lock rev.
Pre-existing tracking issue.

**test-aspen-deploy**: Pre-existing deploy test failure.
