# Extraction Manifest Stub: Foundational Types and Helpers

## Candidate

- **Family**: Foundational types/helpers
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-constants`, `aspen-hlc`, `aspen-kv-types`, `aspen-storage-types`, `aspen-cluster-types`, `aspen-traits`, `aspen-time`
- **Intended audience**: Rust projects that need Aspen's bounded constants, portable clocks/types, KV/storage type contracts, cluster address types, or shared traits without Aspen runtime shells.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable leaf/helper candidates

## Package and release metadata

- **Documentation entrypoint**: crate-level Rustdoc per crate plus family-level examples once selected.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet.
- **Publish readiness**: blocked; no crate in this family may be marked publishable during `prepare-crate-extraction`.

## Feature contract

Foundational defaults should be alloc/no-std friendly where already designed that way. Runtime helpers must be opt-in.

| Crate | Default-feature contract | Current status | Next action |
| --- | --- | --- | --- |
| `aspen-constants` | Leaf constants only. | `workspace-internal` | Document semver policy and downstream examples. |
| `aspen-hlc` | Portable HLC types; `std` only when explicitly needed. | `workspace-internal` | Keep `uhlc` rand/getrandom optional and prove no unexpected std graph. |
| `aspen-kv-types` | Reusable KV types. | Covered by Redb Raft KV manifest. | Keep direct manifest as first-slice layer. |
| `aspen-storage-types` | Reusable storage data types without Redb table definitions. | `workspace-internal` | Split `SM_KV_TABLE` / `redb::TableDefinition` into shell/storage crate. |
| `aspen-cluster-types` | Alloc-safe defaults; iroh helpers behind explicit feature. | `workspace-internal` | Keep `default = []` and prove representative consumers do not re-enable defaults accidentally. |
| `aspen-traits` | Trait contracts without std/runtime leaks. | `workspace-internal` | Replace std-only assumptions or move blanket impls to shell crates; verify transitive default-feature leak absence. |
| `aspen-time` | Explicit time boundary helpers. | `workspace-internal` | Document wall-clock boundary ownership and examples. |

## Dependency decisions

- `aspen-storage-types` Redb table definitions are not portable type surface and must move before readiness.
- `aspen-traits` must be checked through representative consumers because prior no-std work showed transitive default-feature leaks can re-enable runtime dependencies.
- `aspen-cluster-types` may expose iroh conversions only behind explicit feature gates.

## Compatibility and aliases

No compatibility re-exports are planned by this stub. If a future task moves storage table definitions or trait blanket impls, it must add old path, new path, owner, tests, and removal criteria.

## Verification rails

- compile selected crates with `--no-default-features` where supported;
- dependency-boundary checker with direct and representative-consumer paths;
- positive examples for portable imports;
- negative checks proving runtime-only helpers are unavailable without opt-in features.

## First blocker

The highest-ROI blocker is `aspen-storage-types`: `KvEntry` is alloc-safe, but `SM_KV_TABLE` keeps `redb`/`libc` in the foundational graph. The next blocker is `aspen-traits` transitive default-feature leak proof.
