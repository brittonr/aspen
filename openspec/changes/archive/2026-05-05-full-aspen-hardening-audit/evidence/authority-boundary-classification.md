# Authority Boundary Classification Seed

Status: captured (Phase 2).

## Intentionally public

Public classification is not assumed from naming. Phase 3 must enumerate exact `to_operation() == None` cases. Seed candidates are bootstrap/challenge and health-style requests only when source tests confirm public behavior.

## Capability-protected

Ordinary data and domain operations must map to `Operation` variants from `crates/aspen-auth-core/src/capability.rs`. Generic `Read`/`Write`/`Full` capabilities do not automatically imply domain-specific authority.

## Presenter-bound

Key-bound delegated client tokens require authenticated Iroh presenter identity. Audit evidence must show the remote public key is used before handler logic and is not substituted by caller-provided metadata.

## Cluster-admin

Cluster membership, deploy lifecycle/status, privileged maintenance, and fail-closed mixed reserved-prefix operations require `ClusterAdmin { action: "cluster_operation" }` or a narrower future admin operation.

## Federation-proxy delegation

Proxyable federation credentials must be bearer-only, short-lived, proof-bearing, bounded by delegation depth, marked with the federation-proxy signed fact, and limited to federation pull/push child capabilities.

## SNIX-specific

SNIX DirectoryService and PathInfoService store/cache access requires `SnixRead`/`SnixWrite`. Generic KV capabilities over `snix:` prefixes are not accepted for store authority.

## Filesystem-permission protected

Local secret key, token, and ticket outputs are protected by owner-only permissions and safe operator output. Evidence may record path, permission mode, file size, or redacted placeholder, but not credential contents.
