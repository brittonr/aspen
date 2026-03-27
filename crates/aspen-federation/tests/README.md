# Federation Test Tiers

## Tier 1: Unit Tests (always run)

Inline `#[cfg(test)]` modules in `src/`. No network, no external dependencies.

```sh
cargo nextest run -p aspen-federation
```

**199 tests** covering:

- Verified pure functions (ref_diff, quorum, fork detection)
- Policy modules (resource_policy, verification, selection)
- Auth token issuance, verification, delegation, expiry, revocation
- Type roundtrips (postcard serialization)
- Trust manager, discovery types, subscription types

## Tier 2: Integration Tests (network profile)

Files in `tests/`. Require iroh socket binding (loopback). Marked `#[ignore]` for
CI sandboxes that block networking.

```sh
cargo nextest run -P network --run-ignored all -p aspen-federation
```

**238 tests** (199 unit + 39 integration):

- **`federation_wire_test.rs`** — Two iroh endpoints simulating independent clusters.
  Tests handshake, resource listing, state queries, object sync, delegate signature
  verification, content hash validation, incremental pull.

- **`federation_auth_cluster_test.rs`** — Credential-based authorization over real
  iroh QUIC connections. Tests the full handshake → credential verification →
  resource access pipeline across simulated clusters.

- **`federation_auth_test.rs`** — Token issuance, verification, delegation chains,
  unauthorized access rejection, expiry, and revocation (no network needed, runs
  in both tiers).

## Tier 3: NixOS VM Tests (nix build)

Full two-cluster tests with real aspen-node processes, Forge, and CI.

```sh
nix build .#checks.x86_64-linux.federation-test
nix build .#checks.x86_64-linux.federation-ref-fetch-test
nix build .#checks.x86_64-linux.federation-blob-transfer-test
nix build .#checks.x86_64-linux.federation-git-clone-test
nix build .#checks.x86_64-linux.federation-ci-dogfood-test --impure --option sandbox false
```

- **`federation.nix`** — Status, trust management, repo creation, cluster independence
- **`federation-ref-fetch.nix`** — Cross-cluster sync + fetch of ref objects
- **`federation-blob-transfer.nix`** — Git objects transferred via sync protocol
- **`federation-git-clone.nix`** — `git clone aspen://` across clusters
- **`federation-ci-dogfood.nix`** — Full pipeline: Alice pushes → Bob federates → CI builds → binary runs

## Verus Formal Verification

Pure function specs verified via SMT solver. Not tests per se, but proves properties
for all possible inputs.

```sh
nix run .#verify-verus federation
```

**13 verified items** covering:

- Seeder quorum (threshold minimum, majority, monotonicity)
- Fork detection (agreement, disagreement, hash comparison)
- Ref diff (partition completeness, classification correctness, conflict resolution)
