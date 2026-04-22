Evidence-ID: extend-no-std-foundation-and-wire.r1-storage
Task-ID: R1
Artifact-Type: baseline-verification
Covers: core.no-std-core-baseline.compile-fail-verification-is-reviewable, core.no-std-core-baseline.shell-alias-path-proof-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.acyclic-boundary-proof-is-reviewable

# Baseline storage seam verification

- Baseline source commit: `f1c02f9f5c34f3d3f6218a26668b36157618c9ce`
- Final rail logic source commit: `4f0c9801e`
- Baseline source captured in a standalone git clone at `/tmp/aspen-no-std-baseline-root.ETpnVI/aspen`
- Final rail logic scripts copied from current HEAD into the baseline snapshot before second-pass checks.

## wasm32 target setup

Host setup record:

```text
- rustup unavailable in this task environment; target availability proved by saved `cargo check --target wasm32-unknown-unknown` command results below.
```

## `cargo check -p aspen-storage-types`

```text
$ cargo check -p aspen-storage-types
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling libc v0.2.183
   Compiling serde v1.0.228
   Compiling redb v2.6.3
   Compiling syn v2.0.117
   Compiling serde_derive v1.0.228
    Checking bincode v1.3.3
    Checking aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.96s

[exit status: 0]
```

## `cargo check -p aspen-storage-types --no-default-features`

```text
$ cargo check -p aspen-storage-types --no-default-features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s

[exit status: 0]
```

## `cargo check -p aspen-storage-types --no-default-features --target wasm32-unknown-unknown`

```text
$ cargo check -p aspen-storage-types --no-default-features --target wasm32-unknown-unknown
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling serde v1.0.228
   Compiling redb v2.6.3
   Compiling syn v2.0.117
   Compiling serde_derive v1.0.228
    Checking bincode v1.3.3
    Checking aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.47s

[exit status: 0]
```

## `cargo tree -p aspen-storage-types -e normal`

```text
$ cargo tree -p aspen-storage-types -e normal
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
├── bincode v1.3.3
│   └── serde v1.0.228
│       ├── serde_core v1.0.228
│       └── serde_derive v1.0.228 (proc-macro)
│           ├── proc-macro2 v1.0.106
│           │   └── unicode-ident v1.0.24
│           ├── quote v1.0.45
│           │   └── proc-macro2 v1.0.106 (*)
│           └── syn v2.0.117
│               ├── proc-macro2 v1.0.106 (*)
│               ├── quote v1.0.45 (*)
│               └── unicode-ident v1.0.24
├── redb v2.6.3
│   └── libc v0.2.183
└── serde v1.0.228 (*)

[exit status: 0]
```

## `cargo tree -p aspen-storage-types -e features`

```text
$ cargo tree -p aspen-storage-types -e features
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
warning: resolver for the non root package will be ignored, specify resolver at the workspace root:
package:   /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/vendor/iroh-h3-axum/Cargo.toml
workspace: /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/Cargo.toml
aspen-storage-types v0.1.0 (/tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-storage-types)
├── bincode v1.3.3
│   └── serde feature "default"
│       ├── serde v1.0.228
│       │   ├── serde_core feature "result"
│       │   │   └── serde_core v1.0.228
│       │   └── serde_derive feature "default"
│       │       └── serde_derive v1.0.228 (proc-macro)
│       │           ├── proc-macro2 feature "proc-macro"
│       │           │   └── proc-macro2 v1.0.106
│       │           │       └── unicode-ident feature "default"
│       │           │           └── unicode-ident v1.0.24
│       │           ├── quote feature "proc-macro"
│       │           │   ├── quote v1.0.45
│       │           │   │   └── proc-macro2 v1.0.106 (*)
│       │           │   └── proc-macro2 feature "proc-macro" (*)
│       │           ├── syn feature "clone-impls"
│       │           │   └── syn v2.0.117
│       │           │       ├── proc-macro2 v1.0.106 (*)
│       │           │       ├── quote v1.0.45 (*)
│       │           │       └── unicode-ident feature "default" (*)
│       │           ├── syn feature "derive"
│       │           │   └── syn v2.0.117 (*)
│       │           ├── syn feature "parsing"
│       │           │   └── syn v2.0.117 (*)
│       │           ├── syn feature "printing"
│       │           │   └── syn v2.0.117 (*)
│       │           └── syn feature "proc-macro"
│       │               ├── syn v2.0.117 (*)
│       │               ├── proc-macro2 feature "proc-macro" (*)
│       │               └── quote feature "proc-macro" (*)
│       └── serde feature "std"
│           ├── serde v1.0.228 (*)
│           └── serde_core feature "std"
│               └── serde_core v1.0.228
├── serde feature "alloc"
│   ├── serde v1.0.228 (*)
│   └── serde_core feature "alloc"
│       └── serde_core v1.0.228
├── serde feature "derive"
│   ├── serde v1.0.228 (*)
│   └── serde feature "serde_derive"
│       └── serde v1.0.228 (*)
└── redb feature "default"
    └── redb v2.6.3
        └── libc feature "default"
            ├── libc v0.2.183
            └── libc feature "std"
                └── libc v0.2.183
[dev-dependencies]
├── postcard feature "alloc"
│   ├── postcard v1.1.3
│   │   ├── cobs v0.3.0
│   │   │   └── thiserror v2.0.18
│   │   │       └── thiserror-impl feature "default"
│   │   │           └── thiserror-impl v2.0.18 (proc-macro)
│   │   │               ├── proc-macro2 feature "default"
│   │   │               │   ├── proc-macro2 v1.0.106 (*)
│   │   │               │   └── proc-macro2 feature "proc-macro" (*)
│   │   │               ├── quote feature "default"
│   │   │               │   ├── quote v1.0.45 (*)
│   │   │               │   └── quote feature "proc-macro" (*)
│   │   │               └── syn feature "default"
│   │   │                   ├── syn v2.0.117 (*)
│   │   │                   ├── syn feature "clone-impls" (*)
│   │   │                   ├── syn feature "derive" (*)
│   │   │                   ├── syn feature "parsing" (*)
│   │   │                   ├── syn feature "printing" (*)
│   │   │                   └── syn feature "proc-macro" (*)
│   │   ├── serde feature "derive" (*)
│   │   └── heapless feature "serde"
│   │       └── heapless v0.7.17
│   │           ├── serde v1.0.228 (*)
│   │           ├── stable_deref_trait v1.2.1
│   │           ├── hash32 feature "default"
│   │           │   └── hash32 v0.2.1
│   │           │       └── byteorder v1.5.0
│   │           └── spin feature "default"
│   │               ├── spin v0.9.8
│   │               │   └── lock_api feature "default"
│   │               │       ├── lock_api v0.4.14
│   │               │       │   └── scopeguard v1.2.0
│   │               │       └── lock_api feature "atomic_usize"
│   │               │           └── lock_api v0.4.14 (*)
│   │               ├── spin feature "barrier"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "mutex"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "lazy"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "once"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "lock_api"
│   │               │   ├── spin v0.9.8 (*)
│   │               │   └── spin feature "lock_api_crate"
│   │               │       └── spin v0.9.8 (*)
│   │               ├── spin feature "mutex" (*)
│   │               ├── spin feature "once" (*)
│   │               ├── spin feature "rwlock"
│   │               │   └── spin v0.9.8 (*)
│   │               └── spin feature "spin_mutex"
│   │                   ├── spin v0.9.8 (*)
│   │                   └── spin feature "mutex" (*)
│   │           [build-dependencies]
│   │           └── rustc_version feature "default"
│   │               └── rustc_version v0.4.1
│   │                   └── semver feature "default"
│   │                       ├── semver v1.0.27
│   │                       └── semver feature "std"
│   │                           └── semver v1.0.27
│   └── serde feature "alloc" (*)
└── postcard feature "default"
    ├── postcard v1.1.3 (*)
    └── postcard feature "heapless-cas"
        ├── postcard v1.1.3 (*)
        ├── postcard feature "heapless"
        │   └── postcard v1.1.3 (*)
        └── heapless feature "cas"
            ├── heapless v0.7.17 (*)
            └── heapless feature "atomic-polyfill"
                └── heapless v0.7.17 (*)

[exit status: 0]
```

## `python3 scripts/check-aspen-core-no-std-surface.py --crate-dir /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/src --output-dir /tmp/aspen-no-std-baseline-run.MvIgm5/surface`

```text
$ python3 scripts/check-aspen-core-no-std-surface.py --crate-dir /tmp/aspen-no-std-baseline-root.ETpnVI/aspen/crates/aspen-core/src --output-dir /tmp/aspen-no-std-baseline-run.MvIgm5/surface
error: core storage.rs must not mention `SM_KV_TABLE`
error: missing shell module `storage` guarded by `always`

[exit status: 1]
```

## Surface inventory

```text
missing file: /tmp/aspen-no-std-baseline-run.MvIgm5/surface/surface-inventory.md
```

## Export map

```text
missing file: /tmp/aspen-no-std-baseline-run.MvIgm5/surface/export-map.md
```

## Source audit

```text
missing file: /tmp/aspen-no-std-baseline-run.MvIgm5/surface/source-audit.txt
```
