# Dependency audit triage

Last refreshed: 2026-05-06.

The authoritative dependency-security gate is:

```bash
nix build .#checks.x86_64-linux.audit --no-link -L
```

The gate ignores only advisory-specific exceptions documented in `flake.nix` and
this file. Do not add blanket warning ignores: each new exception needs an
advisory ID, dependency path, exposure assessment, compensating control, and
removal trigger.

After remediating the two `rand` soundness warnings by lockfile-only updates,
replacing Aspen-owned bincode serialization, and accepting the SNIX/FUSE unsound
cluster as bounded upstream dependency debt, the direct audit inventory is:

```bash
cargo audit -n \
  --ignore RUSTSEC-2023-0071 \
  --ignore RUSTSEC-2026-0066 \
  --ignore RUSTSEC-2026-0002 \
  --ignore RUSTSEC-2023-0056 \
  --ignore RUSTSEC-2024-0002 \
  --ignore RUSTSEC-2023-0086
```

- vulnerabilities: `0`
- allowed warnings: `6`
  - unmaintained: `6`
  - unsound: `0`

## Remediated or bounded in this triage slice

| Advisory | Crate | Old | New | Remediation |
| --- | --- | ---: | ---: | --- |
| `RUSTSEC-2026-0097` | `rand` | `0.8.5` | `0.8.6` | Lockfile point update; patched release stays within `0.8` ABI constraints. |
| `RUSTSEC-2026-0097` | `rand` | `0.9.2` | `0.9.4` | Lockfile point update from broad `cargo update`; patched release stays within `0.9` ABI constraints. |
| `RUSTSEC-2025-0119` | `number_prefix` | `0.4.0` | removed | Bumped Aspen CLI `indicatif` to `0.18` and removed the unused root `cargo-nextest` crate dev-dependency; `number_prefix` left the lockfile while the external `cargo nextest` tool remains the test runner. |
| `RUSTSEC-2024-0370` | `proc-macro-error` | `1.0.4` | removed | Bumped Forge web `maud` to `0.27`; `maud_macros` no longer depends on `proc-macro-error`. |
| `RUSTSEC-2026-0002` | `lru` | `0.12.5` | upstream-pinned | Advisory-specific exception. SNIX `nar-bridge`/`snix-store` pin `lru = ^0.12.4`; Aspen does not call `IterMut` through its SNIX integration. |
| `RUSTSEC-2023-0056` | `vm-memory` | `0.10.0` | upstream-pinned | Advisory-specific exception through `snix-castore -> fuse-backend-rs`; Aspen's SNIX path uses store/NAR traits, not FUSE `VolatileMemory` helpers. |
| `RUSTSEC-2024-0002` | `vmm-sys-util` | `0.11.2` | upstream-pinned | Advisory-specific exception through `snix-castore -> fuse-backend-rs`; Aspen does not deserialize attacker-controlled `FamStructWrapper` values through this edge. |
| `RUSTSEC-2023-0086` | `lexical-core` | `0.8.5` | upstream-pinned | Advisory-specific exception. This remains a `snix-eval` lockfile edge and is not selected by default locked metadata after the current triage. |

## Remaining warning backlog

Only unmaintained warnings remain after applying the advisory-specific exceptions
above. The SNIX/FUSE exceptions stay removal-tracked here because they must be
revisited when upstream dependency constraints move.

### Removal-tracked SNIX/FUSE exceptions

| Priority | Advisory | Crate | Current path | Triage | Removal trigger |
| --- | --- | --- | --- | --- | --- |
| P1 | `RUSTSEC-2026-0002` | `lru 0.12.5` | `nar-bridge` / `snix-store` -> Aspen SNIX crates | Parent crates pin `lru = ^0.12.4`; direct `cargo update -p lru@0.12.5 --precise 0.16.3` is rejected. The advisory affects `IterMut`; audit Aspen/SNIX cache use for mutable iteration before any exception. | Upstream Snix moves `nar-bridge`/`snix-store` to a patched `lru`, or Aspen vendors/patches the Snix dependency. |
| P1 | `RUSTSEC-2023-0056` | `vm-memory 0.10.0` | `fuse-backend-rs 0.12.0` -> `snix-castore` -> Aspen SNIX crates | SNIX/FUSE path. Risk is trait default methods lacking bounds checks; exposure depends on whether Aspen calls affected FUSE memory helpers in its SNIX bridge/cache path. | Snix/fuse-backend-rs update to a patched `vm-memory`, or Aspen removes the FUSE-backed dependency edge from production SNIX builds. |
| P1 | `RUSTSEC-2024-0002` | `vmm-sys-util 0.11.2` | `fuse-backend-rs 0.12.0` -> `snix-castore` -> Aspen SNIX crates | SNIX/FUSE path. Risk is serde deserialization for `FamStructWrapper`; audit whether Aspen deserializes attacker-controlled FAM structs through this edge. | Snix/fuse-backend-rs update to a patched `vmm-sys-util`, or Aspen removes the FUSE-backed dependency edge from production SNIX builds. |
| P2 | `RUSTSEC-2023-0086` | `lexical-core 0.8.5` | lockfile edge from `snix-eval 0.1.0` | Not present in the default `cargo metadata --locked` resolve after this triage, but remains in `Cargo.lock`. Treat as a SNIX eval lockfile/feature edge until proven dead. | Upstream Snix eval moves to `lexical-core 1.x`, or lockfile pruning proves no Aspen build target can select the old edge. |

### Unmaintained warnings

| Priority | Advisory | Crate | Current path | Triage | Removal trigger |
| --- | --- | --- | --- | --- | --- |
| P2 | `RUSTSEC-2025-0141` | `bincode 1.3.3` | `madsim` transitive test/simulation edge | Aspen-owned `aspen-codec` now uses postcard, so no first-party storage/wire path depends on `bincode`. The remaining lockfile warning is upstream madsim-only. | Upstream `madsim` drops its `bincode 1.x` edge, or Aspen removes/replaces that simulation dependency. |
| P2 | `RUSTSEC-2023-0089` | `atomic-polyfill 1.0.3` | `iroh-blobs` / `iroh-docs` default postcard feature -> `heapless 0.7.17` | Aspen direct postcard dependencies disable default features, but the iroh ecosystem still selects postcard `default`/`heapless-cas`. | iroh-blobs/docs disables postcard heapless-cas or updates to a postcard/heapless stack that drops `atomic-polyfill`. |
| P2 | `RUSTSEC-2024-0384` | `instant 0.1.13` | `nostr 0.44.2` -> forge/CLI/Nostr crates | Forge/Nostr integration edge. | Upgrade Nostr stack to remove `instant`. |
| P3 | `RUSTSEC-2024-0436` | `paste 1.0.15` | DataFusion SQL path and netlink/iroh path | Proc-macro unmaintained warning. No direct runtime input exposure; remove through parent upgrades. | DataFusion/iroh/netlink stack drops `paste`. |
| P3 | `RUSTSEC-2024-0370` | `proc-macro-error 0.4.12` | `genawaiter-proc-macro` -> `genawaiter` -> `bao-tree`/`iroh-blobs` | Proc-macro build-time warning through blob DAG stack. | iroh-blobs/bao-tree stack drops `genawaiter` or moves to maintained macro deps. |
| P3 | `RUSTSEC-2023-0081` | `safemem 0.2.0` | `base64 0.7.0` -> `cloud-hypervisor-client 0.3.3` -> Aspen root | Cloud Hypervisor client edge; likely isolated to VM job control surfaces. | Upgrade/replace `cloud-hypervisor-client` or avoid that old base64 path. |

## Next remediation order

1. Parent-stack refreshes: Nostr, DataFusion/iroh/netlink, iroh-blobs/bao-tree, and Cloud Hypervisor client.
2. Upstream-watch the remaining `madsim` bincode edge, iroh postcard/heapless edge, and SNIX/FUSE exceptions; remove ignores as soon as parent stacks move or Aspen can make the affected edge unreachable in the selected feature graph.
